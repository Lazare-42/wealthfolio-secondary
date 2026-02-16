use std::sync::Arc;

use axum::{
    extract::{Multipart, Path, State},
    routing::{get, post},
    Json, Router,
};
use wealthfolio_ai::PdfTransactionParserTrait;
use wealthfolio_core::activities::{ActivityImport, ImportActivitiesResult};

use crate::error::{ApiError, ApiResult};
use crate::main_lib::AppState;
use crate::pdf_import::{StagedImport, StagedImportSummary};

/// List all staged PDF imports.
async fn list_staged(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<StagedImportSummary>>> {
    let list = state.pdf_staging.list().await;
    Ok(Json(list))
}

/// Get a staged import's activities for review.
async fn get_staged(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<StagedImport>> {
    let import = state.pdf_staging.get(&id).await.ok_or(ApiError::NotFound)?;
    Ok(Json(import))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmBody {
    account_id: String,
    activities: Vec<ActivityImport>,
}

/// Confirm a staged import: validate and import activities.
async fn confirm_staged(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<ConfirmBody>,
) -> ApiResult<Json<ImportActivitiesResult>> {
    // Verify the staged import exists
    state.pdf_staging.get(&id).await.ok_or(ApiError::NotFound)?;

    // Import via the existing pipeline
    let result = state
        .activity_service
        .import_activities(body.account_id, body.activities)
        .await?;

    // Remove from staging
    state.pdf_staging.remove(&id).await;

    Ok(Json(result))
}

/// Discard a staged import.
async fn discard_staged(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<()>> {
    state
        .pdf_staging
        .remove(&id)
        .await
        .ok_or(ApiError::NotFound)?;
    Ok(Json(()))
}

/// Manual PDF upload (multipart).
async fn upload_pdf(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> ApiResult<Json<StagedImport>> {
    let mut file_content: Option<Vec<u8>> = None;
    let mut filename = "upload.pdf".to_string();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read multipart field: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            if let Some(fname) = field.file_name() {
                filename = fname.to_string();
            }
            file_content = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| {
                        ApiError::BadRequest(format!("Failed to read file content: {}", e))
                    })?
                    .to_vec(),
            );
        }
    }

    let content = file_content
        .ok_or_else(|| ApiError::BadRequest("Missing file in multipart request".to_string()))?;

    // Get AI provider config
    let (provider_id, model_id) = crate::pdf_import::get_default_ai_config_from_state(&state)
        .ok_or_else(|| {
            ApiError::BadRequest(
                "No AI provider configured. Please configure an AI provider first.".to_string(),
            )
        })?;

    // Extract text from PDF bytes
    let text = pdf_extract::extract_text_from_mem(&content)
        .map_err(|e| ApiError::Internal(format!("PDF text extraction failed: {}", e)))?;

    if text.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "PDF contains no extractable text".to_string(),
        ));
    }

    // Parse via LLM
    let parser = wealthfolio_ai::PdfTransactionParser::new(state.ai_environment.clone());
    let activities = parser
        .parse_transactions(&text, &provider_id, &model_id)
        .await
        .map_err(|e| ApiError::Internal(format!("LLM parsing failed: {}", e)))?;

    let import = StagedImport {
        id: uuid::Uuid::new_v4().to_string(),
        filename,
        activities,
        created_at: chrono::Utc::now(),
    };

    state.pdf_staging.insert(import.clone()).await;

    Ok(Json(import))
}

/// Check staged activities against existing data (validation + duplicate detection).
async fn check_staged(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<CheckBody>,
) -> ApiResult<Json<Vec<ActivityImport>>> {
    let _import = state.pdf_staging.get(&id).await.ok_or(ApiError::NotFound)?;

    let checked = state
        .activity_service
        .check_activities_import(body.account_id, body.activities)
        .await?;

    Ok(Json(checked))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckBody {
    account_id: String,
    activities: Vec<ActivityImport>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/pdf-imports/staged", get(list_staged))
        .route(
            "/pdf-imports/staged/{id}",
            get(get_staged).delete(discard_staged),
        )
        .route("/pdf-imports/staged/{id}/confirm", post(confirm_staged))
        .route("/pdf-imports/staged/{id}/check", post(check_staged))
        .route("/pdf-imports/upload", post(upload_pdf))
}
