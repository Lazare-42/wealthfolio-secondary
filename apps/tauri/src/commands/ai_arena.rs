use std::sync::Arc;

use tauri::State;

use crate::context::ServiceContext;
use wealthfolio_core::ai_arena::{
    ArenaAgent, ArenaChallenge, ArenaLeaderboard, ArenaParticipant, ArenaPortfolio, ArenaRun,
    ArenaTrade, CompanyThesis, CreateArenaAgentRequest, CreateArenaChallengeRequest,
    CreateCompanyThesisRequest, RunArenaAgentRequest,
};

#[tauri::command]
pub async fn get_arena_agents(
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<ArenaAgent>, String> {
    state
        .ai_arena_service()
        .list_agents()
        .map_err(|e| format!("Failed to load arena agents: {}", e))
}

#[tauri::command]
pub async fn create_arena_agent(
    agent: CreateArenaAgentRequest,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<ArenaAgent, String> {
    state
        .ai_arena_service()
        .create_agent(agent)
        .await
        .map_err(|e| format!("Failed to create arena agent: {}", e))
}

#[tauri::command]
pub async fn get_arena_challenges(
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<ArenaChallenge>, String> {
    state
        .ai_arena_service()
        .list_challenges()
        .map_err(|e| format!("Failed to load arena challenges: {}", e))
}

#[tauri::command]
pub async fn create_arena_challenge(
    challenge: CreateArenaChallengeRequest,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<ArenaChallenge, String> {
    state
        .ai_arena_service()
        .create_challenge(challenge)
        .await
        .map_err(|e| format!("Failed to create arena challenge: {}", e))
}

#[tauri::command]
pub async fn join_arena_challenge(
    challenge_id: String,
    agent_id: String,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<ArenaParticipant, String> {
    state
        .ai_arena_service()
        .join_challenge(&challenge_id, &agent_id)
        .await
        .map_err(|e| format!("Failed to join arena challenge: {}", e))
}

#[tauri::command]
pub async fn get_arena_participants(
    challenge_id: String,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<ArenaParticipant>, String> {
    state
        .ai_arena_service()
        .list_participants(&challenge_id)
        .map_err(|e| format!("Failed to load arena participants: {}", e))
}

#[tauri::command]
pub async fn run_arena_agent(
    request: RunArenaAgentRequest,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<ArenaRun, String> {
    state
        .ai_arena_service()
        .run_agent(request)
        .await
        .map_err(|e| format!("Failed to run arena agent: {}", e))
}

#[tauri::command]
pub async fn run_due_arena_agents(
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<ArenaRun>, String> {
    state
        .ai_arena_service()
        .run_due_scheduled()
        .await
        .map_err(|e| format!("Failed to run due arena agents: {}", e))
}

#[tauri::command]
pub async fn settle_arena_challenge(
    challenge_id: String,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<ArenaLeaderboard, String> {
    state
        .ai_arena_service()
        .settle_challenge(&challenge_id)
        .await
        .map_err(|e| format!("Failed to settle arena challenge: {}", e))
}

#[tauri::command]
pub async fn get_arena_leaderboard(
    challenge_id: String,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<ArenaLeaderboard, String> {
    state
        .ai_arena_service()
        .get_leaderboard(&challenge_id)
        .await
        .map_err(|e| format!("Failed to load arena leaderboard: {}", e))
}

#[tauri::command]
pub async fn get_arena_portfolio(
    participant_id: String,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<ArenaPortfolio, String> {
    state
        .ai_arena_service()
        .get_portfolio(&participant_id)
        .await
        .map_err(|e| format!("Failed to load arena portfolio: {}", e))
}

#[tauri::command]
pub async fn get_arena_runs(
    challenge_id: String,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<ArenaRun>, String> {
    state
        .ai_arena_service()
        .list_runs(&challenge_id)
        .map_err(|e| format!("Failed to load arena runs: {}", e))
}

#[tauri::command]
pub async fn get_arena_trades(
    challenge_id: String,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<ArenaTrade>, String> {
    state
        .ai_arena_service()
        .list_trades(&challenge_id)
        .map_err(|e| format!("Failed to load arena trades: {}", e))
}

#[tauri::command]
pub async fn create_company_thesis(
    thesis: CreateCompanyThesisRequest,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<CompanyThesis, String> {
    state
        .ai_arena_service()
        .create_thesis(thesis)
        .await
        .map_err(|e| format!("Failed to create company thesis: {}", e))
}

#[tauri::command]
pub async fn get_company_theses(
    symbol: Option<String>,
    challenge_id: Option<String>,
    limit: Option<i64>,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<CompanyThesis>, String> {
    state
        .ai_arena_service()
        .list_theses(
            symbol.as_deref(),
            challenge_id.as_deref(),
            limit.unwrap_or(50),
        )
        .map_err(|e| format!("Failed to load company theses: {}", e))
}
