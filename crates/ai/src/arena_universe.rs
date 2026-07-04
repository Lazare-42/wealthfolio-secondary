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
- universe: 10-30 liquid, US-listed, USD-priced stock or equity-ETF tickers \
that fit the theme, as plain US ticker symbols (e.g. \"AAPL\", \"NVDA\"). \
The arena supports US listings only — no foreign exchange suffixes.";

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
US-listed, USD-priced stock or equity-ETF tickers up to about 30 total, \
as plain US ticker symbols (e.g. \"AAPL\", \"NVDA\"). \
The arena supports US listings only — no foreign exchange suffixes.";

/// Maximum theme length accepted at the crates/ai boundary.
const MAX_THEME_CHARS: usize = 2000;
/// Maximum number of tickers accepted in a user draft.
const MAX_DRAFT_TICKERS: usize = 40;
/// Total candidates (draft first, then model) validated per request.
const MAX_CANDIDATES: usize = 40;
/// Validation stops once this many symbols resolve.
const MAX_UNIVERSE: usize = 30;
/// Concurrent symbol lookups during validation.
const LOOKUP_CONCURRENCY: usize = 4;

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

    if let Some(theme) = theme {
        if theme.chars().count() > MAX_THEME_CHARS {
            return Err(AiError::InvalidInput(format!(
                "theme is too long (max {MAX_THEME_CHARS} characters)"
            )));
        }
    }
    if let Some(draft) = draft {
        let ticker_count = draft
            .universe
            .iter()
            .filter(|t| !t.trim().is_empty())
            .count();
        if ticker_count > MAX_DRAFT_TICKERS {
            return Err(AiError::InvalidInput(format!(
                "draft universe is too large (max {MAX_DRAFT_TICKERS} tickers)"
            )));
        }
    }

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

    // Validate against the market-data symbol search, matching results the
    // same way trade-time resolution does (canonical_symbol fallback OR raw
    // symbol — see core ai_arena service::resolve_allowed_price). User-provided
    // draft tickers go first so they are always kept when they resolve.
    let mut candidates: Vec<String> = request
        .draft
        .map(|draft| draft.universe)
        .unwrap_or_default();
    candidates.extend(spec.universe);

    let quote_service = env.quote_service();
    let (universe, dropped) = validate_universe(candidates, |ticker| {
        let quote_service = quote_service.clone();
        async move {
            quote_service
                .search_symbol_with_currency(&ticker, Some("USD"))
                .await
                .map(|results| {
                    results.iter().any(|result| {
                        result
                            .canonical_symbol
                            .as_deref()
                            .unwrap_or(result.symbol.as_str())
                            .eq_ignore_ascii_case(&ticker)
                            || result.symbol.eq_ignore_ascii_case(&ticker)
                    })
                })
                .unwrap_or(false)
        }
    })
    .await?;

    Ok(GeneratedArenaChallengeSpec {
        name: spec.name.trim().to_string(),
        description: spec.description.trim().to_string(),
        universe,
        dropped,
        provider_id,
        model_id,
    })
}

/// Trim, uppercase, and dedupe candidates (order-preserving, so draft tickers
/// listed first keep their keep-priority), cap them at [`MAX_CANDIDATES`],
/// then resolve each through `lookup` with bounded concurrency, stopping once
/// [`MAX_UNIVERSE`] symbols resolve. Errors when nothing resolves so callers
/// never get an `Ok` spec with an empty universe (e.g. during a market-data
/// outage).
async fn validate_universe<F, Fut>(
    candidates: Vec<String>,
    lookup: F,
) -> Result<(Vec<String>, Vec<String>), AiError>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    use futures::StreamExt;

    let mut seen: HashSet<String> = HashSet::new();
    let candidates: Vec<String> = candidates
        .into_iter()
        .map(|t| t.trim().to_uppercase())
        .filter(|t| !t.is_empty() && seen.insert(t.clone()))
        .take(MAX_CANDIDATES)
        .collect();

    let lookup = &lookup;
    // `buffered` keeps result order == candidate order, so draft tickers
    // (listed first) keep priority even with concurrent lookups.
    let mut results = futures::stream::iter(candidates.into_iter().map(|ticker| {
        let resolved = lookup(ticker.clone());
        async move { (ticker, resolved.await) }
    }))
    .buffered(LOOKUP_CONCURRENCY);

    let mut universe = Vec::new();
    let mut dropped = Vec::new();
    while let Some((ticker, resolved)) = results.next().await {
        if resolved {
            universe.push(ticker);
            if universe.len() >= MAX_UNIVERSE {
                break;
            }
        } else {
            dropped.push(ticker);
        }
    }

    if universe.is_empty() {
        return Err(AiError::Provider(
            "could not validate any universe symbols — market data may be unavailable".to_string(),
        ));
    }
    Ok((universe, dropped))
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
    fn rejects_oversized_inputs() {
        let long_theme = GenerateChallengeSpecRequest {
            theme: Some("x".repeat(MAX_THEME_CHARS + 1)),
            draft: None,
        };
        assert!(matches!(
            build_prompts(&long_theme),
            Err(AiError::InvalidInput(_))
        ));

        let big_draft = GenerateChallengeSpecRequest {
            theme: None,
            draft: Some(DraftChallenge {
                universe: (0..MAX_DRAFT_TICKERS + 1)
                    .map(|i| format!("T{i}"))
                    .collect(),
                ..Default::default()
            }),
        };
        assert!(matches!(
            build_prompts(&big_draft),
            Err(AiError::InvalidInput(_))
        ));
    }

    #[tokio::test]
    async fn validate_universe_caps_candidates_and_universe() {
        let looked_up = Arc::new(std::sync::Mutex::new(0usize));
        let candidates: Vec<String> = (0..60).map(|i| format!("T{i}")).collect();
        let counter = looked_up.clone();
        let (universe, dropped) = validate_universe(candidates, move |_ticker| {
            let counter = counter.clone();
            async move {
                *counter.lock().unwrap() += 1;
                true
            }
        })
        .await
        .expect("universe");

        // Stops once MAX_UNIVERSE resolve, in stable candidate order.
        assert_eq!(universe.len(), MAX_UNIVERSE);
        assert_eq!(universe.first().map(String::as_str), Some("T0"));
        assert_eq!(universe.last().map(String::as_str), Some("T29"));
        assert!(dropped.is_empty());
        // Never validates more than the candidate cap.
        assert!(*looked_up.lock().unwrap() <= MAX_CANDIDATES);
    }

    #[tokio::test]
    async fn validate_universe_dedupes_and_keeps_draft_first_order() {
        let candidates = vec![
            " aapl ".to_string(),
            "MSFT".to_string(),
            "AAPL".to_string(),
            "".to_string(),
            "bad".to_string(),
        ];
        let (universe, dropped) =
            validate_universe(candidates, |ticker| async move { ticker != "BAD" })
                .await
                .expect("universe");
        assert_eq!(universe, vec!["AAPL", "MSFT"]);
        assert_eq!(dropped, vec!["BAD"]);
    }

    #[tokio::test]
    async fn validate_universe_errors_when_nothing_resolves() {
        let result = validate_universe(vec!["AAPL".to_string()], |_| async { false }).await;
        assert!(matches!(result, Err(AiError::Provider(_))));

        // No candidates at all is also an error, never an empty Ok universe.
        let result = validate_universe(Vec::new(), |_| async { true }).await;
        assert!(matches!(result, Err(AiError::Provider(_))));
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
