use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use wealthfolio_core::reconciliation::{
    ReconciliationConfig, ReconciliationResult, ResolveRequest, ScanResult,
};
use wealthfolio_core::sync::ImportRun;

use crate::error::{ApiError, ApiResult};
use crate::main_lib::AppState;

fn require_service(
    state: &AppState,
) -> Result<
    &(dyn wealthfolio_core::reconciliation::ReconciliationServiceTrait + Send + Sync),
    ApiError,
> {
    state.reconciliation_service.as_deref().ok_or_else(|| {
        ApiError::BadRequest("Reconciliation not configured. Set WF_STATEMENTS_DIR.".into())
    })
}

async fn scan(State(state): State<Arc<AppState>>) -> ApiResult<Json<ScanResult>> {
    let service = require_service(&state)?;
    let result = service.scan_directory().await?;
    Ok(Json(result))
}

async fn list_pending(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<ReconciliationResult>>> {
    let service = require_service(&state)?;
    let results = service.get_pending_reconciliations().await?;
    Ok(Json(results))
}

async fn get_detail(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> ApiResult<Json<ReconciliationResult>> {
    let service = require_service(&state)?;
    let result = service.get_reconciliation(&run_id)?;
    Ok(Json(result))
}

async fn resolve(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ResolveRequest>,
) -> ApiResult<Json<ImportRun>> {
    let service = require_service(&state)?;
    let run = service.resolve(request).await?;
    Ok(Json(run))
}

async fn get_config(State(state): State<Arc<AppState>>) -> ApiResult<Json<ReconciliationConfig>> {
    let service = require_service(&state)?;
    let config = service.get_config()?;
    Ok(Json(config))
}

async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(config): Json<ReconciliationConfig>,
) -> ApiResult<Json<ReconciliationConfig>> {
    let service = require_service(&state)?;
    service.save_config(config.clone()).await?;
    Ok(Json(config))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/reconciliation/scan", post(scan))
        .route("/reconciliation/pending", get(list_pending))
        .route("/reconciliation/{run_id}", get(get_detail))
        .route("/reconciliation/resolve", post(resolve))
        .route("/reconciliation/config", get(get_config).put(update_config))
}
