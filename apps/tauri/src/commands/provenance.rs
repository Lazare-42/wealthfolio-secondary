use std::sync::Arc;

use tauri::State;

use crate::context::ServiceContext;
use wealthfolio_core::provenance::ChatSourceEmail;

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
