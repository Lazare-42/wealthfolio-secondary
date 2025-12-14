use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use wealthfolio_core::ai::{csv_mapper, test_connection as test_ai_connection, AIProviderConfig};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct SetApiKeyRequest {
    pub provider: String,
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
    pub model_used: Option<String>,
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

// GET /ai/config - Get current AI provider configuration
#[axum::debug_handler]
async fn get_ai_config(State(state): State<Arc<AppState>>) -> Json<AIProviderConfig> {
    let config_guard = state.ai_config.read().unwrap_or_else(|e| {
        tracing::error!("Failed to read AI config: {}", e);
        panic!("AI config lock poisoned");
    });
    Json(config_guard.clone())
}

// PUT /ai/config - Set AI provider configuration
#[axum::debug_handler]
async fn set_ai_config(
    State(state): State<Arc<AppState>>,
    Json(new_config): Json<AIProviderConfig>,
) -> Json<AIProviderConfig> {
    let mut config_guard = state.ai_config.write().unwrap_or_else(|e| {
        tracing::error!("Failed to write AI config: {}", e);
        panic!("AI config lock poisoned");
    });
    *config_guard = new_config.clone();
    Json(new_config)
}

// POST /ai/api-key - Set API key for provider
#[axum::debug_handler]
async fn set_api_key(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SetApiKeyRequest>,
) -> Json<serde_json::Value> {
    let service_id = format!("ai_{}", request.provider.to_lowercase());

    match state.secret_store.set_secret(&service_id, &request.key) {
        Ok(_) => Json(serde_json::json!({ "success": true })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": format!("Failed to store API key: {}", e)
        })),
    }
}

// GET /ai/api-key/:provider - Check if API key exists
#[axum::debug_handler]
async fn has_api_key(
    State(state): State<Arc<AppState>>,
    Path(provider): Path<String>,
) -> Json<bool> {
    let service_id = format!("ai_{}", provider.to_lowercase());

    match state.secret_store.get_secret(&service_id) {
        Ok(Some(_)) => Json(true),
        _ => Json(false),
    }
}

// POST /ai/test-connection - Test AI connection
#[axum::debug_handler]
async fn test_connection(
    State(state): State<Arc<AppState>>,
) -> Json<TestConnectionResponse> {
    // Extract client and model
    let (client, model): (_, String) = {
        let config_guard = match state.ai_config.read() {
            Ok(guard) => guard,
            Err(e) => {
                return Json(TestConnectionResponse {
                    success: false,
                    message: format!("Failed to read AI config: {}", e),
                    model_used: None,
                });
            }
        };

        match config_guard.create_client(&state.secret_store) {
            Ok(Some(c)) => c,
            Ok(None) => {
                return Json(TestConnectionResponse {
                    success: false,
                    message: "AI provider not configured".to_string(),
                    model_used: None,
                });
            }
            Err(e) => {
                return Json(TestConnectionResponse {
                    success: false,
                    message: format!("Failed to create AI client: {}", e),
                    model_used: None,
                });
            }
        }
    };

    // Make a real API call to test the connection
    match test_ai_connection(&client, &model).await {
        Ok(response) => Json(TestConnectionResponse {
            success: true,
            message: format!("Connection successful. Response: {}", response),
            model_used: Some(model),
        }),
        Err(e) => Json(TestConnectionResponse {
            success: false,
            message: e,
            model_used: Some(model),
        }),
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/ai/suggest-mapping", post(suggest_csv_mapping))
        .route("/ai/config", get(get_ai_config))
        .route("/ai/config", put(set_ai_config))
        .route("/ai/api-key", post(set_api_key))
        .route("/ai/api-key/{provider}", get(has_api_key))
        .route("/ai/test-connection", post(test_connection))
}
