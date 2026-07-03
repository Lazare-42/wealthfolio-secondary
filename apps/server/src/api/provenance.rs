use std::sync::Arc;

use crate::{error::ApiResult, main_lib::AppState};
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use wealthfolio_core::provenance::{
    ActivitySource, ChatSourceEmail, NewActivitySource, NewChatSourceEmail,
};

async fn record_source(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NewActivitySource>,
) -> ApiResult<Json<ActivitySource>> {
    let created = state.provenance_service.record_source(payload).await?;
    Ok(Json(created))
}

async fn activity_sources(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<ActivitySource>>> {
    let sources = state.provenance_service.activity_sources(&id).await?;
    Ok(Json(sources))
}

async fn save_source_email(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NewChatSourceEmail>,
) -> ApiResult<Json<ChatSourceEmail>> {
    let saved = state.provenance_service.save_email(payload).await?;
    Ok(Json(saved))
}

#[derive(Debug, Deserialize)]
struct EmailQuery {
    #[serde(default)]
    thread_id: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
}

async fn list_source_emails(
    Query(q): Query<EmailQuery>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<ChatSourceEmail>>> {
    let emails = match q.thread_id {
        Some(thread_id) => state.provenance_service.thread_emails(&thread_id).await?,
        None => {
            state
                .provenance_service
                .recent_emails(q.limit.unwrap_or(100))
                .await?
        }
    };
    Ok(Json(emails))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/provenance/sources", post(record_source))
        .route("/activities/{id}/sources", get(activity_sources))
        .route(
            "/provenance/source-emails",
            post(save_source_email).get(list_source_emails),
        )
}
