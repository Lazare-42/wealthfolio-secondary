//! AI-generated arena challenge specs from a free-text theme or a user draft.
//!
//! Given a theme like "European defense primes", asks the AI Assistant's
//! configured provider/model for a challenge name, description, and ticker
//! universe, then validates each ticker against the market-data symbol
//! search (same lookup the chat's `search_market_symbols` tool uses).
//! When a draft challenge is provided instead (or as well), the model
//! enhances the draft: it keeps the user's intent and tickers, sharpens the
//! description, and adds complementary tickers.
//!
//! When the resolved model supports tool calling, the generation runs as a
//! small rig tool loop: the model can discover candidates with the shared
//! `screen_stocks` tool and verify tickers with a `search_symbol` tool
//! before emitting its strict-JSON answer. Models without tool support keep
//! the historical single-shot path. Either way, `validate_universe` remains
//! the safety net for every ticker.

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use rig::completion::ToolDefinition;
use rig::tool::{ToolDyn, ToolError};
use rig::wasm_compat::WasmBoxedFuture;
use serde::{Deserialize, Serialize};
use wealthfolio_agent_tools::AgentEnvironment;
use wealthfolio_core::ai_arena::{service::parse_model_json, ArenaDecisionRequest};
use wealthfolio_core::quotes::QuoteServiceTrait;

use crate::arena_runner::AiArenaLlmRunner;
use crate::env::AiEnvironment;
use crate::error::AiError;
use crate::providers::ProviderService;
use crate::tools::RigAgentTool;

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
Think in company names first, then map each company to its US ticker. \
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
Think in company names first, then map each company to its US ticker. \
The arena supports US listings only — no foreign exchange suffixes.";

/// Extra system-prompt guidance appended only on the tool-loop path, where
/// the model actually has these tools. The single-shot path must never see
/// it — mentioning unavailable tools invites fake tool-call prose that would
/// break the strict-JSON parse.
const TOOL_LOOP_GUIDANCE: &str = "\n\nYou have two tools: \
- screen_stocks: discover candidate stocks by STRUCTURAL criteria (sector, industry, \
market cap, price, beta, dividend, volume, exchange, ETF flag). Use it when the theme \
implies such criteria (e.g. sector, company size, dividend payers). It has NO \
fundamental filters (no P/E, revenue, growth). If it returns an error (e.g. the FMP \
provider is not configured), do not retry it — fall back to companies you know. \
- search_symbol: verify a candidate company's US ticker before putting it in the \
universe. \
Work in company names first, use screen_stocks to discover candidates, verify each \
ticker with search_symbol, then answer. Your FINAL message must be ONLY the strict \
JSON object — no prose, no tool commentary.";

/// Maximum model-↔-tool round trips on the tool-loop path.
const MAX_TOOL_TURNS: usize = 6;
/// Symbol-search tool results shown to the model per query.
const SYMBOL_TOOL_RESULTS: usize = 5;

/// Per-call cap on the screener `limit` argument inside the tool loop.
const ARENA_SCREENER_CALL_LIMIT: u64 = 25;
/// Cumulative screener hits fed back to the model across one generation loop.
const ARENA_SCREENER_HIT_BUDGET: usize = 100;

/// Maximum theme length accepted at the crates/ai boundary.
const MAX_THEME_CHARS: usize = 2000;
/// Post-parse caps on model-provided text fields.
const MAX_NAME_CHARS: usize = 100;
const MAX_DESCRIPTION_CHARS: usize = 1000;
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

/// The `search_symbol` tool for the universe-generation tool loop: verifies
/// a candidate ticker (or finds one from a company name) through the same
/// USD symbol search that `validate_universe` uses.
struct UniverseSymbolSearch {
    quote_service: Arc<dyn QuoteServiceTrait>,
}

#[derive(Debug, Deserialize)]
struct UniverseSymbolSearchArgs {
    query: String,
}

/// Compact search-result view for the model.
fn symbol_search_output(
    results: Vec<wealthfolio_core::quotes::SymbolSearchResult>,
) -> serde_json::Value {
    let results: Vec<serde_json::Value> = results
        .into_iter()
        .take(SYMBOL_TOOL_RESULTS)
        .map(|r| {
            serde_json::json!({
                "symbol": r.canonical_symbol.unwrap_or(r.symbol),
                "name": r.long_name,
                "exchange": r.exchange,
                "quoteType": r.quote_type,
            })
        })
        .collect();
    serde_json::json!({ "results": results })
}

impl ToolDyn for UniverseSymbolSearch {
    fn name(&self) -> String {
        "search_symbol".to_string()
    }

    fn definition<'a>(&'a self, _prompt: String) -> WasmBoxedFuture<'a, ToolDefinition> {
        Box::pin(async move {
            ToolDefinition {
                name: "search_symbol".to_string(),
                description: "Search US-listed, USD-priced market symbols by ticker or \
                              company name. Use it to verify each candidate ticker before \
                              including it in the universe."
                    .to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Ticker or company name, e.g. NVDA or Nvidia."
                        }
                    },
                    "required": ["query"]
                }),
            }
        })
    }

    fn call<'a>(&'a self, args: String) -> WasmBoxedFuture<'a, Result<String, ToolError>> {
        Box::pin(async move {
            let args: UniverseSymbolSearchArgs =
                serde_json::from_str(&args).map_err(ToolError::JsonError)?;
            let output = match self
                .quote_service
                .search_symbol_with_currency(args.query.trim(), Some("USD"))
                .await
            {
                Ok(results) => symbol_search_output(results),
                // Feed lookup failures back to the model as data, not as a
                // hard tool error, so it can continue from its own knowledge.
                Err(e) => serde_json::json!({ "error": e.to_string() }),
            };
            serde_json::to_string(&output).map_err(ToolError::JsonError)
        })
    }
}

/// Wraps the shared `screen_stocks` tool for the arena generation loop only,
/// bounding how much screener output can flow back into the model's context:
/// each call's `limit` argument is capped at [`ARENA_SCREENER_CALL_LIMIT`],
/// and once a cumulative [`ARENA_SCREENER_HIT_BUDGET`] hits have been
/// returned across the loop, further calls get a budget-exhausted notice
/// instead of provider data. Chat and MCP use the unwrapped tool and are
/// unaffected.
struct BudgetedScreenStocks {
    inner: RigAgentTool,
    remaining_hits: AtomicUsize,
}

impl BudgetedScreenStocks {
    fn new(inner: RigAgentTool) -> Self {
        Self {
            inner,
            remaining_hits: AtomicUsize::new(ARENA_SCREENER_HIT_BUDGET),
        }
    }
}

/// Cap the screener `limit` argument at `cap` (also applied when absent, so
/// the tool's own default cannot exceed the remaining budget). Unparseable
/// or non-object args pass through untouched so the inner tool reports its
/// usual schema error; a present-but-non-integer `limit` is likewise left
/// for the inner tool to reject.
fn cap_screener_limit(args: &str, cap: u64) -> String {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(args) else {
        return args.to_string();
    };
    let Some(object) = value.as_object_mut() else {
        return args.to_string();
    };
    match object.get("limit").map(serde_json::Value::as_u64) {
        Some(Some(requested)) => {
            object.insert("limit".to_string(), serde_json::json!(requested.min(cap)));
        }
        Some(None) => {}
        None => {
            object.insert("limit".to_string(), serde_json::json!(cap));
        }
    }
    value.to_string()
}

/// Truncate a `screen_stocks` output payload to at most `max_hits` hits,
/// fixing `count` to match. Returns the (possibly rewritten) payload and the
/// number of hits it now carries (0 for error or non-screener payloads).
fn truncate_screener_hits(output: &str, max_hits: usize) -> (String, usize) {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(output) else {
        return (output.to_string(), 0);
    };
    let Some(hits) = value.get_mut("hits").and_then(|h| h.as_array_mut()) else {
        return (output.to_string(), 0);
    };
    hits.truncate(max_hits);
    let count = hits.len();
    value["count"] = serde_json::json!(count);
    (value.to_string(), count)
}

impl ToolDyn for BudgetedScreenStocks {
    fn name(&self) -> String {
        ToolDyn::name(&self.inner)
    }

    fn definition<'a>(&'a self, prompt: String) -> WasmBoxedFuture<'a, ToolDefinition> {
        self.inner.definition(prompt)
    }

    fn call<'a>(&'a self, args: String) -> WasmBoxedFuture<'a, Result<String, ToolError>> {
        Box::pin(async move {
            let remaining = self.remaining_hits.load(Ordering::Relaxed);
            if remaining == 0 {
                // Answer as data (not a hard tool error) so the model moves
                // on with the candidates it already has.
                return Ok(serde_json::json!({
                    "error": "screen_stocks result budget exhausted for this request — \
                              do not call it again; work with the candidates you already have"
                })
                .to_string());
            }
            let capped = cap_screener_limit(&args, ARENA_SCREENER_CALL_LIMIT.min(remaining as u64));
            let output = self.inner.call(capped).await?;
            let (output, returned) = truncate_screener_hits(&output, remaining);
            self.remaining_hits.fetch_sub(returned, Ordering::Relaxed);
            Ok(output)
        })
    }
}

/// Whether a failed tool-loop generation should be retried once via the
/// single-shot path. Config errors (invalid input, missing API key) would
/// fail identically without tools, so they surface immediately; anything
/// else (provider rejecting the tools payload, max-turns exhaustion, tool
/// failures bubbling up) gets one tool-free retry.
fn should_retry_single_shot(error: &AiError) -> bool {
    !matches!(error, AiError::InvalidInput(_) | AiError::MissingApiKey(_))
}

/// Truncate to at most `max_chars` characters (char-boundary safe).
fn truncate_chars(value: &str, max_chars: usize) -> &str {
    match value.char_indices().nth(max_chars) {
        Some((idx, _)) => &value[..idx],
        None => value,
    }
}

/// Generate or enhance an arena challenge spec using the AI Assistant's
/// configured default provider and model. With only a theme, generates a
/// fresh spec; with a draft, enhances it (keeping user tickers).
pub async fn generate_challenge_spec<E: AiEnvironment + 'static>(
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
    // Models with tool support get a small tool loop (screen_stocks +
    // search_symbol); the rest keep the historical single-shot prompt.
    let runner = AiArenaLlmRunner::new(env.clone());
    let capabilities = provider_service.get_model_capabilities(&provider_id, &model_id);
    let raw = if capabilities.tools {
        let agent_env: Arc<dyn AgentEnvironment> = env.clone();
        let tools: Vec<Box<dyn ToolDyn>> = vec![
            Box::new(BudgetedScreenStocks::new(RigAgentTool::new(
                Arc::new(wealthfolio_agent_tools::tools::ScreenStocks),
                agent_env,
            ))),
            Box::new(UniverseSymbolSearch {
                quote_service: env.quote_service(),
            }),
        ];
        let tool_result = runner
            .complete_with_tools(
                ArenaDecisionRequest {
                    provider_id: provider_id.clone(),
                    model_id: model_id.clone(),
                    system_prompt: format!("{system_prompt}{TOOL_LOOP_GUIDANCE}"),
                    prompt: prompt.clone(),
                },
                tools,
                MAX_TOOL_TURNS,
            )
            .await;
        match tool_result {
            Ok(raw) => raw,
            Err(error) if should_retry_single_shot(&error) => {
                log::warn!(
                    "Arena challenge tool-loop generation failed ({error}); \
                     retrying once via the single-shot path"
                );
                runner
                    .complete(ArenaDecisionRequest {
                        provider_id: provider_id.clone(),
                        model_id: model_id.clone(),
                        system_prompt: system_prompt.to_string(),
                        prompt,
                    })
                    .await?
            }
            Err(error) => return Err(error),
        }
    } else {
        runner
            .complete(ArenaDecisionRequest {
                provider_id: provider_id.clone(),
                model_id: model_id.clone(),
                system_prompt: system_prompt.to_string(),
                prompt,
            })
            .await?
    };

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
        name: truncate_chars(spec.name.trim(), MAX_NAME_CHARS).to_string(),
        description: truncate_chars(spec.description.trim(), MAX_DESCRIPTION_CHARS).to_string(),
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
    fn tool_loop_guidance_names_the_tools_and_strict_json() {
        // The tool-path system prompt must tell the model exactly which
        // tools exist, that the screener is structural-only, and that the
        // final message is still strict JSON (parse_model_json depends on it).
        assert!(TOOL_LOOP_GUIDANCE.contains("screen_stocks"));
        assert!(TOOL_LOOP_GUIDANCE.contains("search_symbol"));
        assert!(TOOL_LOOP_GUIDANCE.contains("no P/E"));
        assert!(TOOL_LOOP_GUIDANCE.contains("strict"));
        // The base prompts stay tool-free (single-shot models see them alone).
        assert!(!GENERATE_SYSTEM_PROMPT.contains("screen_stocks"));
        assert!(!ENHANCE_SYSTEM_PROMPT.contains("screen_stocks"));
        // Both paths think in company names first.
        assert!(GENERATE_SYSTEM_PROMPT.contains("company names first"));
        assert!(ENHANCE_SYSTEM_PROMPT.contains("company names first"));
    }

    #[tokio::test]
    async fn search_symbol_tool_definition_is_well_formed() {
        // Only the definition is exercised — no service call happens.
        let tool = UniverseSymbolSearch {
            quote_service: Arc::new(crate::env::test_env::MockQuoteService::default()),
        };
        let def = tool.definition(String::new()).await;
        assert_eq!(def.name, "search_symbol");
        assert_eq!(ToolDyn::name(&tool), "search_symbol");
        assert_eq!(def.parameters["required"], serde_json::json!(["query"]));
        assert_eq!(def.parameters["properties"]["query"]["type"], "string");
    }

    #[tokio::test]
    async fn search_symbol_tool_call_returns_compact_json() {
        let mock = crate::env::test_env::MockQuoteService::default();
        *mock.search_results.write().unwrap() =
            vec![wealthfolio_core::quotes::SymbolSearchResult {
                symbol: "NVDA".to_string(),
                long_name: "NVIDIA Corporation".to_string(),
                exchange: "NMS".to_string(),
                ..Default::default()
            }];
        let tool = UniverseSymbolSearch {
            quote_service: Arc::new(mock),
        };
        let out = tool
            .call(r#"{"query": "Nvidia"}"#.to_string())
            .await
            .expect("tool output");
        let value: serde_json::Value = serde_json::from_str(&out).expect("json");
        assert_eq!(value["results"][0]["symbol"], "NVDA");
        assert_eq!(value["results"][0]["name"], "NVIDIA Corporation");
    }

    #[test]
    fn symbol_search_output_is_compact_and_capped() {
        let result = |symbol: &str| wealthfolio_core::quotes::SymbolSearchResult {
            symbol: symbol.to_string(),
            ..Default::default()
        };
        let many: Vec<_> = (0..10).map(|i| result(&format!("T{i}"))).collect();
        let value = symbol_search_output(many);
        let results = value["results"].as_array().expect("results");
        assert_eq!(results.len(), SYMBOL_TOOL_RESULTS);
        assert_eq!(results[0]["symbol"], "T0");
        assert!(results[0].get("score").is_none(), "compact view only");
    }

    #[test]
    fn truncate_chars_is_char_boundary_safe() {
        assert_eq!(truncate_chars("short", MAX_NAME_CHARS), "short");
        assert_eq!(truncate_chars("abcdef", 3), "abc");

        // Multibyte characters: counts chars, never splits one.
        let name = "é".repeat(MAX_NAME_CHARS + 50);
        let capped = truncate_chars(&name, MAX_NAME_CHARS);
        assert_eq!(capped.chars().count(), MAX_NAME_CHARS);
        assert_eq!(capped, "é".repeat(MAX_NAME_CHARS));

        let description = "🚀".repeat(MAX_DESCRIPTION_CHARS + 1);
        assert_eq!(
            truncate_chars(&description, MAX_DESCRIPTION_CHARS)
                .chars()
                .count(),
            MAX_DESCRIPTION_CHARS
        );
    }

    #[test]
    fn tool_loop_failures_retry_single_shot_except_config_errors() {
        // Provider/tool failures (e.g. max turns, tools payload rejected)
        // fall back to the single-shot path.
        assert!(should_retry_single_shot(&AiError::Provider(
            "MaxDepthError: (6) turns".to_string()
        )));
        assert!(should_retry_single_shot(&AiError::ToolExecutionFailed(
            "boom".to_string()
        )));
        // Config errors would fail identically single-shot — surface them.
        assert!(!should_retry_single_shot(&AiError::InvalidInput(
            "bad request".to_string()
        )));
        assert!(!should_retry_single_shot(&AiError::MissingApiKey(
            "openai".to_string()
        )));
    }

    #[test]
    fn cap_screener_limit_caps_defaults_and_passes_through() {
        let get_limit = |args: &str| -> serde_json::Value {
            serde_json::from_str::<serde_json::Value>(args).expect("json")["limit"].clone()
        };

        // Oversized requests are capped.
        let capped = cap_screener_limit(r#"{"sector":"Technology","limit":100}"#, 25);
        assert_eq!(get_limit(&capped), serde_json::json!(25));
        assert!(capped.contains("Technology"), "other args preserved");

        // Smaller requests pass through.
        let capped = cap_screener_limit(r#"{"limit":5}"#, 25);
        assert_eq!(get_limit(&capped), serde_json::json!(5));

        // An absent limit is pinned to the cap (the tool's own default may
        // exceed the remaining budget).
        let capped = cap_screener_limit("{}", 10);
        assert_eq!(get_limit(&capped), serde_json::json!(10));

        // Invalid limit and unparseable args are left for the inner tool
        // to reject with its usual error.
        let capped = cap_screener_limit(r#"{"limit":"lots"}"#, 25);
        assert_eq!(get_limit(&capped), serde_json::json!("lots"));
        assert_eq!(cap_screener_limit("not json", 25), "not json");
    }

    #[test]
    fn truncate_screener_hits_truncates_and_fixes_count() {
        let output = serde_json::json!({
            "count": 3,
            "hits": [{"symbol": "A"}, {"symbol": "B"}, {"symbol": "C"}]
        })
        .to_string();

        let (truncated, returned) = truncate_screener_hits(&output, 2);
        assert_eq!(returned, 2);
        let value: serde_json::Value = serde_json::from_str(&truncated).expect("json");
        assert_eq!(value["count"], 2);
        assert_eq!(value["hits"].as_array().expect("hits").len(), 2);
        assert_eq!(value["hits"][1]["symbol"], "B");

        // Under-budget output is untouched.
        let (kept, returned) = truncate_screener_hits(&output, 100);
        assert_eq!(returned, 3);
        let value: serde_json::Value = serde_json::from_str(&kept).expect("json");
        assert_eq!(value["count"], 3);

        // Error payloads carry no hits and consume no budget.
        let (out, returned) = truncate_screener_hits(r#"{"error":"nope"}"#, 2);
        assert_eq!(returned, 0);
        assert!(out.contains("nope"));
    }

    #[tokio::test]
    async fn budgeted_screener_refuses_once_budget_is_exhausted() {
        // With the budget at zero the wrapper answers from data without ever
        // touching the inner tool (whose mock quote service would panic).
        let tool = BudgetedScreenStocks {
            inner: RigAgentTool::new(
                Arc::new(wealthfolio_agent_tools::tools::ScreenStocks),
                Arc::new(crate::env::test_env::MockEnvironment::new()),
            ),
            remaining_hits: AtomicUsize::new(0),
        };
        let out = tool.call("{}".to_string()).await.expect("tool output");
        assert!(out.contains("budget exhausted"));

        // Name/definition still pass through to the shared tool.
        assert_eq!(ToolDyn::name(&tool), "screen_stocks");
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
