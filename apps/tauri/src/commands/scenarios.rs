use std::sync::Arc;

use chrono::NaiveDate;
use tauri::State;

use crate::context::ServiceContext;
use wealthfolio_agent_tools::{compute_basket_return_series, SymbolPerformanceOutput};
use wealthfolio_core::scenarios::{NewPortfolioScenario, PortfolioScenario, ScenarioKind};

#[tauri::command]
pub async fn get_scenarios(
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<PortfolioScenario>, String> {
    state
        .scenario_service()
        .list_scenarios()
        .map_err(|e| format!("Failed to load scenarios: {}", e))
}

#[tauri::command]
pub async fn get_scenario(
    scenario_id: String,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<PortfolioScenario, String> {
    state
        .scenario_service()
        .get_scenario(&scenario_id)
        .map_err(|e| format!("Failed to load scenario: {}", e))
}

#[tauri::command]
pub async fn create_scenario(
    scenario: NewPortfolioScenario,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<PortfolioScenario, String> {
    state
        .scenario_service()
        .create_scenario(scenario)
        .await
        .map_err(|e| format!("Failed to create scenario: {}", e))
}

#[tauri::command]
pub async fn update_scenario_entry(
    scenario_id: String,
    scenario: NewPortfolioScenario,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<PortfolioScenario, String> {
    state
        .scenario_service()
        .update_scenario(&scenario_id, scenario)
        .await
        .map_err(|e| format!("Failed to update scenario: {}", e))
}

/// Replay a saved basket scenario over history and return its return series,
/// shaped so the Performance page can plot it as a benchmark reference line.
#[tauri::command]
pub async fn calculate_scenario_performance(
    scenario_id: String,
    start_date: Option<String>,
    end_date: Option<String>,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<SymbolPerformanceOutput, String> {
    let scenario = state
        .scenario_service()
        .get_scenario(&scenario_id)
        .map_err(|e| format!("Failed to load scenario: {}", e))?;
    if scenario.kind != ScenarioKind::Basket || scenario.basket.is_empty() {
        return Err("Scenario has no basket to replay.".to_string());
    }
    let start = parse_optional_date(start_date.as_deref())?;
    let end = parse_optional_date(end_date.as_deref())?;
    Ok(compute_basket_return_series(state.agent_environment(), &scenario.basket, start, end).await)
}

fn parse_optional_date(value: Option<&str>) -> Result<Option<NaiveDate>, String> {
    match value.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| format!("Invalid date '{}'. Expected YYYY-MM-DD.", s)),
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn delete_scenario_entry(
    scenario_id: String,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<(), String> {
    state
        .scenario_service()
        .delete_scenario(&scenario_id)
        .await
        .map_err(|e| format!("Failed to delete scenario: {}", e))
}
