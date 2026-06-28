use std::sync::Arc;

use tauri::State;

use crate::context::ServiceContext;
use wealthfolio_core::scenarios::{NewPortfolioScenario, PortfolioScenario};

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
