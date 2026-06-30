use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Client as HttpClient;
use rig::{
    client::{CompletionClient, Nothing},
    completion::Prompt,
    providers::{anthropic, gemini, groq, ollama, openai, openrouter},
};
use wealthfolio_core::ai_arena::{AiArenaDecisionRunner, ArenaDecisionRequest};
use wealthfolio_core::errors::{Error as CoreError, Result as CoreResult};

use crate::env::AiEnvironment;
use crate::error::AiError;
use crate::provider_urls::ensure_openai_v1_base_url;
use crate::providers::ProviderService;

pub struct AiArenaLlmRunner<E: AiEnvironment> {
    env: Arc<E>,
}

impl<E: AiEnvironment> AiArenaLlmRunner<E> {
    pub fn new(env: Arc<E>) -> Self {
        Self { env }
    }

    async fn complete(&self, request: ArenaDecisionRequest) -> Result<String, AiError> {
        let provider_service = ProviderService::new(self.env.clone());
        let api_key = provider_service.get_api_key(&request.provider_id)?;
        let provider_url = provider_service.get_provider_url(&request.provider_id);
        let prompt = request.prompt;
        let system_prompt = format!(
            "{}\n\nReturn only valid JSON. No markdown. No prose outside JSON.",
            request.system_prompt.trim()
        );

        match request.provider_id.as_str() {
            "anthropic" => {
                let key = api_key.ok_or_else(|| AiError::MissingApiKey(request.provider_id))?;
                let mut builder = anthropic::Client::<HttpClient>::builder().api_key(&key);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&url);
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(&request.model_id)
                    .preamble(&system_prompt)
                    .max_tokens(4096)
                    .temperature(0.2)
                    .build()
                    .prompt(&prompt)
                    .await
                    .map_err(|e| AiError::Provider(e.to_string()))
            }
            "gemini" | "google" => {
                let key = api_key.ok_or_else(|| AiError::MissingApiKey(request.provider_id))?;
                let mut builder = gemini::Client::<HttpClient>::builder().api_key(&key);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&url);
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(&request.model_id)
                    .preamble(&system_prompt)
                    .max_tokens(4096)
                    .temperature(0.2)
                    .build()
                    .prompt(&prompt)
                    .await
                    .map_err(|e| AiError::Provider(e.to_string()))
            }
            "groq" => {
                let key = api_key.ok_or_else(|| AiError::MissingApiKey(request.provider_id))?;
                let mut builder = groq::Client::<HttpClient>::builder().api_key(&key);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&ensure_openai_v1_base_url(&url));
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(&request.model_id)
                    .preamble(&system_prompt)
                    .max_tokens(4096)
                    .temperature(0.2)
                    .build()
                    .prompt(&prompt)
                    .await
                    .map_err(|e| AiError::Provider(e.to_string()))
            }
            "ollama" => {
                let mut builder = ollama::Client::<HttpClient>::builder().api_key(Nothing);
                if let Some(url) = provider_url {
                    let normalized = url.trim_end_matches('/').trim_end_matches("/v1");
                    builder = builder.base_url(normalized);
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(&request.model_id)
                    .preamble(&system_prompt)
                    .max_tokens(4096)
                    .temperature(0.2)
                    .build()
                    .prompt(&prompt)
                    .await
                    .map_err(|e| AiError::Provider(e.to_string()))
            }
            "openrouter" => {
                let key = api_key.ok_or_else(|| AiError::MissingApiKey(request.provider_id))?;
                let mut builder = openrouter::Client::<HttpClient>::builder().api_key(&key);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&ensure_openai_v1_base_url(&url));
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(&request.model_id)
                    .preamble(&system_prompt)
                    .max_tokens(4096)
                    .temperature(0.2)
                    .build()
                    .prompt(&prompt)
                    .await
                    .map_err(|e| AiError::Provider(e.to_string()))
            }
            _ => {
                let key = api_key.ok_or_else(|| AiError::MissingApiKey(request.provider_id))?;
                let mut builder = openai::CompletionsClient::<HttpClient>::builder().api_key(&key);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&ensure_openai_v1_base_url(&url));
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(&request.model_id)
                    .preamble(&system_prompt)
                    .max_tokens(4096)
                    .temperature(0.2)
                    .build()
                    .prompt(&prompt)
                    .await
                    .map_err(|e| AiError::Provider(e.to_string()))
            }
        }
    }
}

#[async_trait]
impl<E: AiEnvironment + 'static> AiArenaDecisionRunner for AiArenaLlmRunner<E> {
    async fn run_decision(&self, request: ArenaDecisionRequest) -> CoreResult<String> {
        self.complete(request)
            .await
            .map_err(|e| CoreError::Unexpected(e.to_string()))
    }
}
