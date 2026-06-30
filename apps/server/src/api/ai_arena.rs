use std::sync::Arc;

use crate::{error::ApiResult, main_lib::AppState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use wealthfolio_core::ai_arena::{
    ArenaAgent, ArenaChallenge, ArenaLeaderboard, ArenaParticipant, ArenaPortfolio, ArenaRun,
    ArenaTrade, CompanyThesis, CreateArenaAgentRequest, CreateArenaChallengeRequest,
    CreateCompanyThesisRequest, RunArenaAgentRequest,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThesisQuery {
    symbol: Option<String>,
    challenge_id: Option<String>,
    limit: Option<i64>,
}

async fn list_agents(State(state): State<Arc<AppState>>) -> ApiResult<Json<Vec<ArenaAgent>>> {
    Ok(Json(state.ai_arena_service.list_agents()?))
}

async fn create_agent(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateArenaAgentRequest>,
) -> ApiResult<Json<ArenaAgent>> {
    Ok(Json(state.ai_arena_service.create_agent(payload).await?))
}

async fn update_agent(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateArenaAgentRequest>,
) -> ApiResult<Json<ArenaAgent>> {
    Ok(Json(
        state.ai_arena_service.update_agent(&id, payload).await?,
    ))
}

async fn delete_agent(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<StatusCode> {
    state.ai_arena_service.delete_agent(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_challenges(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<ArenaChallenge>>> {
    Ok(Json(state.ai_arena_service.list_challenges()?))
}

async fn get_challenge(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<ArenaChallenge>> {
    Ok(Json(state.ai_arena_service.get_challenge(&id)?))
}

async fn create_challenge(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateArenaChallengeRequest>,
) -> ApiResult<Json<ArenaChallenge>> {
    Ok(Json(
        state.ai_arena_service.create_challenge(payload).await?,
    ))
}

async fn join_challenge(
    Path((challenge_id, agent_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<ArenaParticipant>> {
    Ok(Json(
        state
            .ai_arena_service
            .join_challenge(&challenge_id, &agent_id)
            .await?,
    ))
}

async fn list_participants(
    Path(challenge_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<ArenaParticipant>>> {
    Ok(Json(
        state.ai_arena_service.list_participants(&challenge_id)?,
    ))
}

async fn run_agent(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RunArenaAgentRequest>,
) -> ApiResult<Json<ArenaRun>> {
    Ok(Json(state.ai_arena_service.run_agent(payload).await?))
}

async fn run_due_scheduled(State(state): State<Arc<AppState>>) -> ApiResult<Json<Vec<ArenaRun>>> {
    Ok(Json(state.ai_arena_service.run_due_scheduled().await?))
}

async fn list_runs(
    Path(challenge_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<ArenaRun>>> {
    Ok(Json(state.ai_arena_service.list_runs(&challenge_id)?))
}

async fn list_trades(
    Path(challenge_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<ArenaTrade>>> {
    Ok(Json(state.ai_arena_service.list_trades(&challenge_id)?))
}

async fn get_leaderboard(
    Path(challenge_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<ArenaLeaderboard>> {
    Ok(Json(
        state
            .ai_arena_service
            .get_leaderboard(&challenge_id)
            .await?,
    ))
}

async fn settle_challenge(
    Path(challenge_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<ArenaLeaderboard>> {
    Ok(Json(
        state
            .ai_arena_service
            .settle_challenge(&challenge_id)
            .await?,
    ))
}

async fn get_portfolio(
    Path(participant_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<ArenaPortfolio>> {
    Ok(Json(
        state
            .ai_arena_service
            .get_portfolio(&participant_id)
            .await?,
    ))
}

async fn list_theses(
    Query(query): Query<ThesisQuery>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<CompanyThesis>>> {
    let theses = state.ai_arena_service.list_theses(
        query.symbol.as_deref(),
        query.challenge_id.as_deref(),
        query.limit.unwrap_or(50),
    )?;
    Ok(Json(theses))
}

async fn create_thesis(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateCompanyThesisRequest>,
) -> ApiResult<Json<CompanyThesis>> {
    Ok(Json(state.ai_arena_service.create_thesis(payload).await?))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/ai-arena/agents", get(list_agents).post(create_agent))
        .route(
            "/ai-arena/agents/{id}",
            axum::routing::put(update_agent).delete(delete_agent),
        )
        .route(
            "/ai-arena/challenges",
            get(list_challenges).post(create_challenge),
        )
        .route("/ai-arena/challenges/{id}", get(get_challenge))
        .route(
            "/ai-arena/challenges/{challenge_id}/participants",
            get(list_participants),
        )
        .route(
            "/ai-arena/challenges/{challenge_id}/participants/{agent_id}",
            axum::routing::post(join_challenge),
        )
        .route("/ai-arena/challenges/{challenge_id}/runs", get(list_runs))
        .route(
            "/ai-arena/challenges/{challenge_id}/trades",
            get(list_trades),
        )
        .route(
            "/ai-arena/challenges/{challenge_id}/leaderboard",
            get(get_leaderboard),
        )
        .route(
            "/ai-arena/challenges/{challenge_id}/settle",
            axum::routing::post(settle_challenge),
        )
        .route("/ai-arena/runs", axum::routing::post(run_agent))
        .route("/ai-arena/runs/due", axum::routing::post(run_due_scheduled))
        .route(
            "/ai-arena/participants/{participant_id}/portfolio",
            get(get_portfolio),
        )
        .route("/ai-arena/theses", get(list_theses).post(create_thesis))
}
