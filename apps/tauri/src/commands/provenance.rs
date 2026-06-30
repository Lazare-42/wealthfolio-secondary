use std::sync::Arc;

use tauri::State;

use crate::context::ServiceContext;
use wealthfolio_core::provenance::{
    ActivitySource, ChatSourceEmail, NewActivitySource, NewChatSourceEmail,
};

#[tauri::command]
pub async fn record_activity_source(
    source: NewActivitySource,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<ActivitySource, String> {
    state
        .provenance_service()
        .record_source(source)
        .await
        .map_err(|e| format!("Failed to record activity source: {}", e))
}

#[tauri::command]
pub async fn get_activity_sources(
    activity_id: String,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<ActivitySource>, String> {
    state
        .provenance_service()
        .activity_sources(&activity_id)
        .await
        .map_err(|e| format!("Failed to load activity sources: {}", e))
}

#[tauri::command]
pub async fn save_source_email(
    email: NewChatSourceEmail,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<ChatSourceEmail, String> {
    state
        .provenance_service()
        .save_email(email)
        .await
        .map_err(|e| format!("Failed to save source email: {}", e))
}

#[tauri::command]
pub async fn list_source_emails(
    thread_id: Option<String>,
    limit: Option<i64>,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<ChatSourceEmail>, String> {
    let service = state.provenance_service();
    match thread_id {
        Some(thread_id) => service.thread_emails(&thread_id).await,
        None => service.recent_emails(limit.unwrap_or(100)).await,
    }
    .map_err(|e| format!("Failed to list source emails: {}", e))
}
