//! AI-generated arena challenge specs from a free-text theme or a user draft.
//!
//! Given a theme like "European defense primes", asks the AI Assistant's
//! configured provider/model for a challenge name, description, and ticker
//! universe, then validates each ticker against the market-data symbol
//! search (same lookup the chat's `search_market_symbols` tool uses).
//! When a draft challenge is provided instead (or as well), the model
//! enhances the draft: it keeps the user's intent and tickers, sharpens the
//! description, and adds complementary tickers.

use std::collections::HashSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use wealthfolio_core::ai_arena::{service::parse_model_json, ArenaDecisionRequest};

use crate::arena_runner::AiArenaLlmRunner;
use crate::env::AiEnvironment;
use crate::error::AiError;
use crate::providers::ProviderService;

const GENERATE_SYSTEM_PROMPT: &str =
    "You design paper-trading stock challenges for an investment app. \
Given an investment theme, respond with STRICT JSON of this exact shape: \
{\"name\": \"...\", \"description\": \"...\", \"universe\": [\"TICKER\", ...]}. \
Rules: \
- name: a short, punchy challenge name (max 6 words). \
- description: 2-4 sentences expanding the theme into an investable thesis, \
stating what is in and out of scope. \
- universe: 10-30 liquid, exchange-listed stock or equity-ETF tickers that fit the theme, \
as plain ticker symbols (e.g. \"AAPL\", or \"RHM.DE\" for non-US listings).";

const ENHANCE_SYSTEM_PROMPT: &str =
    "You improve draft paper-trading stock challenges for an investment app. \
Given a user's draft challenge (name, description, tickers) and optionally a theme, \
respond with STRICT JSON of this exact shape: \
{\"name\": \"...\", \"description\": \"...\", \"universe\": [\"TICKER\", ...]}. \
Rules: \
- Keep the user's intent; if a theme is given, respect it too. \
- name: keep the user's name unless it is clearly weak; if so, tighten it (max 6 words). \
- description: improve and expand the draft into a crisp 2-4 sentence investable thesis, \
stating what is in and out of scope. \
- universe: KEEP every user-provided ticker, then add complementary liquid, \
exchange-listed stock or equity-ETF tickers up to about 30 total, \
as plain ticker symbols (e.g. \"AAPL\", or \"RHM.DE\" for non-US listings).";

/// A user's draft challenge to enhance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftChallenge {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub universe: Vec<String>,
}

impl DraftChallenge {
    fn is_empty(&self) -> bool {
        self.name.trim().is_empty()
            && self.description.trim().is_empty()
            && self.universe.iter().all(|t| t.trim().is_empty())
    }
}

/// Request for [`generate_challenge_spec`]: a free-text theme, a draft
/// challenge to enhance, or both. At least one must be non-empty.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateChallengeSpecRequest {
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub draft: Option<DraftChallenge>,
}

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

/// Build the (system, user) prompt pair for a request, validating that at
/// least a theme or a non-empty draft is present.
fn build_prompts(
    request: &GenerateChallengeSpecRequest,
) -> Result<(&'static str, String), AiError> {
    let theme = request
        .theme
        .as_deref()
        .map(str::trim)
        .filter(|theme| !theme.is_empty());
    let draft = request.draft.as_ref().filter(|draft| !draft.is_empty());

    match (theme, draft) {
        (theme, Some(draft)) => {
            let mut prompt = String::new();
            if let Some(theme) = theme {
                prompt.push_str(&format!("Theme: {theme}\n"));
            }
            prompt.push_str(&format!("Draft name: {}\n", draft.name.trim()));
            prompt.push_str(&format!(
                "Draft description: {}\n",
                draft.description.trim()
            ));
            let tickers: Vec<String> = draft
                .universe
                .iter()
                .map(|t| t.trim().to_uppercase())
                .filter(|t| !t.is_empty())
                .collect();
            prompt.push_str(&format!("Draft tickers (keep all): {}", tickers.join(", ")));
            Ok((ENHANCE_SYSTEM_PROMPT, prompt))
        }
        (Some(theme), None) => Ok((GENERATE_SYSTEM_PROMPT, format!("Theme: {theme}"))),
        (None, None) => Err(AiError::InvalidInput(
            "provide a theme or a draft challenge".to_string(),
        )),
    }
}

/// Generate or enhance an arena challenge spec using the AI Assistant's
/// configured default provider and model. With only a theme, generates a
/// fresh spec; with a draft, enhances it (keeping user tickers).
pub async fn generate_challenge_spec<E: AiEnvironment>(
    env: Arc<E>,
    request: GenerateChallengeSpecRequest,
) -> Result<GeneratedArenaChallengeSpec, AiError> {
    let (system_prompt, prompt) = build_prompts(&request)?;

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
            system_prompt: system_prompt.to_string(),
            prompt,
        })
        .await?;

    let value = parse_model_json(&raw)?;
    let spec: RawChallengeSpec = serde_json::from_value(value)
        .map_err(|e| AiError::Provider(format!("Model returned an unexpected JSON shape: {e}")))?;

    // Validate against the market-data symbol search. User-provided draft
    // tickers go first so they are always kept when they resolve.
    let mut candidates: Vec<String> = request
        .draft
        .map(|draft| draft.universe)
        .unwrap_or_default();
    candidates.extend(spec.universe);

    let quote_service = env.quote_service();
    let mut seen: HashSet<String> = HashSet::new();
    let mut universe = Vec::new();
    let mut dropped = Vec::new();
    for raw_ticker in candidates {
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

    #[test]
    fn theme_only_uses_generate_prompt() {
        let request = GenerateChallengeSpecRequest {
            theme: Some("European defense primes".to_string()),
            draft: None,
        };
        let (system, prompt) = build_prompts(&request).expect("prompts");
        assert_eq!(system, GENERATE_SYSTEM_PROMPT);
        assert_eq!(prompt, "Theme: European defense primes");
    }

    #[test]
    fn draft_uses_enhance_prompt_and_lists_user_tickers() {
        let request = GenerateChallengeSpecRequest {
            theme: Some("defense".to_string()),
            draft: Some(DraftChallenge {
                name: "My defense picks".to_string(),
                description: "Buy defense stuff".to_string(),
                universe: vec!["rhm.de".to_string(), " BA.L ".to_string()],
            }),
        };
        let (system, prompt) = build_prompts(&request).expect("prompts");
        assert_eq!(system, ENHANCE_SYSTEM_PROMPT);
        assert!(prompt.contains("Theme: defense"));
        assert!(prompt.contains("Draft name: My defense picks"));
        assert!(prompt.contains("Draft description: Buy defense stuff"));
        assert!(prompt.contains("Draft tickers (keep all): RHM.DE, BA.L"));
    }

    #[test]
    fn rejects_empty_request() {
        let empty_draft = GenerateChallengeSpecRequest {
            theme: Some("   ".to_string()),
            draft: Some(DraftChallenge::default()),
        };
        assert!(matches!(
            build_prompts(&empty_draft),
            Err(AiError::InvalidInput(_))
        ));
        assert!(matches!(
            build_prompts(&GenerateChallengeSpecRequest::default()),
            Err(AiError::InvalidInput(_))
        ));
    }

    #[test]
    fn deserializes_camel_case_request() {
        let request: GenerateChallengeSpecRequest = serde_json::from_str(
            r#"{"theme": "defense", "draft": {"name": "N", "description": "D", "universe": ["AAPL"]}}"#,
        )
        .expect("request");
        assert_eq!(request.theme.as_deref(), Some("defense"));
        assert_eq!(request.draft.expect("draft").universe, vec!["AAPL"]);

        // {theme} alone still works.
        let theme_only: GenerateChallengeSpecRequest =
            serde_json::from_str(r#"{"theme": "defense"}"#).expect("request");
        assert!(theme_only.draft.is_none());
    }
}
