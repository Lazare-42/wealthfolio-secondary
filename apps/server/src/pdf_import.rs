//! PDF statement auto-import: folder watcher + in-memory staging store.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, info, warn};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use wealthfolio_ai::{
    DocumentMediaType, ImageMediaType, OneOrMany, PdfTransactionParserTrait, RigDocument,
    RigDocumentSourceKind, RigMessage, RigUserContent, PARSE_INSTRUCTIONS,
};
use wealthfolio_core::activities::ActivityImport;

/// How long staged imports remain before expiry.
const STAGING_EXPIRY_SECS: u64 = 3600; // 1 hour

/// Watcher poll interval.
const POLL_INTERVAL_SECS: u64 = 30;

/// Minimum extracted text length to use the text path; below this, use vision.
const MIN_TEXT_LENGTH: usize = 50;

/// Maximum pages to render for vision fallback.
const MAX_VISION_PAGES: u32 = 20;

/// DPI for rendering PDF pages to images.
const VISION_DPI: u32 = 150;

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

/// Whether the provider supports native PDF document input (no rendering needed).
fn supports_native_pdf(provider_id: &str) -> bool {
    matches!(provider_id, "anthropic" | "gemini" | "google")
}

/// Render PDF pages to PNG images via `pdftoppm`.
/// Returns `None` if `pdftoppm` is not found, `Some(Vec<PNG bytes>)` on success.
fn render_pdf_to_images(pdf_bytes: &[u8]) -> anyhow::Result<Option<Vec<Vec<u8>>>> {
    // Check if pdftoppm is available
    if std::process::Command::new("pdftoppm")
        .arg("--help")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_err()
    {
        return Ok(None);
    }

    let tmp_dir = tempfile::tempdir()?;
    let input_path = tmp_dir.path().join("input.pdf");
    std::fs::write(&input_path, pdf_bytes)?;

    let output_prefix = tmp_dir.path().join("page");
    let status = std::process::Command::new("pdftoppm")
        .args([
            "-png",
            "-r",
            &VISION_DPI.to_string(),
            "-l",
            &MAX_VISION_PAGES.to_string(),
        ])
        .arg(&input_path)
        .arg(&output_prefix)
        .status()?;

    if !status.success() {
        anyhow::bail!("pdftoppm exited with status {}", status);
    }

    // Read output PNGs (sorted by name for page order)
    let mut png_paths: Vec<PathBuf> = std::fs::read_dir(tmp_dir.path())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "png").unwrap_or(false))
        .collect();
    png_paths.sort();

    let images: Vec<Vec<u8>> = png_paths
        .iter()
        .map(std::fs::read)
        .collect::<Result<_, _>>()?;

    Ok(Some(images))
}

/// Build a multipart rig Message for vision-based PDF parsing.
fn build_vision_message(
    pdf_bytes: &[u8],
    page_images: Option<Vec<Vec<u8>>>,
    provider_id: &str,
) -> anyhow::Result<RigMessage> {
    let mut parts = OneOrMany::one(RigUserContent::text(PARSE_INSTRUCTIONS));

    if supports_native_pdf(provider_id) {
        // Anthropic/Gemini: send base64-encoded PDF as document
        let b64_pdf = BASE64.encode(pdf_bytes);
        parts.push(RigUserContent::Document(RigDocument {
            data: RigDocumentSourceKind::Base64(b64_pdf),
            media_type: Some(DocumentMediaType::PDF),
            additional_params: None,
        }));
    } else {
        // OpenAI/OpenRouter/others: send rendered page images
        let images = page_images
            .ok_or_else(|| anyhow::anyhow!(
                "pdftoppm is required for vision-based PDF parsing with this provider. \
                 Install poppler-utils (e.g., `apt install poppler-utils` or `brew install poppler`)."
            ))?;

        for img_bytes in &images {
            let b64 = BASE64.encode(img_bytes);
            parts.push(RigUserContent::image_base64(
                b64,
                Some(ImageMediaType::PNG),
                None,
            ));
        }
    }

    Ok(RigMessage::User { content: parts })
}

/// Process PDF bytes: try text extraction, fall back to vision if text is too short.
pub async fn process_pdf_bytes(
    bytes: &[u8],
    provider_id: &str,
    model_id: &str,
    state: &Arc<crate::main_lib::AppState>,
) -> anyhow::Result<Vec<ActivityImport>> {
    let text = pdf_extract::extract_text_from_mem(bytes).unwrap_or_default();

    if text.trim().len() >= MIN_TEXT_LENGTH {
        // Text path
        debug!("PDF has sufficient text ({} chars), using text path", text.trim().len());
        let parser = wealthfolio_ai::PdfTransactionParser::new(state.ai_environment.clone());
        let activities = parser
            .parse_transactions(&text, provider_id, model_id)
            .await
            .map_err(|e| anyhow::anyhow!("LLM parsing failed: {}", e))?;
        return Ok(activities);
    }

    // Vision fallback
    let native_pdf = supports_native_pdf(provider_id);
    info!(
        "PDF text too short ({} chars), using vision fallback (native_pdf={}, pdf_size={} bytes)",
        text.trim().len(),
        native_pdf,
        bytes.len()
    );

    let page_images = if !native_pdf {
        render_pdf_to_images(bytes)?
    } else {
        None
    };

    let message = build_vision_message(bytes, page_images, provider_id)?;

    info!("Sending vision request to {} model {}", provider_id, model_id);
    let parser = wealthfolio_ai::PdfTransactionParser::new(state.ai_environment.clone());
    let activities = parser
        .parse_transactions_vision(message, provider_id, model_id)
        .await
        .map_err(|e| anyhow::anyhow!("LLM vision parsing failed: {}", e))?;

    Ok(activities)
}

/// Extract text from PDF file and parse via LLM, with vision fallback for scanned PDFs.
pub async fn process_pdf(
    path: &Path,
    provider_id: &str,
    model_id: &str,
    state: &Arc<crate::main_lib::AppState>,
) -> anyhow::Result<Vec<ActivityImport>> {
    let bytes = std::fs::read(path)?;
    process_pdf_bytes(&bytes, provider_id, model_id, state).await
}

/// Get the default AI provider ID and model ID from settings (public for API use).
pub fn get_default_ai_config_from_state(state: &Arc<crate::main_lib::AppState>) -> Option<(String, String)> {
    get_default_ai_config(state)
}

/// Get the default AI provider ID and model ID from settings.
/// Falls back to the first enabled provider if no explicit default is set.
fn get_default_ai_config(state: &Arc<crate::main_lib::AppState>) -> Option<(String, String)> {
    let response = state.ai_provider_service.get_ai_providers().ok()?;

    // Try explicit default first, then fall back to first enabled provider
    let provider = if let Some(ref default_id) = response.default_provider {
        response.providers.iter().find(|p| p.id == *default_id && p.enabled)
    } else {
        None
    }
    .or_else(|| response.providers.iter().find(|p| p.enabled))?;

    let model_id = provider
        .selected_model
        .clone()
        .unwrap_or_else(|| provider.default_model.clone());
    Some((provider.id.clone(), model_id))
}
