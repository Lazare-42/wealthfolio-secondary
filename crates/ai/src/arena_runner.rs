use std::sync::Arc;

use async_trait::async_trait;
use rig::{client::CompletionClient, completion::Prompt, tool::ToolDyn};
use wealthfolio_core::ai_arena::{AiArenaDecisionRunner, ArenaDecisionRequest};
use wealthfolio_core::errors::{Error as CoreError, Result as CoreResult};

use crate::chat::provider_clients::{
    create_anthropic_client, create_gemini_client, create_groq_client, create_ollama_client,
    create_openai_client, create_openrouter_client,
};
use crate::env::AiEnvironment;
use crate::error::AiError;
use crate::providers::ProviderService;

const ARENA_MAX_TOKENS: u64 = 4096;

pub struct AiArenaLlmRunner<E: AiEnvironment> {
    env: Arc<E>,
}

impl<E: AiEnvironment> AiArenaLlmRunner<E> {
    pub fn new(env: Arc<E>) -> Self {
        Self { env }
    }

    pub(crate) async fn complete(&self, request: ArenaDecisionRequest) -> Result<String, AiError> {
        self.complete_with_tools(request, Vec::new(), 0).await
    }

    /// Like [`Self::complete`], but attaches rig tools and allows up to
    /// `max_turns` model-↔-tool round trips before the final answer. With an
    /// empty tool list and `max_turns == 0` this is exactly `complete`.
    pub(crate) async fn complete_with_tools(
        &self,
        request: ArenaDecisionRequest,
        tools: Vec<Box<dyn ToolDyn>>,
        max_turns: usize,
    ) -> Result<String, AiError> {
        let provider_service = ProviderService::new(self.env.clone());
        let api_key = provider_service.get_api_key(&request.provider_id)?;
        let provider_url = provider_service.get_provider_url(&request.provider_id);
        // Resolve tuning through the same catalog+overrides engine the chat path
        // uses (get_resolved_tuning). Temperature is applied only when the
        // resolved tuning provides it — the Anthropic catalog omits it, so newer
        // Claude models (e.g. claude-opus-4-7, which 400s on `temperature`) never
        // receive it. The arena stores the alias "gemini" while the catalog key
        // is "google", so normalize before the lookup. Provider ids absent from
        // the catalog entirely (custom OpenAI-compatible endpoints) fall back to
        // the arena's historical deterministic 0.2 — except "anthropic".
        let tuning_provider_id = if request.provider_id == "gemini" {
            "google"
        } else {
            request.provider_id.as_str()
        };
        let tuning = provider_service.get_resolved_tuning(tuning_provider_id);
        let temperature = tuning.temperature.or({
            if tuning_provider_id == "anthropic"
                || provider_service.is_catalog_provider(tuning_provider_id)
            {
                None
            } else {
                Some(0.2)
            }
        });
        let prompt = request.prompt;
        let system_prompt = format!(
            "{}\n\nReturn only valid JSON. No markdown. No prose outside JSON.",
            request.system_prompt.trim()
        );

        let provider_id = request.provider_id.as_str();
        match provider_id {
            "anthropic" => {
                let client = create_anthropic_client(api_key, provider_id, provider_url)?;
                prompt_once(
                    client,
                    &request.model_id,
                    &system_prompt,
                    &prompt,
                    temperature,
                    tools,
                    max_turns,
                )
                .await
            }
            "gemini" | "google" => {
                let client = create_gemini_client(api_key, provider_id, provider_url)?;
                prompt_once(
                    client,
                    &request.model_id,
                    &system_prompt,
                    &prompt,
                    temperature,
                    tools,
                    max_turns,
                )
                .await
            }
            "groq" => {
                let client = create_groq_client(api_key, provider_id, provider_url)?;
                prompt_once(
                    client,
                    &request.model_id,
                    &system_prompt,
                    &prompt,
                    temperature,
                    tools,
                    max_turns,
                )
                .await
            }
            "ollama" => {
                let client = create_ollama_client(provider_url)?;
                prompt_once(
                    client,
                    &request.model_id,
                    &system_prompt,
                    &prompt,
                    temperature,
                    tools,
                    max_turns,
                )
                .await
            }
            "openrouter" => {
                let client = create_openrouter_client(api_key, provider_id, provider_url)?;
                prompt_once(
                    client,
                    &request.model_id,
                    &system_prompt,
                    &prompt,
                    temperature,
                    tools,
                    max_turns,
                )
                .await
            }
            _ => {
                let client = create_openai_client(api_key, provider_id, provider_url)?;
                prompt_once(
                    client,
                    &request.model_id,
                    &system_prompt,
                    &prompt,
                    temperature,
                    tools,
                    max_turns,
                )
                .await
            }
        }
    }
}

/// Run a single non-streaming prompt against any provider client, applying
/// temperature only when the resolved provider tuning supplies one. With a
/// non-empty `tools` list, the rig agent may take up to `max_turns` tool
/// round trips before its final text answer (0 disables multi-turn — the
/// historical single-shot behavior).
#[allow(clippy::too_many_arguments)]
async fn prompt_once<C: CompletionClient>(
    client: C,
    model_id: &str,
    system_prompt: &str,
    prompt: &str,
    temperature: Option<f64>,
    tools: Vec<Box<dyn ToolDyn>>,
    max_turns: usize,
) -> Result<String, AiError> {
    let builder = client
        .agent(model_id)
        .preamble(system_prompt)
        .max_tokens(ARENA_MAX_TOKENS);
    if tools.is_empty() {
        let mut builder = builder;
        if let Some(t) = temperature {
            builder = builder.temperature(t);
        }
        builder
            .build()
            .prompt(prompt)
            .max_turns(max_turns)
            .await
            .map_err(|e| AiError::Provider(e.to_string()))
    } else {
        let mut builder = builder.tools(tools);
        if let Some(t) = temperature {
            builder = builder.temperature(t);
        }
        builder
            .build()
            .prompt(prompt)
            .max_turns(max_turns)
            .await
            .map_err(|e| AiError::Provider(e.to_string()))
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
