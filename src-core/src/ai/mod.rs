pub mod csv_mapper;

use ai_lib::{AiClient, ChatCompletionRequest, ConnectionOptions, Content, Message, Provider, Role};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::secrets::SecretStore;

/// Configuration for AI provider selection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AIProviderConfig {
    #[serde(rename = "openrouter")]
    OpenRouter {
        model: Option<String>, // Default: "openai/gpt-4o-mini"
    },
    #[serde(rename = "ollama")]
    Ollama {
        base_url: Option<String>, // Default: "http://localhost:11434"
        model: Option<String>,    // Default: "llama3.2"
    },
    #[serde(rename = "openai")]
    OpenAI {
        model: Option<String>, // Default: "gpt-4o-mini"
    },
    #[serde(rename = "anthropic")]
    Anthropic {
        model: Option<String>, // Default: "claude-3-5-sonnet-20241022"
    },
    #[serde(rename = "mistral")]
    Mistral {
        model: Option<String>, // Default: "mistral-small-latest"
    },
    #[serde(rename = "disabled")]
    Disabled,
}

impl Default for AIProviderConfig {
    fn default() -> Self {
        match std::env::var("WF_AI_PROVIDER").unwrap_or_default().as_str() {
            "openrouter" => Self::OpenRouter {
                model: std::env::var("WF_OPENROUTER_MODEL").ok(),
            },
            "ollama" => Self::Ollama {
                base_url: std::env::var("WF_OLLAMA_BASE_URL").ok(),
                model: std::env::var("WF_OLLAMA_MODEL").ok(),
            },
            "openai" => Self::OpenAI {
                model: std::env::var("WF_OPENAI_MODEL").ok(),
            },
            "anthropic" => Self::Anthropic {
                model: std::env::var("WF_ANTHROPIC_MODEL").ok(),
            },
            "mistral" => Self::Mistral {
                model: std::env::var("WF_MISTRAL_MODEL").ok(),
            },
            _ => Self::Disabled,
        }
    }
}

impl AIProviderConfig {
    /// Create an ai-lib client from this configuration
    pub fn create_client(
        &self,
        secret_store: &Arc<dyn SecretStore>,
    ) -> Result<Option<(AiClient, String)>, String> {
        match self {
            AIProviderConfig::OpenRouter { model } => {
                let api_key = std::env::var("WF_OPENROUTER_API_KEY").ok().or_else(|| {
                    secret_store.get_secret("ai_openrouter").ok().flatten()
                });

                let api_key = match api_key {
                    Some(k) => k,
                    None => {
                        return Err("OpenRouter API key not found in env or secret store".to_string())
                    }
                };

                let model_name = model
                    .clone()
                    .or_else(|| std::env::var("WF_OPENROUTER_MODEL").ok())
                    .unwrap_or_else(|| "openai/gpt-4o-mini".to_string());

                let client = AiClient::with_options(
                    Provider::OpenRouter,
                    ConnectionOptions {
                        api_key: Some(api_key),
                        ..Default::default()
                    },
                )
                .map_err(|e| e.to_string())?;

                Ok(Some((client, model_name)))
            }
            AIProviderConfig::Ollama { base_url, model } => {
                let base_url = base_url
                    .clone()
                    .or_else(|| std::env::var("WF_OLLAMA_BASE_URL").ok())
                    .unwrap_or_else(|| "http://localhost:11434".to_string());

                let model_name = model
                    .clone()
                    .or_else(|| std::env::var("WF_OLLAMA_MODEL").ok())
                    .unwrap_or_else(|| "llama3.2".to_string());

                let client = AiClient::with_options(
                    Provider::Ollama,
                    ConnectionOptions {
                        base_url: Some(base_url),
                        ..Default::default()
                    },
                )
                .map_err(|e| e.to_string())?;

                Ok(Some((client, model_name)))
            }
            AIProviderConfig::OpenAI { model } => {
                let api_key = std::env::var("WF_OPENAI_API_KEY").ok().or_else(|| {
                    secret_store.get_secret("ai_openai").ok().flatten()
                });

                let api_key = match api_key {
                    Some(k) => k,
                    None => return Err("OpenAI API key not found in env or secret store".to_string()),
                };

                let model_name = model
                    .clone()
                    .or_else(|| std::env::var("WF_OPENAI_MODEL").ok())
                    .unwrap_or_else(|| "gpt-4o-mini".to_string());

                let client = AiClient::with_options(
                    Provider::OpenAI,
                    ConnectionOptions {
                        api_key: Some(api_key),
                        ..Default::default()
                    },
                )
                .map_err(|e| e.to_string())?;

                Ok(Some((client, model_name)))
            }
            AIProviderConfig::Anthropic { model } => {
                let api_key = std::env::var("WF_ANTHROPIC_API_KEY").ok().or_else(|| {
                    secret_store.get_secret("ai_anthropic").ok().flatten()
                });

                let api_key = match api_key {
                    Some(k) => k,
                    None => return Err("Anthropic API key not found in env or secret store".to_string()),
                };

                let model_name = model
                    .clone()
                    .or_else(|| std::env::var("WF_ANTHROPIC_MODEL").ok())
                    .unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string());

                let client = AiClient::with_options(
                    Provider::Anthropic,
                    ConnectionOptions {
                        api_key: Some(api_key),
                        ..Default::default()
                    },
                )
                .map_err(|e| e.to_string())?;

                Ok(Some((client, model_name)))
            }
            AIProviderConfig::Mistral { model } => {
                let api_key = std::env::var("WF_MISTRAL_API_KEY").ok().or_else(|| {
                    secret_store.get_secret("ai_mistral").ok().flatten()
                });

                let api_key = match api_key {
                    Some(k) => k,
                    None => return Err("Mistral API key not found in env or secret store".to_string()),
                };

                let model_name = model
                    .clone()
                    .or_else(|| std::env::var("WF_MISTRAL_MODEL").ok())
                    .unwrap_or_else(|| "mistral-small-latest".to_string());

                let client = AiClient::with_options(
                    Provider::Mistral,
                    ConnectionOptions {
                        api_key: Some(api_key),
                        ..Default::default()
                    },
                )
                .map_err(|e| e.to_string())?;

                Ok(Some((client, model_name)))
            }
            AIProviderConfig::Disabled => Ok(None),
        }
    }
}

/// Test AI connection by making a simple API call
pub async fn test_connection(client: &AiClient, model: &str) -> Result<String, String> {
    let request = ChatCompletionRequest::new(
        model.to_string(),
        vec![Message {
            role: Role::User,
            content: Content::Text("Hello, respond with 'OK'".to_string()),
            function_call: None,
        }],
    );

    match client.chat_completion(request).await {
        Ok(response) => {
            let content = response
                .choices
                .first()
                .map(|c| c.message.content.as_text())
                .unwrap_or_else(|| "Empty response".to_string());
            Ok(content)
        }
        Err(e) => Err(format!("Connection test failed: {}", e)),
    }
}
