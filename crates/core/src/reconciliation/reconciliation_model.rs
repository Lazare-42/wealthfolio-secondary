//! Reconciliation domain models.
//!
//! These are pure API/logic types — no database structs.
//! Reconciliation reuses existing `ImportRun` and `Activity` tables.

use crate::activities::{Activity, ParseConfig};
use crate::activities::{ImportRun, ImportRunSummary};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Match status for a bank statement row against existing activities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MatchStatus {
    /// Bank row matches an existing activity (date + amount within tolerance)
    Matched,
    /// Bank row has no matching activity — new transaction
    Unmatched,
    /// Bank row matches date but not amount — needs review
    Conflict,
    /// Existing activity not found in statement
    Missing,
}

/// User action when resolving a reconciliation proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserAction {
    Accept,
    Reject,
}

/// A single reconciliation item (bank row matched against existing data).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationItem {
    pub row_index: u32,
    pub activity_date: NaiveDate,
    pub description: Option<String>,
    pub amount: Decimal,
    pub currency: String,
    pub raw_type: Option<String>,
    pub match_status: MatchStatus,
    /// The existing activity that was matched/conflicted, if any.
    pub matched_activity: Option<Activity>,
    /// The DRAFT activity created for unmatched rows, if any.
    pub draft_activity_id: Option<String>,
    pub confidence: f64,
    pub mapped_activity_type: Option<String>,
}

/// Result of reconciling a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationResult {
    pub import_run: ImportRun,
    pub file_path: String,
    pub file_name: String,
    pub account_id: String,
    pub items: Vec<ReconciliationItem>,
    pub summary: ImportRunSummary,
}

/// Result of scanning the statements directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub files_found: u32,
    pub files_new: u32,
    pub files_skipped: u32,
    pub reconciliations: Vec<ReconciliationResult>,
}

/// Maps a filename pattern to an account + parsing config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatementAccountMapping {
    pub file_pattern: String,
    pub account_id: String,
    pub field_mappings: StatementFieldMappings,
    #[serde(default)]
    pub parse_config: Option<ParseConfig>,
}

/// Which CSV columns contain the key fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatementFieldMappings {
    pub date_column: String,
    pub amount_column: String,
    #[serde(default)]
    pub description_column: Option<String>,
    #[serde(default)]
    pub type_column: Option<String>,
    #[serde(default)]
    pub default_currency: Option<String>,
}

/// Reconciliation configuration stored in app_settings KV.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationConfig {
    pub statements_dir: String,
    pub mappings: Vec<StatementAccountMapping>,
    /// Amount tolerance for matching (default: 0.01).
    #[serde(default)]
    pub amount_tolerance: Option<Decimal>,
}

/// Request to resolve reconciliation proposals.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveRequest {
    pub import_run_id: String,
    /// DRAFT activity IDs to promote to Posted.
    pub accept_ids: Vec<String>,
    /// DRAFT activity IDs to Void.
    pub reject_ids: Vec<String>,
}
