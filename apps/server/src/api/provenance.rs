use std::sync::Arc;

use crate::{error::ApiResult, main_lib::AppState};
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use wealthfolio_core::provenance::ChatSourceEmail;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    Router::new().route("/provenance/source-emails", get(list_source_emails))
}
