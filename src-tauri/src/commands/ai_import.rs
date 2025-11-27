use crate::ai::{csv_mapper, AIProviderConfig};
use crate::secret_store::KeyringSecretStore;
use ai_lib::{ChatCompletionRequest, Content, Message, Role};
use base64;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;
use wealthfolio_core::secrets::SecretStore;

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
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
    pub model_used: Option<String>,
}

#[tauri::command]
pub async fn suggest_csv_column_mapping(
    config: State<'_, std::sync::Arc<tokio::sync::RwLock<AIProviderConfig>>>,
    request: SuggestMappingRequest,
) -> Result<SuggestMappingResponse, String> {
    let config_guard = config.read().await;

    // Check if AI is configured
    let secret_store: std::sync::Arc<dyn SecretStore> = std::sync::Arc::new(KeyringSecretStore::default());
    let (client, model) = match config_guard.create_client(&secret_store) {
        Ok(Some(c)) => c,
        Ok(None) => {
            return Ok(SuggestMappingResponse {
                success: false,
                suggestions: None,
                error: Some("AI provider not configured. Please configure OpenRouter or Ollama in settings.".to_string()),
            });
        }
        Err(e) => {
            return Ok(SuggestMappingResponse {
                success: false,
                suggestions: None,
                error: Some(format!("Failed to create AI client: {}", e)),
            });
        }
    };

    // Call the AI to get suggestions
    match csv_mapper::suggest_column_mappings(&client, &model, request.headers, request.sample_rows)
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

#[tauri::command]
pub async fn set_ai_api_key(provider: String, api_key: String) -> Result<(), String> {
    let service_id = format!("ai_{}", provider);
    KeyringSecretStore::default()
        .set_secret(&service_id, &api_key)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn has_ai_api_key(provider: String) -> Result<bool, String> {
    let service_id = format!("ai_{}", provider);
    let secret = KeyringSecretStore::default()
        .get_secret(&service_id)
        .map_err(|e| e.to_string())?;
    Ok(secret.is_some())
}

#[tauri::command]
pub async fn test_ai_connection(
    config: State<'_, std::sync::Arc<tokio::sync::RwLock<AIProviderConfig>>>,
) -> Result<TestConnectionResponse, String> {
    let config_guard = config.read().await;

    let secret_store: std::sync::Arc<dyn SecretStore> = std::sync::Arc::new(KeyringSecretStore::default());
    let (client, model) = match config_guard.create_client(&secret_store) {
        Ok(Some(c)) => c,
        Ok(None) => {
            return Ok(TestConnectionResponse {
                success: false,
                message: "AI provider not configured or API key missing.".to_string(),
                model_used: None,
            });
        }
        Err(e) => {
            return Ok(TestConnectionResponse {
                success: false,
                message: format!("Failed to create AI client: {}", e),
                model_used: None,
            });
        }
    };

    let request = ChatCompletionRequest::new(
        model.clone(),
        vec![Message {
            role: Role::User,
            content: Content::Text("Hello, are you there? Reply with 'Yes'".to_string()),
            function_call: None,
        }],
    );

    match client.chat_completion(request).await {
        Ok(_) => Ok(TestConnectionResponse {
            success: true,
            message: "Connection successful!".to_string(),
            model_used: Some(format!("{:?} ({})", client.current_provider(), model)),
        }),
        Err(e) => Ok(TestConnectionResponse {
            success: false,
            message: format!("Connection failed: {}", e),
            model_used: None,
        }),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParseDocumentRequest {
    pub file_bytes: Vec<u8>,
    pub file_type: String,     // "application/pdf", "image/png", etc.
    pub document_type: String, // "bank_statement", "invoice", etc.
}

#[tauri::command]
pub async fn parse_financial_document(
    config: State<'_, std::sync::Arc<tokio::sync::RwLock<AIProviderConfig>>>,
    request: ParseDocumentRequest,
) -> Result<Vec<wealthfolio_core::activities::ActivityImport>, String> {
    let config_guard = config.read().await;

    // Check if AI is configured
    let secret_store: std::sync::Arc<dyn SecretStore> = std::sync::Arc::new(KeyringSecretStore::default());
    let (client, model) = match config_guard.create_client(&secret_store) {
        Ok(Some(c)) => c,
        Ok(None) => return Err("AI provider not configured or API key missing.".to_string()),
        Err(e) => return Err(format!("Failed to create AI client: {}", e)),
    };

    // Prepare messages (Multimodal support)
    let messages = if request.file_type.starts_with("image/") {
        // Encode bytes to base64 for the image URL
        use base64::{engine::general_purpose, Engine as _};
        let base64_image = general_purpose::STANDARD.encode(&request.file_bytes);
        let data_url = format!("data:{};base64,{}", request.file_type, base64_image);

        vec![
            Message {
                role: Role::User,
                content: Content::new_text(
                    "Parse this financial document and extract transactions.".to_string(),
                ),
                function_call: None,
            },
            Message {
                role: Role::User,
                content: Content::new_image(Some(data_url), None, None),
                function_call: None,
            },
        ]
    } else {
        // TODO: For PDF, we'd need to extract text first or use a model that supports PDF bytes directly (rare).
        // For now, we'll assume text extraction happens elsewhere or fail for PDFs.
        return Err(
            "PDF parsing requires text extraction first. Only images are supported in this mode."
                .to_string(),
        );
    };

    let req = ChatCompletionRequest::new(model, messages);

    // Call AI
    let _response = client
        .chat_completion(req)
        .await
        .map_err(|e| e.to_string())?;

    // TODO: Parse the JSON response into ActivityImport structs.
    // For now, we return empty to signify the connection worked but parsing logic is pending.
    Ok(vec![])
}
