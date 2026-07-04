//! AI-generated arena challenge specs from a free-text theme.
//!
//! Given a theme like "European defense primes", asks the AI Assistant's
//! configured provider/model for a challenge name, description, and ticker
//! universe, then validates each ticker against the market-data symbol
//! search (same lookup the chat's `search_market_symbols` tool uses).

use std::collections::HashSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use wealthfolio_core::ai_arena::{service::parse_model_json, ArenaDecisionRequest};

use crate::arena_runner::AiArenaLlmRunner;
use crate::env::AiEnvironment;
use crate::error::AiError;
use crate::providers::ProviderService;

const SYSTEM_PROMPT: &str = "You design paper-trading stock challenges for an investment app. \
Given an investment theme, respond with STRICT JSON of this exact shape: \
{\"name\": \"...\", \"description\": \"...\", \"universe\": [\"TICKER\", ...]}. \
Rules: \
- name: a short, punchy challenge name (max 6 words). \
- description: 2-4 sentences expanding the theme into an investable thesis, \
stating what is in and out of scope. \
- universe: 10-30 liquid, exchange-listed stock or equity-ETF tickers that fit the theme, \
as plain ticker symbols (e.g. \"AAPL\", or \"RHM.DE\" for non-US listings).";

/// A challenge spec generated from a theme, with symbol-validation results
/// and attribution of the provider/model that produced it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedArenaChallengeSpec {
    pub name: String,
    pub description: String,
    /// Tickers that resolved via the market-data symbol search (deduped).
    pub universe: Vec<String>,
    /// Tickers the model proposed but that did not resolve.
    pub dropped: Vec<String>,
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Deserialize)]
struct RawChallengeSpec {
    name: String,
    description: String,
    #[serde(default)]
    universe: Vec<String>,
}

/// Generate an arena challenge spec from a free-text theme using the AI
/// Assistant's configured default provider and model.
pub async fn generate_challenge_spec<E: AiEnvironment>(
    env: Arc<E>,
    theme: &str,
) -> Result<GeneratedArenaChallengeSpec, AiError> {
    let theme = theme.trim();
    if theme.is_empty() {
        return Err(AiError::InvalidInput("theme must not be empty".to_string()));
    }

    // Resolve provider + model exactly like the AI Assistant chat does.
    let provider_service = ProviderService::new(env.clone());
    let settings = provider_service.get_settings()?;
    let provider_id = settings.provider_id;
    let model_id = settings.model;

    // Reuse the arena LLM runner: shared provider clients, resolved tuning
    // (temperature only when provided), and strict-JSON system suffix.
    let runner = AiArenaLlmRunner::new(env.clone());
    let raw = runner
        .complete(ArenaDecisionRequest {
            provider_id: provider_id.clone(),
            model_id: model_id.clone(),
            system_prompt: SYSTEM_PROMPT.to_string(),
            prompt: format!("Theme: {theme}"),
        })
        .await?;

    let value = parse_model_json(&raw)?;
    let spec: RawChallengeSpec = serde_json::from_value(value)
        .map_err(|e| AiError::Provider(format!("Model returned an unexpected JSON shape: {e}")))?;

    // Validate the universe against the market-data symbol search.
    let quote_service = env.quote_service();
    let mut seen: HashSet<String> = HashSet::new();
    let mut universe = Vec::new();
    let mut dropped = Vec::new();
    for raw_ticker in spec.universe {
        let ticker = raw_ticker.trim().to_uppercase();
        if ticker.is_empty() || !seen.insert(ticker.clone()) {
            continue;
        }
        let resolved = quote_service
            .search_symbol_with_currency(&ticker, None)
            .await
            .map(|results| {
                results
                    .iter()
                    .any(|result| result.symbol.eq_ignore_ascii_case(&ticker))
            })
            .unwrap_or(false);
        if resolved {
            universe.push(ticker);
        } else {
            dropped.push(ticker);
        }
    }

    Ok(GeneratedArenaChallengeSpec {
        name: spec.name.trim().to_string(),
        description: spec.description.trim().to_string(),
        universe,
        dropped,
        provider_id,
        model_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fenced_spec_json() {
        let raw = "```json\n{\"name\": \"Euro Defense Primes\", \"description\": \"A thesis.\", \"universe\": [\"RHM.DE\", \"BA.L\"]}\n```";
        let value = parse_model_json(raw).expect("json");
        let spec: RawChallengeSpec = serde_json::from_value(value).expect("spec");
        assert_eq!(spec.name, "Euro Defense Primes");
        assert_eq!(spec.universe, vec!["RHM.DE", "BA.L"]);
    }
}
