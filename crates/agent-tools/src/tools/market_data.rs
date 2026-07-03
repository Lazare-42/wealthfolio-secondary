//! Market-data tools for scenario and benchmark analysis.

use chrono::{Datelike, Local, NaiveDate};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use wealthfolio_core::accounts::{account_supports_portfolio_scope, Account, AccountPurpose};
use wealthfolio_core::portfolio::performance::{
    empty_performance_metrics, performance_account_ids_from_map,
    performance_account_tracking_modes_from_map, performance_account_types_from_map,
    sync_performance_summary_quality, unique_account_ids, DataQualityStatus,
    PerformanceResult as CorePerformanceResult,
};
use wealthfolio_core::quotes::{Quote, SymbolSearchResult};
use wealthfolio_core::scenarios::{BasketPosition, ScenarioKind};

use crate::env::AgentEnvironment;
use crate::scope::AgentScope;
use crate::tool::{AgentTool, AgentToolAccess, AgentToolError, AgentToolResult};

const DEFAULT_SYMBOL_SEARCH_LIMIT: usize = 10;
const MAX_SYMBOL_SEARCH_LIMIT: usize = 25;
pub(crate) const DEFAULT_SYMBOL_SAMPLE_POINTS: usize = 36;
pub(crate) const MAX_SYMBOL_SAMPLE_POINTS: usize = 120;
const MAX_COMPARE_SYMBOLS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PriceBasis {
    AdjustedClose,
    Close,
}

impl PriceBasis {
    pub(crate) fn parse(value: Option<&str>) -> Result<Self, AgentToolError> {
        match value.unwrap_or("adjclose").to_ascii_lowercase().as_str() {
            "adjclose" | "adjusted_close" | "adjustedclose" => Ok(Self::AdjustedClose),
            "close" => Ok(Self::Close),
            other => Err(AgentToolError::InvalidInput(format!(
                "Unsupported price basis '{other}'. Use 'adjclose' or 'close'."
            ))),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::AdjustedClose => "adjclose",
            Self::Close => "close",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PricePoint {
    date: NaiveDate,
    price: f64,
    currency: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMarketSymbolsArgs {
    pub query: String,
    #[serde(default)]
    pub account_currency: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketSymbolDto {
    pub symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_symbol: Option<String>,
    pub short_name: String,
    pub long_name: String,
    pub exchange: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_mic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_name: Option<String>,
    pub quote_type: String,
    pub type_display: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_source: Option<String>,
    pub is_existing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing_asset_id: Option<String>,
    pub score: f64,
}

impl From<SymbolSearchResult> for MarketSymbolDto {
    fn from(value: SymbolSearchResult) -> Self {
        Self {
            symbol: value.symbol,
            canonical_symbol: value.canonical_symbol,
            provider_symbol: value.provider_symbol,
            short_name: value.short_name,
            long_name: value.long_name,
            exchange: value.exchange,
            exchange_mic: value.exchange_mic,
            exchange_name: value.exchange_name,
            quote_type: value.quote_type,
            type_display: value.type_display,
            currency: value.currency,
            data_source: value.data_source,
            is_existing: value.is_existing,
            existing_asset_id: value.existing_asset_id,
            score: value.score,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMarketSymbolsOutput {
    pub query: String,
    pub results: Vec<MarketSymbolDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_count: Option<usize>,
}

pub struct SearchMarketSymbols;

#[async_trait::async_trait]
impl AgentTool for SearchMarketSymbols {
    fn name(&self) -> &'static str {
        "search_market_symbols"
    }

    fn description(&self) -> &'static str {
        "Search Wealthfolio market-data symbols for benchmark or scenario candidates. Prefer results with isExisting=true and pass existingAssetId to get_symbol_performance."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Ticker, fund name, company name, or benchmark phrase to search, e.g. SPY, S&P 500, Nasdaq 100."
                },
                "accountCurrency": {
                    "type": "string",
                    "description": "Optional account currency such as USD, CAD, or EUR to sort exchange relevance."
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 25,
                    "description": "Maximum number of results to return. Defaults to 10."
                }
            },
            "required": ["query"]
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
        let args: SearchMarketSymbolsArgs = serde_json::from_value(args)?;
        let query = args.query.trim();
        if query.is_empty() {
            return Err(AgentToolError::InvalidInput(
                "query must not be empty".to_string(),
            ));
        }

        let limit = args
            .limit
            .unwrap_or(DEFAULT_SYMBOL_SEARCH_LIMIT)
            .clamp(1, MAX_SYMBOL_SEARCH_LIMIT);
        let results = env
            .quote_service()
            .search_symbol_with_currency(query, args.account_currency.as_deref())
            .await
            .map_err(|e| AgentToolError::ExecutionFailed(e.to_string()))?;

        let original_count = results.len();
        let results: Vec<MarketSymbolDto> =
            results.into_iter().take(limit).map(Into::into).collect();
        let truncated = original_count > results.len();

        Ok(AgentToolResult {
            content: serde_json::to_value(SearchMarketSymbolsOutput {
                query: query.to_string(),
                results,
                truncated: truncated.then_some(true),
                original_count: truncated.then_some(original_count),
            })?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSymbolPerformanceArgs {
    #[serde(default)]
    pub asset_id: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub basis: Option<String>,
    #[serde(default)]
    pub sample_points: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SymbolPerformanceMetricsOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_return_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annualized_return_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volatility_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_drawdown_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_day_return_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worst_day_return_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SymbolPerformanceDataQualityOutput {
    pub status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolPerformancePointOutput {
    pub date: String,
    pub price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drawdown_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolPerformanceOutput {
    pub symbol: String,
    pub asset_id: String,
    pub basis: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_end_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_quote_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_quote_date: Option<String>,
    pub quote_count: usize,
    pub metrics: SymbolPerformanceMetricsOutput,
    pub data_quality: SymbolPerformanceDataQualityOutput,
    pub points: Vec<SymbolPerformancePointOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_point_count: Option<usize>,
}

pub struct GetSymbolPerformance;

#[async_trait::async_trait]
impl AgentTool for GetSymbolPerformance {
    fn name(&self) -> &'static str {
        "get_symbol_performance"
    }

    fn description(&self) -> &'static str {
        "Compute price-based performance for a market symbol or Wealthfolio asset id from local quote history. Use search_market_symbols first and prefer existingAssetId for benchmark scenarios."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "assetId": {
                    "type": "string",
                    "description": "Canonical Wealthfolio asset id, usually an existingAssetId from search_market_symbols, e.g. SEC:SPY:ARCX."
                },
                "symbol": {
                    "type": "string",
                    "description": "Ticker or symbol to resolve when assetId is unknown, e.g. SPY. assetId is preferred."
                },
                "startDate": {
                    "type": "string",
                    "description": "Optional start date in YYYY-MM-DD format."
                },
                "endDate": {
                    "type": "string",
                    "description": "Optional end date in YYYY-MM-DD format."
                },
                "basis": {
                    "type": "string",
                    "enum": ["adjclose", "close"],
                    "description": "Price basis. Defaults to adjusted close."
                },
                "samplePoints": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 120,
                    "description": "Maximum sampled points to return for charting/context. Defaults to 36."
                }
            },
            "required": []
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
        let args: GetSymbolPerformanceArgs = serde_json::from_value(args)?;
        let start_date = parse_optional_date(args.start_date.as_deref(), "startDate")?;
        let end_date = parse_optional_date(args.end_date.as_deref(), "endDate")?;
        validate_date_order(start_date, end_date)?;
        let basis = PriceBasis::parse(args.basis.as_deref())?;
        let sample_points = args
            .sample_points
            .unwrap_or(DEFAULT_SYMBOL_SAMPLE_POINTS)
            .min(MAX_SYMBOL_SAMPLE_POINTS);

        let input = symbol_input(args.asset_id.as_deref(), args.symbol.as_deref())?;
        let resolved = resolve_symbol_input(env.clone(), input, args.asset_id.is_some()).await?;
        let (quotes, fetch_warnings) = load_benchmark_quotes(
            env.clone(),
            &resolved.asset_id,
            resolved.currency.as_deref(),
            start_date,
            end_date,
        )
        .await;
        let mut warnings = resolved.warnings;
        warnings.extend(fetch_warnings);

        let output = calculate_symbol_performance_from_quotes(
            resolved.symbol,
            resolved.asset_id,
            basis,
            start_date,
            end_date,
            quotes,
            sample_points,
            warnings,
        );

        Ok(AgentToolResult {
            content: serde_json::to_value(output)?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparePortfolioToSymbolsArgs {
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default = "default_period")]
    pub period: String,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    pub symbols: Vec<String>,
    #[serde(default)]
    pub basis: Option<String>,
    #[serde(default)]
    pub sample_points: Option<usize>,
}

fn default_period() -> String {
    "YTD".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioComparisonOutput {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_end_date: Option<String>,
    pub currency: String,
    pub mode: String,
    pub basis_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twr_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annualized_twr_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub irr_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annualized_irr_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_return_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annualized_value_return_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_percent_pct: Option<f64>,
    pub data_quality_status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparePortfolioToSymbolsOutput {
    pub period: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    pub comparison_basis: String,
    pub portfolio: PortfolioComparisonOutput,
    pub benchmarks: Vec<SymbolPerformanceOutput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

pub struct ComparePortfolioToSymbols;

#[async_trait::async_trait]
impl AgentTool for ComparePortfolioToSymbols {
    fn name(&self) -> &'static str {
        "compare_portfolio_to_symbols"
    }

    fn description(&self) -> &'static str {
        "Compare Wealthfolio portfolio/account performance against benchmark symbols over the same period. Symbols use local quote history and should preferably be canonical asset ids from search_market_symbols."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "accountId": {
                    "type": "string",
                    "description": "Optional account id. Omit for aggregate performance across all performance-eligible accounts."
                },
                "period": {
                    "type": "string",
                    "enum": ["1M", "3M", "6M", "YTD", "1Y", "ALL"],
                    "description": "Comparison period. Defaults to YTD."
                },
                "startDate": {
                    "type": "string",
                    "description": "Optional explicit comparison start date in YYYY-MM-DD format. Overrides period when provided."
                },
                "endDate": {
                    "type": "string",
                    "description": "Optional explicit comparison end date in YYYY-MM-DD format. Defaults to today when startDate is provided."
                },
                "symbols": {
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": 1,
                    "maxItems": 5,
                    "description": "Benchmark symbols or canonical asset ids. Prefer existingAssetId values from search_market_symbols."
                },
                "basis": {
                    "type": "string",
                    "enum": ["adjclose", "close"],
                    "description": "Benchmark price basis. Defaults to adjusted close."
                },
                "samplePoints": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 120,
                    "description": "Maximum sampled points per benchmark. Defaults to 36."
                }
            },
            "required": ["symbols"]
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
        let args: ComparePortfolioToSymbolsArgs = serde_json::from_value(args)?;
        if args.symbols.is_empty() {
            return Err(AgentToolError::InvalidInput(
                "symbols must contain at least one benchmark".to_string(),
            ));
        }
        if args.symbols.len() > MAX_COMPARE_SYMBOLS {
            return Err(AgentToolError::InvalidInput(format!(
                "symbols can contain at most {MAX_COMPARE_SYMBOLS} benchmarks"
            )));
        }

        let period = args.period.to_uppercase();
        let explicit_start_date = parse_optional_date(args.start_date.as_deref(), "startDate")?;
        let explicit_end_date = parse_optional_date(args.end_date.as_deref(), "endDate")?;
        validate_date_order(explicit_start_date, explicit_end_date)?;
        let end_date = explicit_end_date.unwrap_or_else(|| Local::now().date_naive());
        let start_date = explicit_start_date.or_else(|| period_to_start_date(&period, end_date));
        let basis = PriceBasis::parse(args.basis.as_deref())?;
        let sample_points = args
            .sample_points
            .unwrap_or(DEFAULT_SYMBOL_SAMPLE_POINTS)
            .min(MAX_SYMBOL_SAMPLE_POINTS);

        let portfolio = calculate_portfolio_comparison(
            env.clone(),
            args.account_id.as_deref(),
            start_date,
            Some(end_date),
        )
        .await?;

        let mut benchmarks = Vec::with_capacity(args.symbols.len());
        let mut warnings = Vec::new();
        for raw_symbol in args.symbols {
            let input = raw_symbol.trim();
            if input.is_empty() {
                warnings.push("Skipped an empty benchmark symbol.".to_string());
                continue;
            }
            let resolved = resolve_symbol_input(env.clone(), input, input.contains(':')).await?;
            let (quotes, fetch_warnings) = load_benchmark_quotes(
                env.clone(),
                &resolved.asset_id,
                resolved.currency.as_deref(),
                start_date,
                Some(end_date),
            )
            .await;
            let mut symbol_warnings = resolved.warnings;
            symbol_warnings.extend(fetch_warnings);
            let output = calculate_symbol_performance_from_quotes(
                resolved.symbol,
                resolved.asset_id,
                basis,
                start_date,
                Some(end_date),
                quotes,
                sample_points,
                symbol_warnings,
            );
            if output.data_quality.status != "ok" {
                warnings.push(format!(
                    "{} data quality is {}.",
                    output.symbol, output.data_quality.status
                ));
            }
            benchmarks.push(output);
        }

        let output = ComparePortfolioToSymbolsOutput {
            period,
            account_id: args.account_id.filter(|id| !id.trim().is_empty()),
            comparison_basis: "Portfolio metrics come from Wealthfolio performance calculations; benchmarks are price-based returns from local or provider-fetched quote history using the selected basis.".to_string(),
            portfolio,
            benchmarks,
            warnings,
        };

        Ok(AgentToolResult {
            content: serde_json::to_value(output)?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetQuoteArgs {
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub asset_id: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub basis: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetQuoteOutput {
    pub symbol: String,
    pub asset_id: String,
    pub basis: String,
    pub requested_date: Option<String>,
    pub as_of: Option<String>,
    pub price: Option<f64>,
    pub currency: Option<String>,
    /// "ok" when a price was found, otherwise "noData".
    pub status: String,
    pub warnings: Vec<String>,
}

/// Single-price lookup for any symbol or asset — latest, or last available
/// price on or before an optional as-of date.
pub struct GetQuote;

#[async_trait::async_trait]
impl AgentTool for GetQuote {
    fn name(&self) -> &'static str {
        "get_quote"
    }

    fn description(&self) -> &'static str {
        "Get a single price for a symbol or asset: the latest price, or the last available price on or before an optional as-of date. Prices any symbol — tracked or not — by fetching from the market-data provider when local history is missing. Read-only; writes nothing."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Ticker or symbol, e.g. AAPL or SPY. Provide this or assetId." },
                "assetId": { "type": "string", "description": "Canonical Wealthfolio asset id, e.g. SEC:AAPL:XNAS. Use instead of symbol when known." },
                "date": {
                    "type": "string",
                    "description": "Optional as-of date in YYYY-MM-DD format. Returns the last available price on or before this date. Defaults to the latest price."
                },
                "basis": {
                    "type": "string",
                    "enum": ["adjclose", "close"],
                    "description": "Price basis. Defaults to adjusted close."
                }
            },
            "required": []
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
        let args: GetQuoteArgs = serde_json::from_value(args)?;
        let input = symbol_input(args.asset_id.as_deref(), args.symbol.as_deref())?;
        let resolved = resolve_symbol_input(env.clone(), input, args.asset_id.is_some()).await?;
        let basis = PriceBasis::parse(args.basis.as_deref())?;
        let target = parse_optional_date(args.date.as_deref(), "date")?;
        let end = target.unwrap_or_else(|| Local::now().date_naive());
        // Small window so the provider backfill only fires for a single price,
        // not a decade of history. Local history (any age) is still searched in
        // full for the chosen point.
        let window_start = end - chrono::Duration::days(10);
        let (quotes, fetch_warnings) = load_benchmark_quotes(
            env.clone(),
            &resolved.asset_id,
            resolved.currency.as_deref(),
            Some(window_start),
            Some(end),
        )
        .await;

        let mut warnings = resolved.warnings;
        warnings.extend(fetch_warnings);

        let chosen = quotes
            .iter()
            .filter(|quote| quote.timestamp.date_naive() <= end)
            .filter_map(|quote| {
                price_for_quote(quote, basis)
                    .map(|price| (quote.timestamp.date_naive(), price, quote.currency.clone()))
            })
            .max_by_key(|(date, _, _)| *date);

        if let (Some(requested), Some((as_of, _, _))) = (target, chosen.as_ref()) {
            let gap = (requested - *as_of).num_days();
            if gap > 7 {
                warnings.push(format!(
                    "Closest available quote is {as_of}, {gap} day(s) before the requested {requested}."
                ));
            }
        }

        let basis_label = match basis {
            PriceBasis::AdjustedClose => "adjclose",
            PriceBasis::Close => "close",
        }
        .to_string();

        let output = match chosen {
            Some((as_of, price, currency)) => GetQuoteOutput {
                symbol: resolved.symbol,
                asset_id: resolved.asset_id,
                basis: basis_label,
                requested_date: target.map(|d| d.to_string()),
                as_of: Some(as_of.to_string()),
                price: Some(price),
                currency: Some(currency),
                status: "ok".to_string(),
                warnings,
            },
            None => {
                warnings.push(format!(
                    "No quote found for '{}' on or before {end}.",
                    resolved.symbol
                ));
                GetQuoteOutput {
                    symbol: resolved.symbol,
                    asset_id: resolved.asset_id,
                    basis: basis_label,
                    requested_date: target.map(|d| d.to_string()),
                    as_of: None,
                    price: None,
                    currency: None,
                    status: "noData".to_string(),
                    warnings,
                }
            }
        };

        Ok(AgentToolResult {
            content: serde_json::to_value(output)?,
        })
    }
}

pub(crate) struct ResolvedSymbolInput {
    pub(crate) symbol: String,
    pub(crate) asset_id: String,
    /// Quote currency from the market-data match, when known. Used as the
    /// fetch currency when backfilling an untracked benchmark.
    pub(crate) currency: Option<String>,
    pub(crate) warnings: Vec<String>,
}

fn symbol_input<'a>(
    asset_id: Option<&'a str>,
    symbol: Option<&'a str>,
) -> Result<&'a str, AgentToolError> {
    asset_id
        .or(symbol)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AgentToolError::InvalidInput("assetId or symbol must be provided".to_string())
        })
}

pub(crate) async fn resolve_symbol_input(
    env: Arc<dyn AgentEnvironment>,
    input: &str,
    treat_as_asset_id: bool,
) -> Result<ResolvedSymbolInput, AgentToolError> {
    if treat_as_asset_id {
        return Ok(ResolvedSymbolInput {
            symbol: input.to_string(),
            asset_id: input.to_string(),
            currency: None,
            warnings: Vec::new(),
        });
    }

    let results = env
        .quote_service()
        .search_symbol(input)
        .await
        .map_err(|e| AgentToolError::ExecutionFailed(e.to_string()))?;

    if let Some(existing) = results
        .iter()
        .find(|result| result.is_existing && result.existing_asset_id.is_some())
    {
        return Ok(ResolvedSymbolInput {
            symbol: existing.symbol.clone(),
            asset_id: existing
                .existing_asset_id
                .clone()
                .unwrap_or_else(|| input.to_string()),
            currency: existing.currency.clone(),
            warnings: Vec::new(),
        });
    }

    // Not tracked locally: fall back to the best market-data match so an
    // untracked symbol can still be priced via the provider (canonical id +
    // currency feed the on-demand fetch in `load_benchmark_quotes`).
    if let Some(best) = results.first() {
        return Ok(ResolvedSymbolInput {
            symbol: best.symbol.clone(),
            asset_id: best
                .canonical_symbol
                .clone()
                .unwrap_or_else(|| best.symbol.clone()),
            currency: best.currency.clone(),
            warnings: Vec::new(),
        });
    }

    Ok(ResolvedSymbolInput {
        symbol: input.to_string(),
        asset_id: input.to_string(),
        currency: None,
        warnings: vec![format!("No market-data match was found for '{input}'.")],
    })
}

/// Load historical quotes for `asset_id`, backfilling from the market-data
/// provider when local history does not reach back to `start_date`. This is
/// what lets the assistant price *any* symbol — tracked or not, past or
/// present — for scenario benchmarks and on-demand performance lookups.
///
/// Ephemeral: provider quotes are returned for in-memory computation only.
/// No asset row and no quote row is written, so benchmarks never pollute the
/// user's holdings or quote store. Provider/network failures fall back to
/// whatever local history exists, with a warning.
pub(crate) async fn load_benchmark_quotes(
    env: Arc<dyn AgentEnvironment>,
    asset_id: &str,
    currency: Option<&str>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
) -> (Vec<Quote>, Vec<String>) {
    let mut warnings = Vec::new();
    let local = env
        .quote_service()
        .get_historical_quotes(asset_id)
        .unwrap_or_default();

    let end = end_date.unwrap_or_else(|| Local::now().date_naive());
    // When no explicit start is given, look back far enough to cover typical
    // historical scenarios rather than only what is already synced locally.
    let start = start_date.unwrap_or_else(|| end - chrono::Duration::days(365 * 10));

    if quote_history_reaches(&local, start) {
        return (local, warnings);
    }

    match env
        .quote_service()
        .fetch_quotes_for_symbol(asset_id, currency.unwrap_or("USD"), start, end)
        .await
    {
        Ok(fetched) if !fetched.is_empty() => (merge_quotes_by_date(local, fetched), warnings),
        Ok(_) => {
            warnings.push(format!(
                "The market-data provider returned no history for '{asset_id}' over {start}..{end}; used local quotes only."
            ));
            (local, warnings)
        }
        Err(e) => {
            warnings.push(format!(
                "Could not fetch '{asset_id}' history from the market-data provider ({e}); used local quotes only."
            ));
            (local, warnings)
        }
    }
}

/// True when local history is non-empty and its earliest quote is at or before
/// `start` — i.e. local data already covers the requested window's start.
fn quote_history_reaches(quotes: &[Quote], start: NaiveDate) -> bool {
    quotes
        .iter()
        .map(|q| q.timestamp.date_naive())
        .min()
        .map(|earliest| earliest <= start)
        .unwrap_or(false)
}

/// Merge local and provider quotes by calendar date, preferring the provider's
/// value on overlap (fresher and split/dividend-adjusted).
fn merge_quotes_by_date(local: Vec<Quote>, fetched: Vec<Quote>) -> Vec<Quote> {
    let mut by_date: BTreeMap<NaiveDate, Quote> = BTreeMap::new();
    for quote in local {
        by_date.insert(quote.timestamp.date_naive(), quote);
    }
    for quote in fetched {
        by_date.insert(quote.timestamp.date_naive(), quote);
    }
    by_date.into_values().collect()
}

/// Public entry point for the command layer (server and Tauri): load a saved
/// scenario, validate it has a basket, parse the optional YYYY-MM-DD bounds,
/// and replay the basket as a buy-and-hold index. Uses adjusted close at full
/// density (no sampling) so the points line up with the daily series the
/// performance chart rebases everything onto.
pub async fn replay_scenario_performance(
    env: Arc<dyn AgentEnvironment>,
    scenario_id: &str,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<SymbolPerformanceOutput, AgentToolError> {
    let scenario = env
        .scenario_service()
        .get_scenario(scenario_id)
        .map_err(|e| AgentToolError::InvalidInput(format!("Failed to load scenario: {e}")))?;
    if scenario.kind != ScenarioKind::Basket || scenario.basket.is_empty() {
        return Err(AgentToolError::InvalidInput(
            "Scenario has no basket to replay.".to_string(),
        ));
    }
    let start = parse_optional_date(start_date, "startDate")?;
    let end = parse_optional_date(end_date, "endDate")?;
    Ok(compute_basket_performance(
        env,
        &scenario.basket,
        PriceBasis::AdjustedClose,
        start,
        end,
        usize::MAX,
    )
    .await)
}

/// Replay a synthetic weighted basket over history as a buy-and-hold index.
///
/// Method (currency-agnostic, no FX): each leg is rebased to 1.0 at a common
/// base date, and the basket index is the weight-normalized sum of leg ratios,
/// scaled to 100 at the base. Legs are priced with [`load_benchmark_quotes`]
/// (local-first + provider backfill), so any symbol — tracked or not, past or
/// present — can take part. Legs with no usable history are dropped and the
/// remaining weights renormalized, with a warning. The resulting index series
/// is run through the same metric path as a single symbol, so the output is a
/// [`SymbolPerformanceOutput`] labelled `Basket`.
pub(crate) async fn compute_basket_performance(
    env: Arc<dyn AgentEnvironment>,
    positions: &[BasketPosition],
    basis: PriceBasis,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    sample_points: usize,
) -> SymbolPerformanceOutput {
    let end = end_date.unwrap_or_else(|| Local::now().date_naive());
    let mut warnings = Vec::new();

    // Price each leg into a sorted (date, price) series within [start, end].
    let mut legs: Vec<(f64, Vec<(NaiveDate, f64)>)> = Vec::new();
    for position in positions {
        let treat_as_id = position.symbol.contains(':');
        let resolved = match resolve_symbol_input(env.clone(), &position.symbol, treat_as_id).await
        {
            Ok(resolved) => resolved,
            Err(e) => {
                warnings.push(format!("Skipped basket leg '{}' ({e}).", position.symbol));
                continue;
            }
        };
        let (quotes, fetch_warnings) = load_benchmark_quotes(
            env.clone(),
            &resolved.asset_id,
            resolved.currency.as_deref(),
            start_date,
            Some(end),
        )
        .await;
        warnings.extend(fetch_warnings);

        let mut series: Vec<(NaiveDate, f64)> =
            BTreeMap::from_iter(quotes.iter().filter_map(|quote| {
                let date = quote.timestamp.date_naive();
                if date > end || start_date.is_some_and(|start| date < start) {
                    return None;
                }
                price_for_quote(quote, basis).map(|price| (date, price))
            }))
            .into_iter()
            .collect();
        series.sort_by_key(|(date, _)| *date);

        if series.is_empty() {
            warnings.push(format!(
                "Dropped basket leg '{}': no usable price history in range.",
                resolved.symbol
            ));
            continue;
        }
        legs.push((position.weight, series));
    }

    if legs.is_empty() {
        warnings.push("Basket has no priceable legs.".to_string());
        return calculate_symbol_performance_from_quotes(
            "Basket".to_string(),
            "basket".to_string(),
            PriceBasis::Close,
            None,
            None,
            Vec::new(),
            sample_points,
            warnings,
        );
    }

    // Common base date: the latest "first available" across legs, clamped to
    // start_date when given — so every leg has a price at or before the base.
    let base_date = legs
        .iter()
        .map(|(_, series)| series[0].0)
        .max()
        .map(|latest_first| match start_date {
            Some(start) if start > latest_first => start,
            _ => latest_first,
        })
        .expect("legs is non-empty");

    let weight_sum: f64 = legs.iter().map(|(weight, _)| *weight).sum();
    if weight_sum <= 0.0 {
        warnings.push("Basket weights sum to zero.".to_string());
        return calculate_symbol_performance_from_quotes(
            "Basket".to_string(),
            "basket".to_string(),
            PriceBasis::Close,
            None,
            None,
            Vec::new(),
            sample_points,
            warnings,
        );
    }

    // Union of all leg dates within [base_date, end].
    let mut dates: Vec<NaiveDate> = legs
        .iter()
        .flat_map(|(_, series)| series.iter().map(|(date, _)| *date))
        .filter(|date| *date >= base_date)
        .collect();
    dates.sort_unstable();
    dates.dedup();

    let index: Vec<(NaiveDate, f64)> = dates
        .into_iter()
        .filter_map(|date| {
            let mut value = 0.0;
            for (weight, series) in &legs {
                let base_price = price_asof(series, base_date)?;
                let price = price_asof(series, date)?;
                if base_price <= 0.0 {
                    return None;
                }
                value += (weight / weight_sum) * (price / base_price);
            }
            Some((date, value * 100.0))
        })
        .collect();

    let synthetic: Vec<Quote> = index
        .into_iter()
        .map(|(date, value)| synthetic_index_quote(date, value))
        .collect();

    calculate_symbol_performance_from_quotes(
        "Basket".to_string(),
        "basket".to_string(),
        PriceBasis::Close,
        None,
        None,
        synthetic,
        sample_points,
        warnings,
    )
}

/// Last price at or before `target` in an ascending `(date, price)` series.
fn price_asof(series: &[(NaiveDate, f64)], target: NaiveDate) -> Option<f64> {
    let idx = series.partition_point(|(date, _)| *date <= target);
    (idx > 0).then(|| series[idx - 1].1)
}

/// Build a synthetic daily quote carrying the basket index value as its price.
fn synthetic_index_quote(date: NaiveDate, value: f64) -> Quote {
    let price = Decimal::from_f64_retain(value).unwrap_or_default();
    Quote {
        id: format!("basket-{date}"),
        asset_id: "basket".to_string(),
        timestamp: date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(chrono::Utc)
            .unwrap(),
        open: price,
        high: price,
        low: price,
        close: price,
        adjclose: price,
        volume: Decimal::ZERO,
        currency: "INDEX".to_string(),
        data_source: "BASKET".to_string(),
        created_at: chrono::Utc::now(),
        notes: None,
    }
}

pub(crate) fn parse_optional_date(
    value: Option<&str>,
    field: &str,
) -> Result<Option<NaiveDate>, AgentToolError> {
    value
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").map_err(|_| {
                AgentToolError::InvalidInput(format!("{field} must be a date in YYYY-MM-DD format"))
            })
        })
        .transpose()
}

pub(crate) fn validate_date_order(
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
) -> Result<(), AgentToolError> {
    if let (Some(start), Some(end)) = (start_date, end_date) {
        if start > end {
            return Err(AgentToolError::InvalidInput(
                "startDate must be on or before endDate".to_string(),
            ));
        }
    }
    Ok(())
}

fn price_for_quote(quote: &Quote, basis: PriceBasis) -> Option<f64> {
    let value = match basis {
        PriceBasis::AdjustedClose => {
            if quote.adjclose > Decimal::ZERO {
                quote.adjclose
            } else {
                quote.close
            }
        }
        PriceBasis::Close => quote.close,
    };
    (value > Decimal::ZERO).then(|| value.to_f64()).flatten()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn calculate_symbol_performance_from_quotes(
    symbol: String,
    asset_id: String,
    basis: PriceBasis,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    quotes: Vec<Quote>,
    sample_points: usize,
    mut warnings: Vec<String>,
) -> SymbolPerformanceOutput {
    let mut ignored_non_positive = 0usize;
    let mut by_date: BTreeMap<NaiveDate, PricePoint> = BTreeMap::new();

    for quote in quotes {
        let date = quote.timestamp.date_naive();
        if let Some(start) = start_date {
            if date < start {
                continue;
            }
        }
        if let Some(end) = end_date {
            if date > end {
                continue;
            }
        }

        match price_for_quote(&quote, basis) {
            Some(price) => {
                by_date.insert(
                    date,
                    PricePoint {
                        date,
                        price,
                        currency: quote.currency,
                    },
                );
            }
            None => ignored_non_positive += 1,
        }
    }

    if ignored_non_positive > 0 {
        warnings.push(format!(
            "Ignored {ignored_non_positive} quote rows with non-positive prices."
        ));
    }

    let points: Vec<PricePoint> = by_date.into_values().collect();
    let quote_count = points.len();

    if quote_count == 0 {
        warnings.push(
            "No local quote history was found for the requested symbol and date range.".to_string(),
        );
        return SymbolPerformanceOutput {
            symbol,
            asset_id,
            basis: basis.as_str().to_string(),
            currency: None,
            requested_start_date: start_date.map(|d| d.to_string()),
            requested_end_date: end_date.map(|d| d.to_string()),
            first_quote_date: None,
            last_quote_date: None,
            quote_count,
            metrics: SymbolPerformanceMetricsOutput::default(),
            data_quality: SymbolPerformanceDataQualityOutput {
                status: "noData".to_string(),
                warnings,
            },
            points: Vec::new(),
            truncated: None,
            original_point_count: None,
        };
    }

    let first = points.first().expect("quote_count > 0");
    let last = points.last().expect("quote_count > 0");
    if let Some(requested) = start_date {
        if first.date > requested {
            warnings.push(format!(
                "First available quote is {}, after requested startDate {}.",
                first.date, requested
            ));
        }
    }
    if let Some(requested) = end_date {
        if last.date < requested {
            warnings.push(format!(
                "Last available quote is {}, before requested endDate {}.",
                last.date, requested
            ));
        }
    }
    if quote_count < 2 {
        warnings.push("At least two quotes are required to compute returns.".to_string());
    }

    let metrics = calculate_price_metrics(&points);
    let status = if quote_count < 2 || !warnings.is_empty() {
        "partial"
    } else {
        "ok"
    };

    let original_point_count = points.len();
    let sampled_points = sample_price_points(&points, sample_points);
    let truncated = original_point_count > sampled_points.len();

    SymbolPerformanceOutput {
        symbol,
        asset_id,
        basis: basis.as_str().to_string(),
        currency: Some(last.currency.clone()),
        requested_start_date: start_date.map(|d| d.to_string()),
        requested_end_date: end_date.map(|d| d.to_string()),
        first_quote_date: Some(first.date.to_string()),
        last_quote_date: Some(last.date.to_string()),
        quote_count,
        metrics,
        data_quality: SymbolPerformanceDataQualityOutput {
            status: status.to_string(),
            warnings,
        },
        points: sampled_points,
        truncated: truncated.then_some(true),
        original_point_count: truncated.then_some(original_point_count),
    }
}

fn calculate_price_metrics(points: &[PricePoint]) -> SymbolPerformanceMetricsOutput {
    if points.is_empty() {
        return SymbolPerformanceMetricsOutput::default();
    }

    let first = &points[0];
    let last = &points[points.len() - 1];
    let total_return = if points.len() >= 2 {
        Some(last.price / first.price - 1.0)
    } else {
        None
    };
    let days = (last.date - first.date).num_days();
    let annualized_return = total_return
        .and_then(|value| (days > 0).then(|| ((1.0 + value).powf(365.25 / days as f64)) - 1.0));

    let mut daily_returns = Vec::new();
    for window in points.windows(2) {
        let previous = window[0].price;
        let current = window[1].price;
        if previous > 0.0 {
            daily_returns.push(current / previous - 1.0);
        }
    }

    let best_day_return = daily_returns
        .iter()
        .copied()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let worst_day_return = daily_returns
        .iter()
        .copied()
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let volatility = if daily_returns.len() >= 2 {
        Some(std_dev(&daily_returns) * 252.0_f64.sqrt())
    } else {
        None
    };

    SymbolPerformanceMetricsOutput {
        start_price: Some(first.price),
        end_price: Some(last.price),
        total_return_pct: total_return.map(rate_to_pct),
        annualized_return_pct: annualized_return.map(rate_to_pct),
        volatility_pct: volatility.map(rate_to_pct),
        max_drawdown_pct: max_drawdown(points).map(rate_to_pct),
        best_day_return_pct: best_day_return.map(rate_to_pct),
        worst_day_return_pct: worst_day_return.map(rate_to_pct),
    }
}

fn std_dev(values: &[f64]) -> f64 {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>()
        / (values.len() - 1) as f64;
    variance.sqrt()
}

fn max_drawdown(points: &[PricePoint]) -> Option<f64> {
    if points.is_empty() {
        return None;
    }
    let mut peak = points[0].price;
    let mut worst = 0.0;
    for point in points {
        if point.price > peak {
            peak = point.price;
        }
        if peak > 0.0 {
            let drawdown = point.price / peak - 1.0;
            if drawdown < worst {
                worst = drawdown;
            }
        }
    }
    Some(worst)
}

fn sample_price_points(
    points: &[PricePoint],
    max_points: usize,
) -> Vec<SymbolPerformancePointOutput> {
    if max_points == 0 || points.is_empty() {
        return Vec::new();
    }

    let selected: Vec<&PricePoint> = if points.len() <= max_points {
        points.iter().collect()
    } else if max_points == 1 {
        vec![points.last().expect("non-empty points")]
    } else {
        let last_index = points.len() - 1;
        let mut out = Vec::with_capacity(max_points);
        let mut last_selected = None;
        for i in 0..max_points {
            let idx = ((i as f64 * last_index as f64) / (max_points - 1) as f64).round() as usize;
            if Some(idx) != last_selected {
                out.push(&points[idx]);
                last_selected = Some(idx);
            }
        }
        out
    };

    let start_price = points[0].price;
    let mut peak = points[0].price;
    let mut sampled = Vec::with_capacity(selected.len());
    for point in selected {
        if point.price > peak {
            peak = point.price;
        }
        let return_pct = (start_price > 0.0).then(|| rate_to_pct(point.price / start_price - 1.0));
        let drawdown_pct = (peak > 0.0).then(|| rate_to_pct(point.price / peak - 1.0));
        sampled.push(SymbolPerformancePointOutput {
            date: point.date.to_string(),
            price: point.price,
            return_pct,
            drawdown_pct,
        });
    }
    sampled
}

pub(crate) async fn calculate_portfolio_comparison(
    env: Arc<dyn AgentEnvironment>,
    account_id: Option<&str>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
) -> Result<PortfolioComparisonOutput, AgentToolError> {
    let base_currency = env.base_currency();
    let account_id = account_id.filter(|id| !id.trim().is_empty());
    let end_date = end_date.unwrap_or_else(|| Local::now().date_naive());

    let metrics = if let Some(account_id) = account_id {
        let account = env
            .account_service()
            .get_account(account_id)
            .map_err(|e| AgentToolError::ExecutionFailed(e.to_string()))?;
        if !account_supports_portfolio_scope(&account, AccountPurpose::Performance) {
            return Ok(PortfolioComparisonOutput {
                id: account_id.to_string(),
                period_start_date: start_date.map(|d| d.to_string()),
                period_end_date: Some(end_date.to_string()),
                currency: account.currency,
                mode: "notApplicable".to_string(),
                basis_status: "notApplicable".to_string(),
                data_quality_status: "noData".to_string(),
                warnings: vec!["Performance unavailable for this account type.".to_string()],
                ..Default::default()
            });
        }
        env.performance_service()
            .calculate_performance_history(
                "account",
                account_id,
                start_date,
                Some(end_date),
                Some(account.tracking_mode),
                Some(&account.account_type),
            )
            .await
            .map_err(|e| AgentToolError::ExecutionFailed(e.to_string()))?
    } else {
        let accounts = env
            .account_service()
            .get_non_archived_accounts()
            .map_err(|e| AgentToolError::ExecutionFailed(e.to_string()))?;
        let mut account_tracking_modes = std::collections::HashMap::new();
        let mut account_types = std::collections::HashMap::new();
        let account_ids: Vec<String> = accounts
            .into_iter()
            .filter(|account| {
                account_supports_portfolio_scope(account, AccountPurpose::Performance)
            })
            .map(|account| {
                account_tracking_modes.insert(account.id.clone(), account.tracking_mode);
                account_types.insert(account.id.clone(), account.account_type.clone());
                account.id
            })
            .collect();
        env.performance_service()
            .calculate_performance_history_for_accounts(
                "all",
                &account_ids,
                &base_currency,
                &account_tracking_modes,
                &account_types,
                start_date,
                Some(end_date),
            )
            .await
            .map_err(|e| AgentToolError::ExecutionFailed(e.to_string()))?
    };

    Ok(portfolio_comparison_from_metrics(metrics, base_currency))
}

pub(crate) async fn calculate_portfolio_comparison_for_account_ids(
    env: Arc<dyn AgentEnvironment>,
    scope_id: &str,
    account_ids: &[String],
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
) -> Result<PortfolioComparisonOutput, AgentToolError> {
    let base_currency = env.base_currency();
    let end_date = end_date.unwrap_or_else(|| Local::now().date_naive());
    let unique_requested = unique_account_ids(account_ids.iter().cloned());

    if unique_requested.is_empty() {
        let metrics =
            empty_performance_metrics(scope_id, base_currency.clone(), start_date, Some(end_date));
        return Ok(portfolio_comparison_from_metrics(metrics, base_currency));
    }

    let accounts_by_id: HashMap<String, Account> = env
        .account_service()
        .get_accounts_by_ids(&unique_requested)
        .map_err(|e| AgentToolError::ExecutionFailed(e.to_string()))?
        .into_iter()
        .map(|account| (account.id.clone(), account))
        .collect();
    let eligible_account_ids = performance_account_ids_from_map(&accounts_by_id, &unique_requested);

    if eligible_account_ids.is_empty() {
        let mut metrics =
            empty_performance_metrics(scope_id, base_currency.clone(), start_date, Some(end_date));
        metrics.data_quality.warnings.push(
            "Requested scenario accounts were excluded because they are archived or not eligible for performance."
                .to_string(),
        );
        metrics.data_quality.status = DataQualityStatus::Partial;
        sync_performance_summary_quality(&mut metrics);
        return Ok(portfolio_comparison_from_metrics(metrics, base_currency));
    }

    let account_tracking_modes =
        performance_account_tracking_modes_from_map(&accounts_by_id, &eligible_account_ids);
    let account_types = performance_account_types_from_map(&accounts_by_id, &eligible_account_ids);
    let mut metrics = env
        .performance_service()
        .calculate_performance_history_for_accounts(
            scope_id,
            &eligible_account_ids,
            &base_currency,
            &account_tracking_modes,
            &account_types,
            start_date,
            Some(end_date),
        )
        .await
        .map_err(|e| AgentToolError::ExecutionFailed(e.to_string()))?;

    if eligible_account_ids.len() != unique_requested.len() {
        metrics.data_quality.warnings.push(
            "Some requested scenario accounts were excluded because they are archived, missing, or not eligible for performance."
                .to_string(),
        );
        metrics.data_quality.status = DataQualityStatus::Partial;
        sync_performance_summary_quality(&mut metrics);
    }

    Ok(portfolio_comparison_from_metrics(metrics, base_currency))
}

fn portfolio_comparison_from_metrics(
    metrics: CorePerformanceResult,
    base_currency: String,
) -> PortfolioComparisonOutput {
    PortfolioComparisonOutput {
        id: metrics.scope.id,
        period_start_date: metrics.period.start_date.map(|d| d.to_string()),
        period_end_date: metrics.period.end_date.map(|d| d.to_string()),
        currency: if metrics.scope.currency.is_empty() {
            base_currency
        } else {
            metrics.scope.currency
        },
        mode: serialized_value(&metrics.mode, "notApplicable"),
        basis_status: serialized_value(&metrics.basis_status, "notApplicable"),
        twr_pct: metrics.returns.twr.and_then(decimal_rate_to_pct),
        annualized_twr_pct: metrics.returns.annualized_twr.and_then(decimal_rate_to_pct),
        irr_pct: metrics.returns.irr.and_then(decimal_rate_to_pct),
        annualized_irr_pct: metrics.returns.annualized_irr.and_then(decimal_rate_to_pct),
        value_return_pct: metrics.returns.value_return.and_then(decimal_rate_to_pct),
        annualized_value_return_pct: metrics
            .returns
            .annualized_value_return
            .and_then(decimal_rate_to_pct),
        summary_amount: metrics.summary.amount.and_then(|v| v.to_f64()),
        summary_percent_pct: metrics.summary.percent.and_then(decimal_rate_to_pct),
        data_quality_status: serialized_value(&metrics.data_quality.status, "partial"),
        warnings: metrics.data_quality.warnings,
    }
}

fn serialized_value<T: Serialize>(value: &T, fallback: &str) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToString::to_string))
        .unwrap_or_else(|| fallback.to_string())
}

fn decimal_rate_to_pct(value: Decimal) -> Option<f64> {
    value.to_f64().map(rate_to_pct)
}

fn rate_to_pct(value: f64) -> f64 {
    value * 100.0
}

fn period_to_start_date(period: &str, end_date: NaiveDate) -> Option<NaiveDate> {
    match period.to_uppercase().as_str() {
        "1M" => Some(end_date - chrono::Duration::days(30)),
        "3M" => Some(end_date - chrono::Duration::days(90)),
        "6M" => Some(end_date - chrono::Duration::days(180)),
        "YTD" => NaiveDate::from_ymd_opt(end_date.year(), 1, 1),
        "1Y" => Some(end_date - chrono::Duration::days(365)),
        "ALL" => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn quote(asset_id: &str, day: &str, close: i64) -> Quote {
        let date = NaiveDate::parse_from_str(day, "%Y-%m-%d").unwrap();
        let decimal = Decimal::from(close);
        Quote {
            id: format!("{asset_id}-{day}"),
            asset_id: asset_id.to_string(),
            timestamp: Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap()),
            open: decimal,
            high: decimal,
            low: decimal,
            close: decimal,
            adjclose: decimal,
            volume: Decimal::ZERO,
            currency: "USD".to_string(),
            data_source: "TEST".to_string(),
            created_at: Utc::now(),
            notes: None,
        }
    }

    fn day(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn history_reaches_only_when_earliest_covers_start() {
        let q = vec![quote("X", "2024-03-01", 10), quote("X", "2024-03-05", 11)];
        assert!(quote_history_reaches(&q, day("2024-03-02")));
        assert!(quote_history_reaches(&q, day("2024-03-01")));
        // Local history starts after the requested start -> not covered.
        assert!(!quote_history_reaches(&q, day("2024-02-01")));
        // Empty history never covers.
        assert!(!quote_history_reaches(&[], day("2024-03-01")));
    }

    #[test]
    fn price_asof_returns_last_on_or_before() {
        let series = vec![(day("2024-01-01"), 10.0), (day("2024-01-03"), 12.0)];
        assert_eq!(price_asof(&series, day("2024-01-02")), Some(10.0));
        assert_eq!(price_asof(&series, day("2024-01-03")), Some(12.0));
        // Target before the first point has no as-of price.
        assert_eq!(price_asof(&series, day("2023-12-31")), None);
        assert_eq!(price_asof(&[], day("2024-01-01")), None);
    }

    #[test]
    fn merge_prefers_provider_on_overlap_and_unions_dates() {
        let local = vec![quote("X", "2024-01-01", 100), quote("X", "2024-01-02", 100)];
        let fetched = vec![quote("X", "2024-01-02", 200), quote("X", "2024-01-03", 300)];
        let merged = merge_quotes_by_date(local, fetched);
        assert_eq!(merged.len(), 3); // 01,02,03 unioned
                                     // Sorted by date; overlapping 01-02 takes the provider's 200.
        assert_eq!(merged[0].timestamp.date_naive(), day("2024-01-01"));
        assert_eq!(merged[1].close, Decimal::from(200));
        assert_eq!(merged[2].close, Decimal::from(300));
    }

    #[test]
    fn symbol_performance_computes_return_and_drawdown() {
        let output = calculate_symbol_performance_from_quotes(
            "SPY".to_string(),
            "SEC:SPY:ARCX".to_string(),
            PriceBasis::AdjustedClose,
            None,
            None,
            vec![
                quote("SEC:SPY:ARCX", "2024-01-01", 100),
                quote("SEC:SPY:ARCX", "2024-01-02", 120),
                quote("SEC:SPY:ARCX", "2024-01-03", 90),
                quote("SEC:SPY:ARCX", "2024-01-04", 150),
            ],
            10,
            Vec::new(),
        );

        assert_eq!(output.data_quality.status, "ok");
        assert_eq!(output.quote_count, 4);
        assert_eq!(output.metrics.total_return_pct, Some(50.0));
        assert_eq!(output.metrics.max_drawdown_pct, Some(-25.0));
        assert_eq!(output.points.len(), 4);
    }

    #[test]
    fn symbol_performance_bounds_sample_points() {
        let quotes = (1..=10)
            .map(|day| quote("SEC:AAA:XNAS", &format!("2024-01-{day:02}"), 100 + day))
            .collect();

        let output = calculate_symbol_performance_from_quotes(
            "AAA".to_string(),
            "SEC:AAA:XNAS".to_string(),
            PriceBasis::Close,
            None,
            None,
            quotes,
            3,
            Vec::new(),
        );

        assert_eq!(output.points.len(), 3);
        assert_eq!(output.truncated, Some(true));
        assert_eq!(output.original_point_count, Some(10));
        assert_eq!(output.points.first().unwrap().date, "2024-01-01");
        assert_eq!(output.points.last().unwrap().date, "2024-01-10");
    }

    #[test]
    fn symbol_performance_reports_no_data() {
        let output = calculate_symbol_performance_from_quotes(
            "MISSING".to_string(),
            "MISSING".to_string(),
            PriceBasis::AdjustedClose,
            None,
            None,
            Vec::new(),
            10,
            Vec::new(),
        );

        assert_eq!(output.data_quality.status, "noData");
        assert!(output.metrics.total_return_pct.is_none());
        assert!(output.points.is_empty());
    }
}
