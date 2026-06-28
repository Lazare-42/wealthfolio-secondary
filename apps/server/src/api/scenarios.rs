use std::sync::Arc;

use crate::{error::ApiResult, main_lib::AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use wealthfolio_core::scenarios::{NewPortfolioScenario, PortfolioScenario};

async fn list_scenarios(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<PortfolioScenario>>> {
    let scenarios = state.scenario_service.list_scenarios()?;
    Ok(Json(scenarios))
}

async fn get_scenario(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<PortfolioScenario>> {
    let scenario = state.scenario_service.get_scenario(&id)?;
    Ok(Json(scenario))
}

async fn create_scenario(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NewPortfolioScenario>,
) -> ApiResult<Json<PortfolioScenario>> {
    let created = state.scenario_service.create_scenario(payload).await?;
    Ok(Json(created))
}

async fn update_scenario(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NewPortfolioScenario>,
) -> ApiResult<Json<PortfolioScenario>> {
    let updated = state.scenario_service.update_scenario(&id, payload).await?;
    Ok(Json(updated))
}

async fn delete_scenario(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<StatusCode> {
    state.scenario_service.delete_scenario(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/scenarios", get(list_scenarios).post(create_scenario))
        .route(
            "/scenarios/{id}",
            get(get_scenario).put(update_scenario).delete(delete_scenario),
        )
}
