pub mod providers;
pub mod csv_mapper;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Abstract trait for AI providers (OpenRouter, Ollama, etc.)
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Send a prompt to the AI and get a response
    async fn complete(&self, prompt: String) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Get the provider name for logging/debugging
    fn provider_name(&self) -> &str;
}

/// Configuration for AI provider selection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AIProviderConfig {
    #[serde(rename = "openrouter")]
    OpenRouter {
        api_key: String,
        model: Option<String>, // Default: "openai/gpt-4o-mini"
    },
    #[serde(rename = "ollama")]
    Ollama {
        base_url: Option<String>, // Default: "http://localhost:11434"
        model: Option<String>,    // Default: "llama3.2"
    },
    #[serde(rename = "disabled")]
    Disabled,
}

impl Default for AIProviderConfig {
    fn default() -> Self {
        Self::Disabled
    }
}

impl AIProviderConfig {
    /// Create a provider instance from this configuration
    pub fn create_provider(&self) -> Option<Box<dyn AIProvider>> {
        match self {
            AIProviderConfig::OpenRouter { api_key, model } => {
                Some(Box::new(providers::OpenRouterProvider::new(
                    api_key.clone(),
                    model.clone().unwrap_or_else(|| "openai/gpt-4o-mini".to_string()),
                )))
            }
            AIProviderConfig::Ollama { base_url, model } => {
                Some(Box::new(providers::OllamaProvider::new(
                    base_url.clone().unwrap_or_else(|| "http://localhost:11434".to_string()),
                    model.clone().unwrap_or_else(|| "llama3.2".to_string()),
                )))
            }
            AIProviderConfig::Disabled => None,
        }
    }
}
