use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use wealthfolio_core::activities::ImportRun;
use wealthfolio_core::reconciliation::{
    ReconciliationConfig, ReconciliationResult, ResolveRequest, ScanResult,
};

use crate::error::ApiResult;
use crate::main_lib::AppState;

async fn scan(State(state): State<Arc<AppState>>) -> ApiResult<Json<ScanResult>> {
    let result = state.reconciliation_service.scan_directory().await?;
    Ok(Json(result))
}

async fn list_pending(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<ReconciliationResult>>> {
    let results = state
        .reconciliation_service
        .get_pending_reconciliations()
        .await?;
    Ok(Json(results))
}

async fn get_detail(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> ApiResult<Json<ReconciliationResult>> {
    let result = state.reconciliation_service.get_reconciliation(&run_id)?;
    Ok(Json(result))
}

async fn resolve(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ResolveRequest>,
) -> ApiResult<Json<ImportRun>> {
    let run = state.reconciliation_service.resolve(request).await?;
    Ok(Json(run))
}

async fn get_config(State(state): State<Arc<AppState>>) -> ApiResult<Json<ReconciliationConfig>> {
    let config = state.reconciliation_service.get_config()?;
    Ok(Json(config))
}

async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(config): Json<ReconciliationConfig>,
) -> ApiResult<Json<ReconciliationConfig>> {
    state
        .reconciliation_service
        .save_config(config.clone())
        .await?;
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
