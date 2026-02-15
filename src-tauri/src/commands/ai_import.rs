use crate::ai::{csv_mapper, AIProviderConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

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

#[tauri::command]
pub async fn suggest_csv_column_mapping(
    config: State<'_, std::sync::Arc<tokio::sync::RwLock<AIProviderConfig>>>,
    request: SuggestMappingRequest,
) -> Result<SuggestMappingResponse, String> {
    let config_guard = config.read().await;

    // Check if AI is configured
    let provider = match config_guard.create_provider() {
        Some(p) => p,
        None => {
            return Ok(SuggestMappingResponse {
                success: false,
                suggestions: None,
                error: Some("AI provider not configured. Please configure OpenRouter or Ollama in settings.".to_string()),
            });
        }
    };

    // Call the AI to get suggestions
    match csv_mapper::suggest_column_mappings(
        provider.as_ref(),
        request.headers,
        request.sample_rows,
    )
    .await
    {
        Ok(suggestions) => Ok(SuggestMappingResponse {
            success: true,
            suggestions: Some(suggestions),
            error: None,
        }),
        Err(e) => Ok(SuggestMappingResponse {
            success: false,
            suggestions: None,
            error: Some(format!("AI request failed: {}", e)),
        }),
    }
}

#[tauri::command]
pub async fn get_ai_provider_config(
    config: State<'_, std::sync::Arc<tokio::sync::RwLock<AIProviderConfig>>>,
) -> Result<AIProviderConfig, String> {
    let config_guard = config.read().await;
    Ok(config_guard.clone())
}

#[tauri::command]
pub async fn set_ai_provider_config(
    config: State<'_, std::sync::Arc<tokio::sync::RwLock<AIProviderConfig>>>,
    new_config: AIProviderConfig,
) -> Result<(), String> {
    let mut config_guard = config.write().await;
    *config_guard = new_config;
    Ok(())
}
