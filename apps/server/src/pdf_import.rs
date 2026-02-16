//! PDF statement auto-import: folder watcher + in-memory staging store.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, info, warn};

use wealthfolio_ai::PdfTransactionParserTrait;
use wealthfolio_core::activities::ActivityImport;

/// How long staged imports remain before expiry.
const STAGING_EXPIRY_SECS: u64 = 3600; // 1 hour

/// Watcher poll interval.
const POLL_INTERVAL_SECS: u64 = 30;

/// A staged PDF import awaiting user review.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedImport {
    pub id: String,
    pub filename: String,
    pub activities: Vec<ActivityImport>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Summary of a staged import (for listing).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedImportSummary {
    pub id: String,
    pub filename: String,
    pub activity_count: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// In-memory staging store for parsed PDF imports.
#[derive(Default)]
pub struct StagingStore {
    imports: RwLock<HashMap<String, StagedImport>>,
}

impl StagingStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a new staged import.
    pub async fn insert(&self, import: StagedImport) {
        let mut map = self.imports.write().await;
        // Lazy cleanup of expired entries
        let now = chrono::Utc::now();
        map.retain(|_, v| {
            (now - v.created_at).num_seconds() < STAGING_EXPIRY_SECS as i64
        });
        map.insert(import.id.clone(), import);
    }

    /// List all non-expired staged imports (summary only).
    pub async fn list(&self) -> Vec<StagedImportSummary> {
        let map = self.imports.read().await;
        let now = chrono::Utc::now();
        map.values()
            .filter(|v| (now - v.created_at).num_seconds() < STAGING_EXPIRY_SECS as i64)
            .map(|v| StagedImportSummary {
                id: v.id.clone(),
                filename: v.filename.clone(),
                activity_count: v.activities.len(),
                created_at: v.created_at,
            })
            .collect()
    }

    /// Get a staged import by ID.
    pub async fn get(&self, id: &str) -> Option<StagedImport> {
        let map = self.imports.read().await;
        let now = chrono::Utc::now();
        map.get(id).and_then(|v| {
            if (now - v.created_at).num_seconds() < STAGING_EXPIRY_SECS as i64 {
                Some(v.clone())
            } else {
                None
            }
        })
    }

    /// Remove a staged import (after confirm or discard).
    pub async fn remove(&self, id: &str) -> Option<StagedImport> {
        self.imports.write().await.remove(id)
    }
}

/// Ensure inbox/processed/failed directories exist under data_root.
pub fn ensure_pdf_dirs(data_root: &str) -> std::io::Result<(PathBuf, PathBuf, PathBuf)> {
    let inbox = Path::new(data_root).join("pdf-inbox");
    let processed = Path::new(data_root).join("pdf-processed");
    let failed = Path::new(data_root).join("pdf-failed");
    std::fs::create_dir_all(&inbox)?;
    std::fs::create_dir_all(&processed)?;
    std::fs::create_dir_all(&failed)?;
    Ok((inbox, processed, failed))
}

/// Start the background PDF folder watcher.
pub fn start_pdf_watcher(
    state: Arc<crate::main_lib::AppState>,
    staging: Arc<StagingStore>,
) {
    tokio::spawn(async move {
        info!("PDF watcher started ({}s interval)", POLL_INTERVAL_SECS);

        // Initial delay to let server fully start
        tokio::time::sleep(Duration::from_secs(10)).await;

        let mut poll = interval(Duration::from_secs(POLL_INTERVAL_SECS));

        loop {
            poll.tick().await;
            if let Err(e) = scan_inbox(&state, &staging).await {
                warn!("PDF watcher scan error: {}", e);
            }
        }
    });
}

async fn scan_inbox(
    state: &Arc<crate::main_lib::AppState>,
    staging: &Arc<StagingStore>,
) -> anyhow::Result<()> {
    let inbox = Path::new(&state.data_root).join("pdf-inbox");
    let processed = Path::new(&state.data_root).join("pdf-processed");
    let failed = Path::new(&state.data_root).join("pdf-failed");

    if !inbox.exists() {
        return Ok(());
    }

    // Get default AI provider and model
    let (provider_id, model_id) = match get_default_ai_config(state) {
        Some(cfg) => cfg,
        None => {
            debug!("PDF watcher: no AI provider configured, skipping");
            return Ok(());
        }
    };

    let entries: Vec<_> = std::fs::read_dir(&inbox)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("pdf"))
                .unwrap_or(false)
        })
        .collect();

    for entry in entries {
        let path = entry.path();
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        info!("PDF watcher: processing {}", filename);

        match process_pdf(&path, &provider_id, &model_id, state).await {
            Ok(activities) => {
                let import = StagedImport {
                    id: uuid::Uuid::new_v4().to_string(),
                    filename: filename.clone(),
                    activities,
                    created_at: chrono::Utc::now(),
                };
                staging.insert(import).await;

                // Move to processed
                let dest = processed.join(&filename);
                if let Err(e) = std::fs::rename(&path, &dest) {
                    warn!("Failed to move {} to processed: {}", filename, e);
                }
                info!("PDF watcher: staged {}", filename);
            }
            Err(e) => {
                warn!("PDF watcher: failed to process {}: {}", filename, e);
                // Move to failed
                let dest = failed.join(&filename);
                if let Err(e) = std::fs::rename(&path, &dest) {
                    warn!("Failed to move {} to failed: {}", filename, e);
                }
            }
        }
    }

    Ok(())
}

/// Extract text from PDF and parse via LLM.
pub async fn process_pdf(
    path: &Path,
    provider_id: &str,
    model_id: &str,
    state: &Arc<crate::main_lib::AppState>,
) -> anyhow::Result<Vec<ActivityImport>> {
    let bytes = std::fs::read(path)?;
    let text = pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| anyhow::anyhow!("PDF text extraction failed: {}", e))?;

    if text.trim().is_empty() {
        anyhow::bail!("PDF contains no extractable text");
    }

    let parser = wealthfolio_ai::PdfTransactionParser::new(state.ai_environment.clone());
    let activities = parser
        .parse_transactions(&text, provider_id, model_id)
        .await
        .map_err(|e| anyhow::anyhow!("LLM parsing failed: {}", e))?;

    Ok(activities)
}

/// Get the default AI provider ID and model ID from settings (public for API use).
pub fn get_default_ai_config_from_state(state: &Arc<crate::main_lib::AppState>) -> Option<(String, String)> {
    get_default_ai_config(state)
}

/// Get the default AI provider ID and model ID from settings.
fn get_default_ai_config(state: &Arc<crate::main_lib::AppState>) -> Option<(String, String)> {
    let response = state.ai_provider_service.get_ai_providers().ok()?;
    let provider_id = response.default_provider?;
    let provider = response
        .providers
        .iter()
        .find(|p| p.id == provider_id && p.enabled)?;
    let model_id = provider
        .selected_model
        .clone()
        .unwrap_or_else(|| provider.default_model.clone());
    Some((provider_id, model_id))
}
