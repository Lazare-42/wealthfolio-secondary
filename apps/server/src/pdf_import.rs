//! PDF import pipeline: staging store, PDF processing, and folder watcher.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use wealthfolio_ai::pdf_parser::{PdfTransaction, PdfTransactionParserTrait};
use wealthfolio_ai::provider_service::AiProviderServiceTrait;

// ============================================================================
// Models
// ============================================================================

/// Summary of a staged import (for list view).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedImportSummary {
    pub id: String,
    pub file_name: String,
    pub transaction_count: usize,
    pub staged_at: String,
    pub source: StagedImportSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_account_id: Option<String>,
}

/// Full staged import with all parsed transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedImport {
    pub id: String,
    pub file_name: String,
    pub transactions: Vec<PdfTransaction>,
    pub staged_at: String,
    pub source: StagedImportSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StagedImportSource {
    Upload,
    FolderWatcher,
}

/// Internal entry with expiry tracking.
struct StagingEntry {
    import: StagedImport,
    created: Instant,
}

// ============================================================================
// Staging Store
// ============================================================================

const STAGING_EXPIRY: Duration = Duration::from_secs(3600); // 1 hour

#[derive(Default)]
pub struct StagingStore {
    entries: RwLock<HashMap<String, StagingEntry>>,
}

impl StagingStore {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Insert a staged import. Returns its ID.
    pub fn insert(&self, import: StagedImport) -> String {
        let id = import.id.clone();
        let mut entries = self.entries.write().unwrap();
        // Lazy cleanup
        entries.retain(|_, e| e.created.elapsed() < STAGING_EXPIRY);
        entries.insert(
            id.clone(),
            StagingEntry {
                import,
                created: Instant::now(),
            },
        );
        id
    }

    /// List summaries of all non-expired staged imports.
    pub fn list(&self) -> Vec<StagedImportSummary> {
        let mut entries = self.entries.write().unwrap();
        entries.retain(|_, e| e.created.elapsed() < STAGING_EXPIRY);
        entries
            .values()
            .map(|e| StagedImportSummary {
                id: e.import.id.clone(),
                file_name: e.import.file_name.clone(),
                transaction_count: e.import.transactions.len(),
                staged_at: e.import.staged_at.clone(),
                source: e.import.source.clone(),
                suggested_account_id: e.import.suggested_account_id.clone(),
            })
            .collect()
    }

    /// Get a staged import by ID.
    pub fn get(&self, id: &str) -> Option<StagedImport> {
        let entries = self.entries.read().unwrap();
        entries
            .get(id)
            .filter(|e| e.created.elapsed() < STAGING_EXPIRY)
            .map(|e| e.import.clone())
    }

    /// Remove a staged import by ID.
    pub fn remove(&self, id: &str) -> Option<StagedImport> {
        let mut entries = self.entries.write().unwrap();
        entries.remove(id).map(|e| e.import)
    }
}

// ============================================================================
// PDF Processing
// ============================================================================

/// Extract text from a PDF file and parse transactions via LLM.
pub async fn process_pdf(
    pdf_bytes: &[u8],
    file_name: &str,
    parser: &dyn PdfTransactionParserTrait,
    provider_id: &str,
    model_id: &str,
    source: StagedImportSource,
    suggested_account_id: Option<String>,
) -> Result<StagedImport, String> {
    // Extract text
    let text = pdf_extract::extract_text_from_mem(pdf_bytes)
        .map_err(|e| format!("Failed to extract text from PDF: {}", e))?;

    // Parse via LLM
    let transactions = parser
        .parse_transactions(&text, provider_id, model_id)
        .await
        .map_err(|e| format!("LLM parsing failed: {}", e))?;

    let id = uuid::Uuid::now_v7().to_string();
    let staged_at = chrono::Utc::now().to_rfc3339();

    Ok(StagedImport {
        id,
        file_name: file_name.to_string(),
        transactions,
        staged_at,
        source,
        suggested_account_id,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SidecarMetadata {
    wealthfolio_account_id: Option<String>,
}

// ============================================================================
// Default AI Config Helper
// ============================================================================

/// Read the default provider + model from AI provider settings.
pub fn get_default_ai_config(
    ai_provider_service: &dyn AiProviderServiceTrait,
) -> Result<(String, String), String> {
    let response = ai_provider_service
        .get_ai_providers()
        .map_err(|e| format!("Failed to get AI providers: {}", e))?;

    let provider_id = response
        .default_provider
        .ok_or("No default AI provider configured")?;

    // Find the provider and get its selected or default model
    let provider = response
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("Default provider '{}' not found in catalog", provider_id))?;

    let model_id = provider
        .selected_model
        .clone()
        .unwrap_or_else(|| provider.default_model.clone());

    Ok((provider_id, model_id))
}

// ============================================================================
// Folder Watcher
// ============================================================================

/// Start a background task that polls `{data_root}/pdf-inbox/` every 30s.
/// Processed files move to `pdf-processed/`; failed files move to `pdf-failed/`.
pub fn start_pdf_watcher(
    data_root: String,
    staging: Arc<StagingStore>,
    parser: Arc<dyn PdfTransactionParserTrait>,
    ai_provider_service: Arc<dyn AiProviderServiceTrait + Send + Sync>,
) {
    tokio::spawn(async move {
        let inbox = PathBuf::from(&data_root).join("pdf-inbox");
        let processed = PathBuf::from(&data_root).join("pdf-processed");
        let failed = PathBuf::from(&data_root).join("pdf-failed");

        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;

            // Ensure directories exist
            for dir in [&inbox, &processed, &failed] {
                if let Err(e) = tokio::fs::create_dir_all(dir).await {
                    warn!("Failed to create directory {:?}: {}", dir, e);
                }
            }

            let mut read_dir = match tokio::fs::read_dir(&inbox).await {
                Ok(rd) => rd,
                Err(_) => continue,
            };

            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let path = entry.path();
                if !is_pdf(&path) {
                    continue;
                }

                let file_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let sidecar_path = sidecar_path_for_pdf(&path);

                info!("PDF watcher: processing {}", file_name);

                let (provider_id, model_id) =
                    match get_default_ai_config(ai_provider_service.as_ref()) {
                        Ok(config) => config,
                        Err(e) => {
                            warn!("PDF watcher: no AI config, skipping: {}", e);
                            continue;
                        }
                    };

                let pdf_bytes = match tokio::fs::read(&path).await {
                    Ok(b) => b,
                    Err(e) => {
                        error!("PDF watcher: failed to read {:?}: {}", path, e);
                        move_file(&path, &failed.join(&file_name)).await;
                        move_sidecar_if_present(
                            &sidecar_path,
                            &failed.join(sidecar_file_name(&path)),
                        )
                        .await;
                        continue;
                    }
                };

                let suggested_account_id = match read_sidecar_metadata(&sidecar_path).await {
                    Ok(metadata) => metadata.and_then(|m| m.wealthfolio_account_id),
                    Err(e) => {
                        warn!(
                            "PDF watcher: failed to read sidecar for {}: {}",
                            file_name, e
                        );
                        None
                    }
                };

                match process_pdf(
                    &pdf_bytes,
                    &file_name,
                    parser.as_ref(),
                    &provider_id,
                    &model_id,
                    StagedImportSource::FolderWatcher,
                    suggested_account_id,
                )
                .await
                {
                    Ok(import) => {
                        info!(
                            "PDF watcher: staged {} with {} transactions",
                            file_name,
                            import.transactions.len()
                        );
                        staging.insert(import);
                        move_file(&path, &processed.join(&file_name)).await;
                        move_sidecar_if_present(
                            &sidecar_path,
                            &processed.join(sidecar_file_name(&path)),
                        )
                        .await;
                    }
                    Err(e) => {
                        error!("PDF watcher: failed to process {}: {}", file_name, e);
                        move_file(&path, &failed.join(&file_name)).await;
                        move_sidecar_if_present(
                            &sidecar_path,
                            &failed.join(sidecar_file_name(&path)),
                        )
                        .await;
                    }
                }
            }
        }
    });
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .map(|ext| ext.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

async fn move_file(from: &Path, to: &Path) {
    if let Err(e) = tokio::fs::rename(from, to).await {
        warn!("Failed to move {:?} -> {:?}: {}", from, to, e);
    }
}

fn sidecar_path_for_pdf(path: &Path) -> PathBuf {
    path.with_extension("json")
}

fn sidecar_file_name(path: &Path) -> String {
    sidecar_path_for_pdf(path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

async fn read_sidecar_metadata(path: &Path) -> Result<Option<SidecarMetadata>, String> {
    let bytes = match tokio::fs::read(path).await {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("failed to read sidecar: {}", err)),
    };

    serde_json::from_slice::<SidecarMetadata>(&bytes)
        .map(Some)
        .map_err(|err| format!("failed to parse sidecar JSON: {}", err))
}

async fn move_sidecar_if_present(from: &Path, to: &Path) {
    match tokio::fs::metadata(from).await {
        Ok(_) => move_file(from, to).await,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => warn!("Failed to stat sidecar {:?}: {}", from, err),
    }
}
