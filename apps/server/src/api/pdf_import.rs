use std::sync::Arc;

use axum::{
    extract::{Multipart, Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use wealthfolio_core::activities::ActivityImport;

use crate::error::{ApiError, ApiResult};
use crate::main_lib::AppState;
use crate::pdf_import::{self, StagedImport, StagedImportSource, StagedImportSummary};

// ============================================================================
// Handlers
// ============================================================================

/// GET /pdf-imports/staged — list all staged imports.
async fn list_staged(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<StagedImportSummary>>> {
    let summaries = state.pdf_staging.list();
    Ok(Json(summaries))
}

/// GET /pdf-imports/staged/{id} — get full detail of a staged import.
async fn get_staged(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<StagedImport>> {
    let import = state.pdf_staging.get(&id).ok_or(ApiError::NotFound)?;
    Ok(Json(import))
}

/// DELETE /pdf-imports/staged/{id} — discard a staged import.
async fn delete_staged(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<()>> {
    state.pdf_staging.remove(&id);
    Ok(Json(()))
}

/// POST /pdf-imports/staged/{id}/confirm — convert staged transactions to activities.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmRequest {
    account_id: String,
    activities: Vec<ActivityImport>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmResponse {
    imported_count: usize,
}

async fn confirm_staged(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<ConfirmRequest>,
) -> ApiResult<Json<ConfirmResponse>> {
    // Verify staged import exists (don't remove yet)
    state.pdf_staging.get(&id).ok_or(ApiError::NotFound)?;

    // Set account_id on all activities and import
    let activities: Vec<ActivityImport> = request
        .activities
        .into_iter()
        .map(|mut a| {
            a.account_id = Some(request.account_id.clone());
            a
        })
        .collect();

    let result = state.activity_service.import_activities(activities).await?;

    // Remove from staging only after successful import
    state.pdf_staging.remove(&id);

    Ok(Json(ConfirmResponse {
        imported_count: result.summary.imported as usize,
    }))
}

/// POST /pdf-imports/staged/{id}/check — validate staged transactions (duplicate detection).
async fn check_staged(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<CheckRequest>,
) -> ApiResult<Json<CheckResponse>> {
    let import = state.pdf_staging.get(&id).ok_or(ApiError::NotFound)?;

    // Build ActivityImport list with account_id for duplicate checking
    let activities: Vec<ActivityImport> = import
        .transactions
        .iter()
        .enumerate()
        .map(|(i, t)| ActivityImport {
            id: None,
            date: t.date.clone(),
            symbol: String::new(),
            activity_type: t.activity_type.clone(),
            quantity: None,
            unit_price: None,
            currency: t.currency.clone(),
            fee: t
                .fee
                .map(|f| rust_decimal::Decimal::from_f64_retain(f).unwrap_or_default()),
            amount: Some(rust_decimal::Decimal::from_f64_retain(t.amount).unwrap_or_default()),
            comment: Some(t.description.clone()),
            provider_id: None,
            provider_symbol: None,
            is_external: None,
            account_id: Some(request.account_id.clone()),
            account_name: None,
            symbol_name: None,
            exchange_mic: None,
            quote_ccy: None,
            instrument_type: None,
            quote_mode: None,
            errors: None,
            warnings: None,
            duplicate_of_id: None,
            duplicate_of_line_number: None,
            is_draft: false,
            is_valid: true,
            line_number: Some(i as i32 + 1),
            fx_rate: None,
            subtype: None,
            asset_id: None,
            isin: None,
            force_import: false,
        })
        .collect();

    let checked = state
        .activity_service
        .check_activities_import(activities)
        .await?;

    let duplicate_count = checked
        .iter()
        .filter(|a| a.duplicate_of_id.is_some())
        .count();

    Ok(Json(CheckResponse {
        duplicate_count,
        total_count: import.transactions.len(),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckRequest {
    account_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckResponse {
    duplicate_count: usize,
    total_count: usize,
}

/// POST /pdf-imports/upload — multipart file upload.
async fn upload_pdf(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> ApiResult<Json<StagedImport>> {
    let mut file_name = String::from("upload.pdf");
    let mut pdf_bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Multipart error: {}", e)))?
    {
        if field.name() == Some("file") {
            if let Some(name) = field.file_name() {
                file_name = name.to_string();
            }
            pdf_bytes = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?
                    .to_vec(),
            );
        }
    }

    let bytes = pdf_bytes.ok_or_else(|| ApiError::BadRequest("No file field in upload".into()))?;

    let (provider_id, model_id) =
        pdf_import::get_default_ai_config(state.ai_provider_service.as_ref())
            .map_err(|e| ApiError::BadRequest(e))?;

    let import = pdf_import::process_pdf(
        &bytes,
        &file_name,
        state.pdf_parser.as_ref(),
        &provider_id,
        &model_id,
        StagedImportSource::Upload,
        None,
    )
    .await
    .map_err(|e| ApiError::Internal(e))?;

    state.pdf_staging.insert(import.clone());
    Ok(Json(import))
}

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/pdf-imports/staged", get(list_staged))
        .route(
            "/pdf-imports/staged/{id}",
            get(get_staged).delete(delete_staged),
        )
        .route("/pdf-imports/staged/{id}/confirm", post(confirm_staged))
        .route("/pdf-imports/staged/{id}/check", post(check_staged))
        .route("/pdf-imports/upload", post(upload_pdf))
}
