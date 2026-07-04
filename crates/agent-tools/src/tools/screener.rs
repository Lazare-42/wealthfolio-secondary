//! Stock screener tool backed by the market-data provider registry
//! (currently Financial Modeling Prep).

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wealthfolio_core::quotes::{
    MarketDataError as CoreMarketDataError, ScreenerHit, ScreenerQuery,
};

use crate::env::AgentEnvironment;
use crate::scope::AgentScope;
use crate::tool::{AgentTool, AgentToolAccess, AgentToolError, AgentToolResult};

const DEFAULT_SCREEN_LIMIT: u32 = 25;
const MAX_SCREEN_LIMIT: u32 = 100;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenStocksArgs {
    #[serde(default)]
    pub sector: Option<String>,
    #[serde(default)]
    pub industry: Option<String>,
    #[serde(default)]
    pub market_cap_min: Option<f64>,
    #[serde(default)]
    pub market_cap_max: Option<f64>,
    #[serde(default)]
    pub price_min: Option<f64>,
    #[serde(default)]
    pub price_max: Option<f64>,
    #[serde(default)]
    pub beta_min: Option<f64>,
    #[serde(default)]
    pub beta_max: Option<f64>,
    #[serde(default)]
    pub dividend_min: Option<f64>,
    #[serde(default)]
    pub volume_min: Option<f64>,
    #[serde(default)]
    pub exchange: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub is_etf: Option<bool>,
    #[serde(default)]
    pub is_actively_trading: Option<bool>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenerHitDto {
    pub symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sector: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_cap: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange: Option<String>,
}

impl From<ScreenerHit> for ScreenerHitDto {
    fn from(hit: ScreenerHit) -> Self {
        Self {
            symbol: hit.symbol,
            name: hit.name,
            sector: hit.sector,
            industry: hit.industry,
            market_cap: hit.market_cap,
            price: hit.price,
            exchange: hit.exchange,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenStocksOutput {
    pub count: usize,
    pub hits: Vec<ScreenerHitDto>,
}

pub struct ScreenStocks;

#[async_trait::async_trait]
impl AgentTool for ScreenStocks {
    fn name(&self) -> &'static str {
        "screen_stocks"
    }

    fn description(&self) -> &'static str {
        "Screen the stock market for candidate tickers by STRUCTURAL criteria only: sector, industry, market cap, share price, beta, dividend per share, average volume, exchange, country, and ETF/actively-trading flags. No fundamental filters exist (no P/E, revenue, growth, or margin filters) - do not invent such parameters. Requires the Financial Modeling Prep (FMP) market-data provider to be enabled with an API key (Settings -> Market Data); returns an error when it is not configured."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "sector": {
                    "type": "string",
                    "description": "Sector name, e.g. Technology, Healthcare, Energy, Financial Services."
                },
                "industry": {
                    "type": "string",
                    "description": "Industry name, e.g. Consumer Electronics, Semiconductors, Biotechnology."
                },
                "marketCapMin": {
                    "type": "number",
                    "description": "Minimum market capitalization in USD, e.g. 1000000000 for $1B."
                },
                "marketCapMax": {
                    "type": "number",
                    "description": "Maximum market capitalization in USD."
                },
                "priceMin": {
                    "type": "number",
                    "description": "Minimum share price."
                },
                "priceMax": {
                    "type": "number",
                    "description": "Maximum share price."
                },
                "betaMin": {
                    "type": "number",
                    "description": "Minimum beta."
                },
                "betaMax": {
                    "type": "number",
                    "description": "Maximum beta."
                },
                "dividendMin": {
                    "type": "number",
                    "description": "Minimum annual dividend per share."
                },
                "volumeMin": {
                    "type": "number",
                    "description": "Minimum average daily volume."
                },
                "exchange": {
                    "type": "string",
                    "description": "Exchange code, e.g. NASDAQ or NYSE."
                },
                "country": {
                    "type": "string",
                    "description": "Country code, e.g. US."
                },
                "isEtf": {
                    "type": "boolean",
                    "description": "true to return only ETFs, false to exclude ETFs."
                },
                "isActivelyTrading": {
                    "type": "boolean",
                    "description": "Restrict to actively trading instruments (recommended: true)."
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 100,
                    "description": "Maximum number of results. Defaults to 25."
                }
            }
        })
    }

    fn required_scopes(&self) -> &'static [AgentScope] {
        &[AgentScope::PerformanceRead]
    }

    fn access_level(&self) -> AgentToolAccess {
        AgentToolAccess::Read
    }

    async fn call(
        &self,
        env: Arc<dyn AgentEnvironment>,
        args: serde_json::Value,
    ) -> Result<AgentToolResult, AgentToolError> {
        let args: ScreenStocksArgs = serde_json::from_value(args)?;
        let query = screener_query_from_args(args);

        let hits = env
            .quote_service()
            .screen_stocks(&query)
            .await
            .map_err(screener_tool_error)?;

        let hits: Vec<ScreenerHitDto> = hits.into_iter().map(Into::into).collect();
        Ok(AgentToolResult {
            content: serde_json::to_value(ScreenStocksOutput {
                count: hits.len(),
                hits,
            })?,
        })
    }
}

/// Map a quote-service screening failure to a tool error. When no enabled
/// provider supports screening the registry returns
/// `NotSupported { operation: "screener", provider: "all" }`; matching on the
/// typed variant (rather than the message text) keeps the friendly guidance
/// robust against error-message rewording.
fn screener_tool_error(error: wealthfolio_core::Error) -> AgentToolError {
    match &error {
        wealthfolio_core::Error::MarketData(CoreMarketDataError::NotSupported {
            operation,
            ..
        }) if operation == "screener" => AgentToolError::ExecutionFailed(
            "Stock screening is not available: the Financial Modeling Prep (FMP) \
             market-data provider is not configured. Ask the user to enable FMP and \
             add an API key in Settings -> Market Data, then try again."
                .to_string(),
        ),
        _ => AgentToolError::ExecutionFailed(error.to_string()),
    }
}

/// Map tool args onto the core screener query, clamping `limit`.
fn screener_query_from_args(args: ScreenStocksArgs) -> ScreenerQuery {
    ScreenerQuery {
        sector: args.sector,
        industry: args.industry,
        market_cap_min: args.market_cap_min,
        market_cap_max: args.market_cap_max,
        price_min: args.price_min,
        price_max: args.price_max,
        beta_min: args.beta_min,
        beta_max: args.beta_max,
        dividend_min: args.dividend_min,
        volume_min: args.volume_min,
        exchange: args.exchange,
        country: args.country,
        is_etf: args.is_etf,
        is_actively_trading: args.is_actively_trading,
        limit: Some(
            args.limit
                .unwrap_or(DEFAULT_SCREEN_LIMIT)
                .clamp(1, MAX_SCREEN_LIMIT),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_deserialize_from_camel_case_and_map_to_query() {
        let args: ScreenStocksArgs = serde_json::from_value(serde_json::json!({
            "sector": "Technology",
            "marketCapMin": 1e9,
            "isEtf": false,
            "limit": 10
        }))
        .expect("args");
        let query = screener_query_from_args(args);
        assert_eq!(query.sector.as_deref(), Some("Technology"));
        assert_eq!(query.market_cap_min, Some(1e9));
        assert_eq!(query.is_etf, Some(false));
        assert_eq!(query.limit, Some(10));
    }

    #[test]
    fn limit_defaults_and_clamps() {
        let query = screener_query_from_args(ScreenStocksArgs::default());
        assert_eq!(query.limit, Some(DEFAULT_SCREEN_LIMIT));

        let args: ScreenStocksArgs =
            serde_json::from_value(serde_json::json!({ "limit": 5000 })).expect("args");
        assert_eq!(screener_query_from_args(args).limit, Some(MAX_SCREEN_LIMIT));
    }

    #[test]
    fn not_supported_screener_error_maps_to_friendly_guidance() {
        let error = wealthfolio_core::Error::MarketData(CoreMarketDataError::NotSupported {
            operation: "screener".to_string(),
            provider: "all".to_string(),
        });
        let message = screener_tool_error(error).to_string();
        assert!(message.contains("Financial Modeling Prep"));
        assert!(message.contains("Settings -> Market Data"));
    }

    #[test]
    fn other_errors_pass_through_unchanged() {
        // NotSupported for a different operation is not the screener case.
        let error = wealthfolio_core::Error::MarketData(CoreMarketDataError::NotSupported {
            operation: "profile".to_string(),
            provider: "all".to_string(),
        });
        let message = screener_tool_error(error).to_string();
        assert!(!message.contains("Financial Modeling Prep"));
        assert!(message.contains("does not support 'profile'"));

        let error = wealthfolio_core::Error::MarketData(CoreMarketDataError::ProviderError(
            "FMP: HTTP 500".to_string(),
        ));
        let message = screener_tool_error(error).to_string();
        assert!(message.contains("FMP: HTTP 500"));
    }

    #[test]
    fn hit_dto_is_compact() {
        let hit = ScreenerHit {
            symbol: "AAPL".to_string(),
            name: Some("Apple Inc.".to_string()),
            market_cap: Some(3e12),
            sector: Some("Technology".to_string()),
            industry: None,
            price: Some(190.0),
            exchange: Some("NASDAQ".to_string()),
            country: Some("US".to_string()),
        };
        let value = serde_json::to_value(ScreenerHitDto::from(hit)).expect("dto");
        assert_eq!(value["symbol"], "AAPL");
        assert_eq!(value["marketCap"], 3e12);
        // Country is intentionally dropped and absent fields are omitted.
        assert!(value.get("country").is_none());
        assert!(value.get("industry").is_none());
    }
}
