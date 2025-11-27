use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use wealthfolio_core::ai::csv_mapper;

use crate::main_lib::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct SuggestMappingRequest {
    pub headers: Vec<String>,
    pub sample_rows: Vec<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuggestMappingResponse {
    pub success: bool,
    pub suggestions: Option<csv_mapper::MappingSuggestions>,
    pub error: Option<String>,
}

#[axum::debug_handler]
async fn suggest_csv_mapping(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SuggestMappingRequest>,
) -> Json<SuggestMappingResponse> {
    // Extract client and model, then drop the guard before awaiting
    let (client, model): (_, String) = {
        let config_guard = match state.ai_config.read() {
            Ok(guard) => guard,
            Err(e) => {
                return Json(SuggestMappingResponse {
                    success: false,
                    suggestions: None,
                    error: Some(format!("Failed to read AI config: {}", e)),
                });
            }
        };

        // Use FileSecretStore (from AppState) for web mode
        match config_guard.create_client(&state.secret_store) {
            Ok(Some(c)) => c,
            Ok(None) => {
                return Json(SuggestMappingResponse {
                    success: false,
                    suggestions: None,
                    error: Some(
                        "AI provider not configured. Please configure OpenRouter or Ollama in settings."
                            .to_string(),
                    ),
                });
            }
            Err(e) => {
                return Json(SuggestMappingResponse {
                    success: false,
                    suggestions: None,
                    error: Some(format!("Failed to create AI client: {}", e)),
                });
            }
        }
        // config_guard is dropped here
    };

    // Call core business logic (same implementation as Tauri)
    match csv_mapper::suggest_column_mappings(&client, &model, request.headers, request.sample_rows)
        .await
    {
        Ok(suggestions) => Json(SuggestMappingResponse {
            success: true,
            suggestions: Some(suggestions),
            error: None,
        }),
        Err(e) => Json(SuggestMappingResponse {
            success: false,
            suggestions: None,
            error: Some(format!("AI request failed: {}", e)),
        }),
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/ai/suggest-mapping", post(suggest_csv_mapping))
}
