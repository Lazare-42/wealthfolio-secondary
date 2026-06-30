use std::sync::Arc;

use crate::{
    error::{ApiError, ApiResult},
    main_lib::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::NaiveDate;
use serde::Deserialize;
use wealthfolio_agent_tools::{compute_basket_return_series, SymbolPerformanceOutput};
use wealthfolio_core::scenarios::{NewPortfolioScenario, PortfolioScenario, ScenarioKind};

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScenarioPerformanceRequest {
    #[serde(default)]
    start_date: Option<String>,
    #[serde(default)]
    end_date: Option<String>,
}

async fn calculate_scenario_performance(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ScenarioPerformanceRequest>,
) -> ApiResult<Json<SymbolPerformanceOutput>> {
    let scenario = state.scenario_service.get_scenario(&id)?;
    if scenario.kind != ScenarioKind::Basket || scenario.basket.is_empty() {
        return Err(ApiError::BadRequest(
            "Scenario has no basket to replay.".to_string(),
        ));
    }
    let start = parse_optional_date(payload.start_date.as_deref())?;
    let end = parse_optional_date(payload.end_date.as_deref())?;
    let series = compute_basket_return_series(
        state.agent_environment.clone(),
        &scenario.basket,
        start,
        end,
    )
    .await;
    Ok(Json(series))
}

fn parse_optional_date(value: Option<&str>) -> Result<Option<NaiveDate>, ApiError> {
    match value.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| {
                ApiError::BadRequest(format!("Invalid date '{}'. Expected YYYY-MM-DD.", s))
            }),
        None => Ok(None),
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/scenarios", get(list_scenarios).post(create_scenario))
        .route(
            "/scenarios/{id}",
            get(get_scenario)
                .put(update_scenario)
                .delete(delete_scenario),
        )
        .route(
            "/scenarios/{id}/performance",
            axum::routing::post(calculate_scenario_performance),
        )
}
