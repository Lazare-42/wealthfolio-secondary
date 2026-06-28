//! Reconciliation service implementation.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::activities::{parse_csv, Activity, ActivityServiceTrait, ActivityStatus, NewActivity};
use crate::activities::{
    ImportRun, ImportRunMode, ImportRunRepositoryTrait, ImportRunSummary, ImportRunType, ReviewMode,
};
use crate::events::DomainEventSink;
use crate::settings::SettingsServiceTrait;
use crate::Result;

use super::{
    MatchStatus, ReconciliationConfig, ReconciliationItem, ReconciliationResult,
    ReconciliationServiceTrait, ResolveRequest, ScanResult, StatementAccountMapping,
};

const SETTINGS_KEY: &str = "reconciliation_config";
const SOURCE_SYSTEM: &str = "STATEMENT";
const DEFAULT_TOLERANCE: Decimal = dec!(0.01);

pub struct ReconciliationService {
    activity_service: Arc<dyn ActivityServiceTrait + Send + Sync>,
    import_run_repo: Arc<dyn ImportRunRepositoryTrait>,
    settings_service: Arc<dyn SettingsServiceTrait>,
    event_sink: Arc<dyn DomainEventSink>,
    /// Overrides WF_STATEMENTS_DIR. If None, reads from config.
    statements_dir_override: Option<String>,
}

impl ReconciliationService {
    pub fn new(
        activity_service: Arc<dyn ActivityServiceTrait + Send + Sync>,
        import_run_repo: Arc<dyn ImportRunRepositoryTrait>,
        settings_service: Arc<dyn SettingsServiceTrait>,
        event_sink: Arc<dyn DomainEventSink>,
        statements_dir_override: Option<String>,
    ) -> Self {
        Self {
            activity_service,
            import_run_repo,
            settings_service,
            event_sink,
            statements_dir_override,
        }
    }

    /// Get the effective config, merging env override for statements_dir.
    fn effective_config(&self) -> Result<ReconciliationConfig> {
        let mut config = self.get_config()?;
        if let Some(ref dir) = self.statements_dir_override {
            config.statements_dir = dir.clone();
        }
        Ok(config)
    }

    /// Compute SHA-256 hash of file content.
    fn file_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        hex::encode(hasher.finalize())
    }

    /// Check if a file hash was already processed.
    fn is_file_already_processed(&self, file_hash: &str) -> Result<bool> {
        let processed = self.get_processed_hashes()?;
        Ok(processed.contains(&file_hash.to_string()))
    }

    fn get_processed_hashes(&self) -> Result<Vec<String>> {
        match self
            .settings_service
            .get_setting_value("reconciliation_processed_hashes")?
        {
            Some(val) => serde_json::from_str(&val).map_err(|e| {
                crate::errors::Error::Unexpected(format!("Failed to parse processed hashes: {}", e))
            }),
            None => Ok(Vec::new()),
        }
    }

    async fn add_processed_hash(&self, hash: &str) -> Result<()> {
        let mut hashes = self.get_processed_hashes()?;
        if !hashes.contains(&hash.to_string()) {
            hashes.push(hash.to_string());
        }
        let json = serde_json::to_string(&hashes).map_err(|e| {
            crate::errors::Error::Unexpected(format!("Failed to serialize processed hashes: {}", e))
        })?;
        self.settings_service
            .set_setting_value("reconciliation_processed_hashes", &json)
            .await
    }

    /// Find which mapping matches a filename.
    fn find_mapping<'a>(
        filename: &str,
        mappings: &'a [StatementAccountMapping],
    ) -> Option<&'a StatementAccountMapping> {
        for mapping in mappings {
            if glob_match(&mapping.file_pattern, filename) {
                return Some(mapping);
            }
        }
        None
    }

    /// Parse a bank statement CSV and extract rows.
    fn parse_statement(
        content: &[u8],
        mapping: &StatementAccountMapping,
    ) -> Result<Vec<ParsedBankRow>> {
        let parse_config = mapping.parse_config.clone().unwrap_or_default();
        let parsed = parse_csv(content, &parse_config)?;

        let headers = &parsed.headers;
        let date_idx = find_column_index(headers, &mapping.field_mappings.date_column);
        let amount_idx = find_column_index(headers, &mapping.field_mappings.amount_column);
        let desc_idx = mapping
            .field_mappings
            .description_column
            .as_ref()
            .and_then(|col| find_column_index(headers, col));
        let type_idx = mapping
            .field_mappings
            .type_column
            .as_ref()
            .and_then(|col| find_column_index(headers, col));

        let date_idx = date_idx.ok_or_else(|| {
            crate::errors::Error::Validation(crate::errors::ValidationError::InvalidInput(format!(
                "Date column '{}' not found in CSV headers: {:?}",
                mapping.field_mappings.date_column, headers
            )))
        })?;
        let amount_idx = amount_idx.ok_or_else(|| {
            crate::errors::Error::Validation(crate::errors::ValidationError::InvalidInput(format!(
                "Amount column '{}' not found in CSV headers: {:?}",
                mapping.field_mappings.amount_column, headers
            )))
        })?;

        let default_currency = mapping
            .field_mappings
            .default_currency
            .clone()
            .unwrap_or_else(|| "USD".to_string());

        let mut rows = Vec::new();
        for (i, row) in parsed.rows.iter().enumerate() {
            let date_str = row.get(date_idx).map(|s| s.trim()).unwrap_or("");
            let amount_str = row.get(amount_idx).map(|s| s.trim()).unwrap_or("0");
            let description = desc_idx.and_then(|idx| row.get(idx).map(|s| s.trim().to_string()));
            let raw_type = type_idx.and_then(|idx| row.get(idx).map(|s| s.trim().to_string()));

            let date = parse_date(date_str);
            let amount = parse_amount(amount_str);

            if let (Some(date), Some(amount)) = (date, amount) {
                rows.push(ParsedBankRow {
                    row_index: i as u32,
                    date,
                    amount,
                    currency: default_currency.clone(),
                    description,
                    raw_type,
                });
            } else {
                log::warn!(
                    "Skipping row {}: could not parse date='{}' or amount='{}'",
                    i,
                    date_str,
                    amount_str
                );
            }
        }
        Ok(rows)
    }

    /// Core matching algorithm: match bank rows against existing activities.
    pub(crate) fn reconcile(
        bank_rows: &[ParsedBankRow],
        existing: &[Activity],
        tolerance: Decimal,
    ) -> Vec<ReconciliationItem> {
        // Build lookup: date → list of existing activities (as mutable pool)
        let mut pool: HashMap<NaiveDate, Vec<&Activity>> = HashMap::new();
        for act in existing {
            let date = act.activity_date.date_naive();
            pool.entry(date).or_default().push(act);
        }

        let mut items = Vec::new();

        for row in bank_rows {
            let bank_amount = row.amount.abs();
            let candidates = pool.get_mut(&row.date);

            let (status, matched, confidence) = match candidates {
                Some(acts) if !acts.is_empty() => {
                    // Find best match by amount
                    let mut best_idx = None;
                    let mut best_diff = Decimal::MAX;
                    for (i, act) in acts.iter().enumerate() {
                        let act_amount = act.amount.unwrap_or(Decimal::ZERO).abs();
                        let diff = (bank_amount - act_amount).abs();
                        if diff < best_diff {
                            best_diff = diff;
                            best_idx = Some(i);
                        }
                    }
                    if let Some(idx) = best_idx {
                        if best_diff <= tolerance {
                            // Exact match — remove from pool
                            let matched_act = acts.remove(idx);
                            (MatchStatus::Matched, Some(matched_act.clone()), 1.0)
                        } else {
                            // Date match but amount mismatch
                            let conflict_act = acts[idx].clone();
                            (MatchStatus::Conflict, Some(conflict_act), 0.5)
                        }
                    } else {
                        (MatchStatus::Unmatched, None, 0.0)
                    }
                }
                _ => (MatchStatus::Unmatched, None, 0.0),
            };

            items.push(ReconciliationItem {
                row_index: row.row_index,
                activity_date: row.date,
                description: row.description.clone(),
                amount: row.amount,
                currency: row.currency.clone(),
                raw_type: row.raw_type.clone(),
                match_status: status,
                matched_activity: matched,
                draft_activity_id: None,
                confidence,
                mapped_activity_type: Some(infer_activity_type(
                    row.amount,
                    row.raw_type.as_deref(),
                )),
            });
        }

        // Remaining unmatched existing activities → Missing
        for (_, acts) in &pool {
            for act in acts {
                items.push(ReconciliationItem {
                    row_index: u32::MAX,
                    activity_date: act.activity_date.date_naive(),
                    description: act.notes.clone(),
                    amount: act.amount.unwrap_or(Decimal::ZERO),
                    currency: act.currency.clone(),
                    raw_type: Some(act.activity_type.clone()),
                    match_status: MatchStatus::Missing,
                    matched_activity: Some((*act).clone()),
                    draft_activity_id: None,
                    confidence: 0.0,
                    mapped_activity_type: None,
                });
            }
        }

        items
    }

    /// Reconstruct a ReconciliationResult from an existing ImportRun.
    fn reconstruct_reconciliation(&self, run: &ImportRun) -> Result<ReconciliationResult> {
        // Try to restore full items from checkpoint_out (new format).
        // Fall back to draft-only reconstruction for legacy runs.
        let items: Vec<ReconciliationItem> = if let Some(stored) = run
            .checkpoint_out
            .as_ref()
            .and_then(|c| c.get("items"))
            .filter(|v| v.is_array() && !v.as_array().unwrap().is_empty())
        {
            serde_json::from_value(stored.clone()).unwrap_or_default()
        } else {
            // Legacy fallback: reconstruct from draft activity IDs only
            let draft_ids: Vec<String> = run
                .checkpoint_out
                .as_ref()
                .and_then(|c| c.get("draft_activity_ids"))
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            let mut run_activities: Vec<Activity> = Vec::new();
            for id in &draft_ids {
                match self.activity_service.get_activity(id) {
                    Ok(act) => run_activities.push(act),
                    Err(_) => {} // Activity may have been deleted (rejected)
                }
            }

            let mut legacy_items: Vec<ReconciliationItem> = Vec::new();
            for act in &run_activities {
                let match_status = act
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("match_status"))
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or(MatchStatus::Unmatched);

                legacy_items.push(ReconciliationItem {
                    row_index: act
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("row_index"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                    activity_date: act.activity_date.date_naive(),
                    description: act.notes.clone(),
                    amount: act.amount.unwrap_or(Decimal::ZERO),
                    currency: act.currency.clone(),
                    raw_type: act.source_type.clone(),
                    match_status,
                    matched_activity: None,
                    draft_activity_id: Some(act.id.clone()),
                    confidence: 0.0,
                    mapped_activity_type: Some(act.activity_type.clone()),
                });
            }
            legacy_items
        };

        let file_path = run
            .checkpoint_out
            .as_ref()
            .and_then(|c| c.get("file_path"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let file_name = Path::new(&file_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        Ok(ReconciliationResult {
            import_run: run.clone(),
            file_path,
            file_name,
            account_id: run.account_id.clone(),
            items,
            summary: run.summary.clone().unwrap_or_default(),
        })
    }
}

#[async_trait]
impl ReconciliationServiceTrait for ReconciliationService {
    async fn scan_directory(&self) -> Result<ScanResult> {
        let config = self.effective_config()?;

        if config.statements_dir.is_empty() {
            return Err(crate::errors::Error::Unexpected(
                "Statements directory not configured".to_string(),
            ));
        }

        let dir = Path::new(&config.statements_dir);
        if !dir.exists() || !dir.is_dir() {
            return Err(crate::errors::Error::Unexpected(format!(
                "Statements directory does not exist: {}",
                config.statements_dir
            )));
        }

        let tolerance = config.amount_tolerance.unwrap_or(DEFAULT_TOLERANCE);
        let mut result = ScanResult {
            files_found: 0,
            files_new: 0,
            files_skipped: 0,
            reconciliations: Vec::new(),
        };

        let entries: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| {
                crate::errors::Error::Unexpected(format!("Failed to read directory: {}", e))
            })?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "csv")
                    .unwrap_or(false)
            })
            .collect();

        result.files_found = entries.len() as u32;

        for entry in entries {
            let path = entry.path();
            let filename = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Find matching account mapping
            let mapping = match Self::find_mapping(&filename, &config.mappings) {
                Some(m) => m,
                None => {
                    log::debug!("No mapping for file: {}", filename);
                    result.files_skipped += 1;
                    continue;
                }
            };

            // Read file
            let content = std::fs::read(&path).map_err(|e| {
                crate::errors::Error::Unexpected(format!("Failed to read file {}: {}", filename, e))
            })?;

            // Check if already processed
            let hash = Self::file_hash(&content);
            if self.is_file_already_processed(&hash)? {
                log::debug!(
                    "File already processed (hash: {}): {}",
                    &hash[..8],
                    filename
                );
                result.files_skipped += 1;
                continue;
            }

            // Parse statement
            let bank_rows = match Self::parse_statement(&content, mapping) {
                Ok(rows) => rows,
                Err(e) => {
                    log::warn!("Failed to parse {}: {}", filename, e);
                    result.files_skipped += 1;
                    continue;
                }
            };

            if bank_rows.is_empty() {
                log::debug!("No parseable rows in: {}", filename);
                result.files_skipped += 1;
                continue;
            }

            // Determine date range
            let min_date = bank_rows.iter().map(|r| r.date).min().unwrap();
            let max_date = bank_rows.iter().map(|r| r.date).max().unwrap();

            // Fetch existing activities for this account, filter to date range
            let all_activities = self
                .activity_service
                .get_activities_by_account_id(&mapping.account_id)?;
            let existing: Vec<Activity> = all_activities
                .into_iter()
                .filter(|a| {
                    let d = a.activity_date.date_naive();
                    d >= min_date && d <= max_date && a.status == ActivityStatus::Posted
                })
                .collect();

            // Run matching
            let mut items = Self::reconcile(&bank_rows, &existing, tolerance);

            // Create ImportRun
            let mut import_run = ImportRun::new(
                mapping.account_id.clone(),
                SOURCE_SYSTEM.to_string(),
                ImportRunType::Import,
                ImportRunMode::Repair,
                ReviewMode::Always,
            );

            // Create DRAFT activities for unmatched rows
            let mut inserted = 0u32;
            let mut conflicts = 0u32;
            let mut matched = 0u32;
            let mut missing = 0u32;
            let mut draft_activity_ids: Vec<String> = Vec::new();

            for item in &mut items {
                match item.match_status {
                    MatchStatus::Unmatched => {
                        let activity_type = item
                            .mapped_activity_type
                            .clone()
                            .unwrap_or_else(|| "DEPOSIT".to_string());

                        let new_activity = NewActivity {
                            id: None,
                            account_id: mapping.account_id.clone(),
                            asset: None,
                            activity_type,
                            subtype: None,
                            activity_date: item.activity_date.to_string(),
                            quantity: None,
                            unit_price: None,
                            currency: item.currency.clone(),
                            fee: None,
                            amount: Some(item.amount),
                            status: Some(ActivityStatus::Draft),
                            notes: item.description.clone(),
                            fx_rate: None,
                            metadata: Some(
                                json!({
                                    "match_status": "UNMATCHED",
                                    "row_index": item.row_index,
                                    "bank_description": item.description,
                                    "bank_raw_type": item.raw_type,
                                })
                                .to_string(),
                            ),
                            needs_review: Some(true),
                            source_system: Some(SOURCE_SYSTEM.to_string()),
                            source_record_id: None,
                            source_group_id: None,
                            idempotency_key: None,
                            import_run_id: None,
                        };

                        match self.activity_service.create_activity(new_activity).await {
                            Ok(created) => {
                                draft_activity_ids.push(created.id.clone());
                                item.draft_activity_id = Some(created.id);
                                inserted += 1;
                            }
                            Err(e) => {
                                log::warn!(
                                    "Failed to create draft activity for row {}: {}",
                                    item.row_index,
                                    e
                                );
                            }
                        }
                    }
                    MatchStatus::Matched => matched += 1,
                    MatchStatus::Conflict => conflicts += 1,
                    MatchStatus::Missing => missing += 1,
                }
            }

            // Store file metadata, draft activity IDs, and full items in checkpoint_out
            import_run.checkpoint_out = Some(json!({
                "file_hash": hash,
                "file_path": path.to_string_lossy(),
                "file_name": filename,
                "draft_activity_ids": draft_activity_ids,
                "items": items,
            }));

            // Update import run summary
            import_run.summary = Some(ImportRunSummary {
                fetched: bank_rows.len() as u32,
                inserted,
                updated: conflicts,
                skipped: matched,
                warnings: conflicts,
                errors: 0,
                removed: missing,
                assets_created: 0,
            });

            import_run.mark_needs_review();
            let import_run = self.import_run_repo.create(import_run).await?;

            // Record file hash as processed
            self.add_processed_hash(&hash).await?;

            result.reconciliations.push(ReconciliationResult {
                import_run,
                file_path: path.to_string_lossy().to_string(),
                file_name: filename,
                account_id: mapping.account_id.clone(),
                items,
                summary: ImportRunSummary {
                    fetched: bank_rows.len() as u32,
                    inserted,
                    updated: conflicts,
                    skipped: matched,
                    warnings: conflicts,
                    errors: 0,
                    removed: missing,
                    assets_created: 0,
                },
            });

            result.files_new += 1;
        }

        Ok(result)
    }

    async fn get_pending_reconciliations(&self) -> Result<Vec<ReconciliationResult>> {
        let config = self.effective_config()?;
        let mut results = Vec::new();

        let mut seen_accounts: Vec<String> = Vec::new();
        for mapping in &config.mappings {
            if seen_accounts.contains(&mapping.account_id) {
                continue;
            }
            seen_accounts.push(mapping.account_id.clone());

            let runs = self
                .import_run_repo
                .get_recent_for_account(&mapping.account_id, 50)?;

            for run in runs {
                if run.source_system == SOURCE_SYSTEM
                    && run.status == crate::activities::ImportRunStatus::NeedsReview
                {
                    match self.reconstruct_reconciliation(&run) {
                        Ok(recon) => results.push(recon),
                        Err(e) => {
                            log::warn!("Failed to reconstruct reconciliation {}: {}", run.id, e)
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    fn get_reconciliation(&self, import_run_id: &str) -> Result<ReconciliationResult> {
        let run = self
            .import_run_repo
            .get_by_id(import_run_id)?
            .ok_or_else(|| {
                crate::errors::Error::Unexpected(format!("Import run not found: {}", import_run_id))
            })?;

        if run.source_system != SOURCE_SYSTEM {
            return Err(crate::errors::Error::Unexpected(format!(
                "Import run {} is not a statement reconciliation",
                import_run_id
            )));
        }

        self.reconstruct_reconciliation(&run)
    }

    async fn resolve(&self, request: ResolveRequest) -> Result<ImportRun> {
        let mut run = self
            .import_run_repo
            .get_by_id(&request.import_run_id)?
            .ok_or_else(|| {
                crate::errors::Error::Unexpected(format!(
                    "Import run not found: {}",
                    request.import_run_id
                ))
            })?;

        let mut account_ids = Vec::new();

        // Accept: promote Draft → Posted via ActivityUpdate
        for id in &request.accept_ids {
            let activity = self.activity_service.get_activity(id)?;
            if activity.status != ActivityStatus::Draft {
                log::warn!("Activity {} is not a draft, skipping accept", id);
                continue;
            }
            let update = crate::activities::ActivityUpdate {
                id: activity.id.clone(),
                account_id: activity.account_id.clone(),
                asset: None,
                activity_type: activity.activity_type.clone(),
                subtype: activity.subtype.clone(),
                activity_date: activity.activity_date.format("%Y-%m-%d").to_string(),
                quantity: None,
                unit_price: None,
                currency: activity.currency.clone(),
                fee: None,
                amount: Some(activity.amount),
                status: Some(ActivityStatus::Posted),
                notes: activity.notes.clone(),
                fx_rate: None,
                metadata: activity.metadata.as_ref().map(|v| v.to_string()),
            };
            match self.activity_service.update_activity(update).await {
                Ok(act) => {
                    if !account_ids.contains(&act.account_id) {
                        account_ids.push(act.account_id);
                    }
                }
                Err(e) => log::warn!("Failed to accept activity {}: {}", id, e),
            }
        }

        // Reject: delete draft activities
        for id in &request.reject_ids {
            match self.activity_service.delete_activity(id.clone()).await {
                Ok(act) => {
                    if !account_ids.contains(&act.account_id) {
                        account_ids.push(act.account_id);
                    }
                }
                Err(e) => log::warn!("Failed to delete rejected activity {}: {}", id, e),
            }
        }

        // Mark run as applied
        run.complete();
        let run = self.import_run_repo.update(run).await?;

        // Emit domain event if activities were changed
        if !account_ids.is_empty() {
            self.event_sink
                .emit(crate::events::DomainEvent::ActivitiesChanged {
                    account_ids,
                    asset_ids: Vec::new(),
                    currencies: Vec::new(),
                    earliest_activity_at_utc: None,
                });
        }

        Ok(run)
    }

    fn get_config(&self) -> Result<ReconciliationConfig> {
        match self.settings_service.get_setting_value(SETTINGS_KEY)? {
            Some(val) => serde_json::from_str(&val).map_err(|e| {
                crate::errors::Error::Unexpected(format!(
                    "Failed to parse reconciliation config: {}",
                    e
                ))
            }),
            None => Ok(ReconciliationConfig::default()),
        }
    }

    async fn save_config(&self, config: ReconciliationConfig) -> Result<()> {
        let json = serde_json::to_string(&config).map_err(|e| {
            crate::errors::Error::Unexpected(format!(
                "Failed to serialize reconciliation config: {}",
                e
            ))
        })?;
        self.settings_service
            .set_setting_value(SETTINGS_KEY, &json)
            .await
    }
}

// ── Helpers ──

pub(crate) struct ParsedBankRow {
    pub(crate) row_index: u32,
    pub(crate) date: NaiveDate,
    pub(crate) amount: Decimal,
    pub(crate) currency: String,
    pub(crate) description: Option<String>,
    pub(crate) raw_type: Option<String>,
}

fn find_column_index(headers: &[String], column_name: &str) -> Option<usize> {
    let lower = column_name.to_lowercase();
    headers
        .iter()
        .position(|h| h.to_lowercase().trim() == lower)
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    let formats = [
        "%Y-%m-%d", "%d/%m/%Y", "%m/%d/%Y", "%d-%m-%Y", "%Y/%m/%d", "%d.%m.%Y",
    ];
    for fmt in &formats {
        if let Ok(d) = NaiveDate::parse_from_str(s, fmt) {
            return Some(d);
        }
    }
    None
}

fn parse_amount(s: &str) -> Option<Decimal> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+')
        .collect();
    cleaned.parse::<Decimal>().ok()
}

/// Infer activity type from amount sign and description keywords.
fn infer_activity_type(amount: Decimal, raw_type: Option<&str>) -> String {
    if let Some(rt) = raw_type {
        let lower = rt.to_lowercase();
        if lower.contains("dividend") {
            return "DIVIDEND".to_string();
        }
        if lower.contains("interest") {
            return "INTEREST".to_string();
        }
        if lower.contains("fee") || lower.contains("charge") {
            return "FEE".to_string();
        }
        if lower.contains("tax") {
            return "TAX".to_string();
        }
    }
    if amount >= Decimal::ZERO {
        "DEPOSIT".to_string()
    } else {
        "WITHDRAWAL".to_string()
    }
}

/// Simple glob matching supporting only `*` wildcards.
fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern_lower = pattern.to_lowercase();
    let text_lower = text.to_lowercase();

    let parts: Vec<&str> = pattern_lower.split('*').collect();
    if parts.len() == 1 {
        return pattern_lower == text_lower;
    }

    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        match text_lower[pos..].find(part) {
            Some(idx) => {
                if i == 0 && idx != 0 {
                    return false;
                }
                pos += idx + part.len();
            }
            None => return false,
        }
    }

    if !pattern_lower.ends_with('*') {
        return pos == text_lower.len();
    }

    true
}
