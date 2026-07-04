//! Financial Modeling Prep (FMP) market data provider implementation.
//!
//! This module provides market data from the FMP "stable" API
//! (https://financialmodelingprep.com/stable/):
//! - Latest quotes via /quote
//! - Historical EOD prices via /historical-price-eod/full
//! - Symbol search via /search-symbol and /search-name
//! - Company profiles via /profile
//! - Dividends via /dividends, splits via /splits
//! - Stock screening via /company-screener
//!
//! Note: the FMP free tier is limited to ~250 API calls per day.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use log::{debug, warn};
use reqwest::Client;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::Deserialize;

use std::time::Duration;

use crate::errors::MarketDataError;
use crate::models::{
    AssetProfile, Coverage, DividendEvent, InstrumentKind, ProviderInstrument, Quote, QuoteContext,
    ScreenerHit, ScreenerQuery, SearchResult, SplitEvent,
};
use crate::provider::{MarketDataProvider, ProviderCapabilities, RateLimit};
use crate::resolver::ResolverChain;
use crate::SymbolResolver;

const BASE_URL: &str = "https://financialmodelingprep.com/stable";
const PROVIDER_ID: &str = "FMP";

/// Default number of screener results when the query doesn't specify a limit.
const DEFAULT_SCREENER_LIMIT: u32 = 50;

/// Financial Modeling Prep market data provider.
///
/// Supports US and global equities/ETFs. Free tier is limited to
/// ~250 API calls per day, so rate limits are conservative.
pub struct FmpProvider {
    client: Client,
    api_key: String,
}

// ============================================================================
// Response structures for the FMP stable API
// ============================================================================

/// Element of the /quote response array.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FmpQuote {
    #[allow(dead_code)]
    symbol: Option<String>,
    price: Option<f64>,
    open: Option<f64>,
    day_high: Option<f64>,
    day_low: Option<f64>,
    volume: Option<f64>,
    /// Unix timestamp (seconds)
    timestamp: Option<i64>,
}

/// Element of the /historical-price-eod/full response array.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FmpHistoricalBar {
    date: Option<String>,
    open: Option<f64>,
    high: Option<f64>,
    low: Option<f64>,
    close: Option<f64>,
    volume: Option<f64>,
}

/// The stable API returns a flat array; older shapes wrap bars in
/// `{"symbol": ..., "historical": [...]}`. Accept both defensively.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FmpHistoricalResponse {
    Flat(Vec<FmpHistoricalBar>),
    Wrapped {
        #[serde(default)]
        historical: Vec<FmpHistoricalBar>,
    },
}

impl FmpHistoricalResponse {
    fn into_bars(self) -> Vec<FmpHistoricalBar> {
        match self {
            Self::Flat(bars) => bars,
            Self::Wrapped { historical } => historical,
        }
    }
}

/// Element of the /search-symbol and /search-name response arrays.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FmpSearchItem {
    symbol: String,
    name: Option<String>,
    currency: Option<String>,
    exchange: Option<String>,
    exchange_full_name: Option<String>,
}

/// Element of the /profile response array.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FmpProfile {
    #[allow(dead_code)]
    symbol: Option<String>,
    company_name: Option<String>,
    sector: Option<String>,
    industry: Option<String>,
    website: Option<String>,
    description: Option<String>,
    country: Option<String>,
    full_time_employees: Option<serde_json::Value>,
    image: Option<String>,
    market_cap: Option<f64>,
    last_dividend: Option<f64>,
    price: Option<f64>,
    /// 52-week range as "low-high" string (e.g., "164.08-260.10")
    range: Option<String>,
    isin: Option<String>,
    is_etf: Option<bool>,
    is_fund: Option<bool>,
}

impl FmpProfile {
    fn parse_employees(&self) -> Option<u64> {
        match self.full_time_employees.as_ref()? {
            serde_json::Value::Number(n) => n.as_u64(),
            serde_json::Value::String(s) => s.replace(',', "").trim().parse::<u64>().ok(),
            _ => None,
        }
    }

    /// Parse the "low-high" 52-week range string.
    fn parse_range(&self) -> (Option<f64>, Option<f64>) {
        let Some(range) = self.range.as_deref() else {
            return (None, None);
        };
        let Some((low, high)) = range.split_once('-') else {
            return (None, None);
        };
        (
            low.trim().parse::<f64>().ok(),
            high.trim().parse::<f64>().ok(),
        )
    }

    fn quote_type(&self) -> Option<String> {
        if self.is_etf == Some(true) {
            Some("ETF".to_string())
        } else if self.is_fund == Some(true) {
            Some("MUTUALFUND".to_string())
        } else {
            Some("EQUITY".to_string())
        }
    }

    fn to_asset_profile(&self) -> AssetProfile {
        let (week_52_low, week_52_high) = self.parse_range();
        // lastDividend is per-share; convert to yield when price is known.
        let dividend_yield = match (self.last_dividend, self.price) {
            (Some(d), Some(p)) if d > 0.0 && p > 0.0 => Some(d / p),
            _ => None,
        };

        AssetProfile {
            source: Some(PROVIDER_ID.to_string()),
            name: self.company_name.clone(),
            quote_type: self.quote_type(),
            sector: self.sector.clone().filter(|s| !s.is_empty()),
            sectors: None,
            industry: self.industry.clone().filter(|s| !s.is_empty()),
            website: self.website.clone(),
            description: self.description.clone(),
            country: self.country.clone(),
            employees: self.parse_employees(),
            logo_url: self.image.clone(),
            market_cap: self.market_cap.filter(|v| *v > 0.0),
            pe_ratio: None,
            dividend_yield,
            week_52_high,
            week_52_low,
            isin: self.isin.clone(),
        }
    }
}

/// Element of the /dividends response array.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FmpDividend {
    /// Ex-dividend date (YYYY-MM-DD)
    date: Option<String>,
    dividend: Option<f64>,
    adj_dividend: Option<f64>,
}

/// Element of the /splits response array.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FmpSplit {
    date: Option<String>,
    numerator: Option<f64>,
    denominator: Option<f64>,
}

/// Element of the /company-screener response array.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FmpScreenerItem {
    symbol: Option<String>,
    company_name: Option<String>,
    market_cap: Option<f64>,
    sector: Option<String>,
    industry: Option<String>,
    price: Option<f64>,
    exchange_short_name: Option<String>,
    exchange: Option<String>,
    country: Option<String>,
}

impl FmpScreenerItem {
    fn into_hit(self) -> Option<ScreenerHit> {
        Some(ScreenerHit {
            symbol: self.symbol?,
            name: self.company_name,
            market_cap: self.market_cap,
            sector: self.sector,
            industry: self.industry,
            price: self.price,
            exchange: self.exchange_short_name.or(self.exchange),
            country: self.country,
        })
    }
}

/// Error body sometimes returned with HTTP 200 (e.g., invalid API key).
#[derive(Debug, Deserialize)]
struct FmpErrorBody {
    #[serde(rename = "Error Message")]
    error_message: Option<String>,
}

// ============================================================================
// FmpProvider implementation
// ============================================================================

impl FmpProvider {
    /// Create a new FMP provider with the given API key.
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client, api_key }
    }

    /// Make a request to the FMP API and return the response body.
    async fn fetch(&self, path: &str, params: &[(&str, &str)]) -> Result<String, MarketDataError> {
        let mut all_params: Vec<(&str, &str)> = params.to_vec();
        all_params.push(("apikey", &self.api_key));

        let url = reqwest::Url::parse_with_params(&format!("{}/{}", BASE_URL, path), &all_params)
            .map_err(|e| MarketDataError::ProviderError {
            provider: PROVIDER_ID.to_string(),
            message: format!("Failed to build URL: {}", e),
        })?;

        debug!(
            "FMP request: {}",
            url.as_str().replace(&self.api_key, "***")
        );

        let response = self.client.get(url).send().await.map_err(|e| {
            if e.is_timeout() {
                MarketDataError::Timeout {
                    provider: PROVIDER_ID.to_string(),
                }
            } else {
                MarketDataError::ProviderError {
                    provider: PROVIDER_ID.to_string(),
                    message: e.to_string(),
                }
            }
        })?;

        let status = response.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(MarketDataError::RateLimited {
                provider: PROVIDER_ID.to_string(),
            });
        }

        if !status.is_success() {
            return Err(MarketDataError::ProviderError {
                provider: PROVIDER_ID.to_string(),
                message: format!("HTTP {}", status),
            });
        }

        let text = response
            .text()
            .await
            .map_err(|e| MarketDataError::ProviderError {
                provider: PROVIDER_ID.to_string(),
                message: e.to_string(),
            })?;

        Self::check_error_body(&text)?;

        Ok(text)
    }

    /// FMP sometimes returns `{"Error Message": "..."}` with HTTP 200.
    fn check_error_body(text: &str) -> Result<(), MarketDataError> {
        if let Ok(body) = serde_json::from_str::<FmpErrorBody>(text) {
            if let Some(msg) = body.error_message {
                if msg.to_lowercase().contains("limit") {
                    return Err(MarketDataError::RateLimited {
                        provider: PROVIDER_ID.to_string(),
                    });
                }
                return Err(MarketDataError::ProviderError {
                    provider: PROVIDER_ID.to_string(),
                    message: msg,
                });
            }
        }
        Ok(())
    }

    /// Parse a JSON response, mapping deserialization failure to a provider error.
    fn parse<T: serde::de::DeserializeOwned>(text: &str, what: &str) -> Result<T, MarketDataError> {
        serde_json::from_str(text).map_err(|e| MarketDataError::ProviderError {
            provider: PROVIDER_ID.to_string(),
            message: format!("Failed to parse {} response: {}", what, e),
        })
    }

    /// Get the currency: prefer exchange metadata, fall back to asset's quote_ccy.
    fn resolve_currency(&self, context: &QuoteContext) -> String {
        let chain = ResolverChain::new();
        chain
            .get_currency(&PROVIDER_ID.into(), context)
            .or_else(|| context.currency_hint.clone())
            .map(|c| c.to_string())
            .unwrap_or_else(|| "USD".to_string())
    }

    /// Extract the equity symbol from a provider instrument.
    fn equity_symbol(instrument: &ProviderInstrument) -> Result<String, MarketDataError> {
        match instrument {
            ProviderInstrument::EquitySymbol { symbol } => Ok(symbol.to_string()),
            other => Err(MarketDataError::UnsupportedAssetType(format!(
                "FMP only supports equity symbols, got: {:?}",
                other
            ))),
        }
    }

    /// Parse a date string in YYYY-MM-DD format to DateTime<Utc>.
    fn parse_date(date_str: &str) -> Option<DateTime<Utc>> {
        NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .and_then(|dt| Utc.from_local_datetime(&dt).single())
    }

    fn to_decimal(value: f64) -> Option<Decimal> {
        Decimal::from_f64(value)
    }

    /// Build the query string parameters for the /company-screener endpoint.
    fn build_screener_params(query: &ScreenerQuery) -> Vec<(String, String)> {
        let mut params: Vec<(String, String)> = Vec::new();

        let mut push_f64 = |key: &str, value: Option<f64>| {
            if let Some(v) = value {
                params.push((key.to_string(), v.to_string()));
            }
        };
        push_f64("marketCapMoreThan", query.market_cap_min);
        push_f64("marketCapLowerThan", query.market_cap_max);
        push_f64("priceMoreThan", query.price_min);
        push_f64("priceLowerThan", query.price_max);
        push_f64("betaMoreThan", query.beta_min);
        push_f64("betaLowerThan", query.beta_max);
        push_f64("dividendMoreThan", query.dividend_min);
        push_f64("volumeMoreThan", query.volume_min);

        if let Some(sector) = query.sector.as_deref().filter(|s| !s.is_empty()) {
            params.push(("sector".to_string(), sector.to_string()));
        }
        if let Some(industry) = query.industry.as_deref().filter(|s| !s.is_empty()) {
            params.push(("industry".to_string(), industry.to_string()));
        }
        if let Some(exchange) = query.exchange.as_deref().filter(|s| !s.is_empty()) {
            params.push(("exchange".to_string(), exchange.to_string()));
        }
        if let Some(country) = query.country.as_deref().filter(|s| !s.is_empty()) {
            params.push(("country".to_string(), country.to_string()));
        }
        if let Some(is_etf) = query.is_etf {
            params.push(("isEtf".to_string(), is_etf.to_string()));
        }
        if let Some(active) = query.is_actively_trading {
            params.push(("isActivelyTrading".to_string(), active.to_string()));
        }

        let limit = query.limit.unwrap_or(DEFAULT_SCREENER_LIMIT);
        params.push(("limit".to_string(), limit.to_string()));

        params
    }

    /// Convert a historical bar to a Quote.
    fn bar_to_quote(bar: FmpHistoricalBar, currency: &str) -> Option<Quote> {
        let timestamp = Self::parse_date(bar.date.as_deref()?)?;
        let close = Self::to_decimal(bar.close?)?;

        Some(Quote {
            timestamp,
            open: bar.open.and_then(Self::to_decimal),
            high: bar.high.and_then(Self::to_decimal),
            low: bar.low.and_then(Self::to_decimal),
            close,
            volume: bar.volume.and_then(Self::to_decimal),
            currency: currency.to_string(),
            source: PROVIDER_ID.to_string(),
        })
    }

    /// Fetch the latest quote using the /quote endpoint.
    async fn fetch_latest(&self, symbol: &str, currency: &str) -> Result<Quote, MarketDataError> {
        let text = self.fetch("quote", &[("symbol", symbol)]).await?;
        let quotes: Vec<FmpQuote> = Self::parse(&text, "quote")?;

        let quote = quotes
            .into_iter()
            .next()
            .ok_or_else(|| MarketDataError::SymbolNotFound(format!("No data for: {}", symbol)))?;

        let close = quote.price.and_then(Self::to_decimal).ok_or_else(|| {
            MarketDataError::ProviderError {
                provider: PROVIDER_ID.to_string(),
                message: format!("No price data for: {}", symbol),
            }
        })?;

        let timestamp = quote
            .timestamp
            .and_then(|t| Utc.timestamp_opt(t, 0).single())
            .unwrap_or_else(Utc::now);

        Ok(Quote {
            timestamp,
            open: quote.open.and_then(Self::to_decimal),
            high: quote.day_high.and_then(Self::to_decimal),
            low: quote.day_low.and_then(Self::to_decimal),
            close,
            volume: quote.volume.and_then(Self::to_decimal),
            currency: currency.to_string(),
            source: PROVIDER_ID.to_string(),
        })
    }

    /// Fetch historical EOD quotes using /historical-price-eod/full.
    async fn fetch_historical(
        &self,
        symbol: &str,
        currency: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Quote>, MarketDataError> {
        let from = start.format("%Y-%m-%d").to_string();
        let to = end.format("%Y-%m-%d").to_string();
        let text = self
            .fetch(
                "historical-price-eod/full",
                &[("symbol", symbol), ("from", &from), ("to", &to)],
            )
            .await?;

        let response: FmpHistoricalResponse = Self::parse(&text, "historical")?;

        let mut quotes: Vec<Quote> = response
            .into_bars()
            .into_iter()
            .filter_map(|bar| Self::bar_to_quote(bar, currency))
            .filter(|q| q.timestamp >= start && q.timestamp <= end)
            .collect();

        quotes.sort_by_key(|q| q.timestamp);

        if quotes.is_empty() {
            return Err(MarketDataError::NoDataForRange);
        }

        debug!("FMP: fetched {} quotes for {}", quotes.len(), symbol);
        Ok(quotes)
    }

    /// Search a single endpoint (/search-symbol or /search-name).
    async fn search_endpoint(
        &self,
        endpoint: &str,
        query: &str,
    ) -> Result<Vec<FmpSearchItem>, MarketDataError> {
        let text = self.fetch(endpoint, &[("query", query)]).await?;
        Self::parse(&text, endpoint)
    }

    fn search_item_to_result(item: FmpSearchItem) -> SearchResult {
        let exchange = item
            .exchange
            .or(item.exchange_full_name)
            .unwrap_or_default();
        let name = item.name.unwrap_or_else(|| item.symbol.clone());

        let mut result = SearchResult::new(&item.symbol, &name, &exchange, "EQUITY")
            .with_data_source(PROVIDER_ID);
        if let Some(currency) = item.currency {
            result = result.with_currency(currency);
        }
        result
    }
}

// ============================================================================
// MarketDataProvider trait implementation
// ============================================================================

#[async_trait]
impl MarketDataProvider for FmpProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    fn priority(&self) -> u8 {
        // Below Yahoo (1) and Finnhub (2), comparable to Alpha Vantage (3)
        // due to the restrictive free-tier daily quota.
        3
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            instrument_kinds: &[InstrumentKind::Equity],
            // Best-effort: accepts instruments without MIC codes
            coverage: Coverage::global_best_effort(),
            supports_latest: true,
            supports_historical: true,
            supports_search: true,    // Via /search-symbol + /search-name
            supports_profile: true,   // Via /profile
            supports_dividends: true, // Via /dividends
            supports_screener: true,  // Via /company-screener
        }
    }

    fn rate_limit(&self) -> RateLimit {
        RateLimit {
            requests_per_minute: 10,           // Free tier: ~250 requests/day
            max_concurrency: 1,                // Sequential requests only
            min_delay: Duration::from_secs(2), // Conservative pacing
        }
    }

    async fn get_latest_quote(
        &self,
        context: &QuoteContext,
        instrument: ProviderInstrument,
    ) -> Result<Quote, MarketDataError> {
        let symbol = Self::equity_symbol(&instrument)?;
        let currency = self.resolve_currency(context);
        self.fetch_latest(&symbol, &currency).await
    }

    async fn get_historical_quotes(
        &self,
        context: &QuoteContext,
        instrument: ProviderInstrument,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Quote>, MarketDataError> {
        let symbol = Self::equity_symbol(&instrument)?;
        let currency = self.resolve_currency(context);
        self.fetch_historical(&symbol, &currency, start, end).await
    }

    async fn search(&self, query: &str) -> Result<Vec<SearchResult>, MarketDataError> {
        debug!("Searching FMP for '{}'", query);

        // Combine ticker search with company-name search.
        let symbol_matches = self.search_endpoint("search-symbol", query).await;
        let name_matches = self.search_endpoint("search-name", query).await;

        // Surface an error only if both endpoints failed.
        let (symbol_matches, name_matches) = match (symbol_matches, name_matches) {
            (Err(e), Err(_)) => return Err(e),
            (s, n) => (s.unwrap_or_default(), n.unwrap_or_default()),
        };

        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let results: Vec<SearchResult> = symbol_matches
            .into_iter()
            .chain(name_matches)
            .filter(|item| seen.insert(item.symbol.clone()))
            .map(Self::search_item_to_result)
            .collect();

        debug!(
            "FMP: search for '{}' returned {} results",
            query,
            results.len()
        );
        Ok(results)
    }

    async fn get_profile(&self, symbol: &str) -> Result<AssetProfile, MarketDataError> {
        debug!("Fetching profile for {} from FMP", symbol);

        let text = self.fetch("profile", &[("symbol", symbol)]).await?;
        let profiles: Vec<FmpProfile> = Self::parse(&text, "profile")?;

        let profile = profiles.into_iter().next().ok_or_else(|| {
            MarketDataError::SymbolNotFound(format!("No profile data for: {}", symbol))
        })?;

        Ok(profile.to_asset_profile())
    }

    async fn get_splits(
        &self,
        _context: &QuoteContext,
        instrument: ProviderInstrument,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<SplitEvent>, MarketDataError> {
        let symbol = Self::equity_symbol(&instrument)?;

        let text = self.fetch("splits", &[("symbol", &symbol)]).await?;
        let splits: Vec<FmpSplit> = Self::parse(&text, "splits")?;

        let mut events: Vec<SplitEvent> = splits
            .into_iter()
            .filter_map(|s| {
                let date = NaiveDate::parse_from_str(s.date.as_deref()?, "%Y-%m-%d").ok()?;
                let timestamp = Self::parse_date(&date.to_string())?;
                if timestamp < start || timestamp > end {
                    return None;
                }
                let numerator = s.numerator.filter(|v| *v > 0.0)?;
                let denominator = s.denominator.filter(|v| *v > 0.0)?;
                let ratio = Self::to_decimal(numerator / denominator)?;
                Some(SplitEvent { date, ratio })
            })
            .collect();

        events.sort_by_key(|e| e.date);
        Ok(events)
    }

    async fn get_dividends(
        &self,
        _context: &QuoteContext,
        instrument: ProviderInstrument,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<DividendEvent>, MarketDataError> {
        let symbol = Self::equity_symbol(&instrument)?;

        let text = self.fetch("dividends", &[("symbol", &symbol)]).await?;
        let dividends: Vec<FmpDividend> = Self::parse(&text, "dividends")?;

        let mut events: Vec<DividendEvent> = dividends
            .into_iter()
            .filter_map(|d| {
                let timestamp = Self::parse_date(d.date.as_deref()?)?;
                if timestamp < start || timestamp > end {
                    return None;
                }
                let amount = d.dividend.or(d.adj_dividend)?;
                Some(DividendEvent {
                    amount,
                    date: timestamp.timestamp(),
                })
            })
            .collect();

        events.sort_by_key(|e| e.date);
        Ok(events)
    }

    async fn screen(&self, query: &ScreenerQuery) -> Result<Vec<ScreenerHit>, MarketDataError> {
        let params = Self::build_screener_params(query);
        let params_ref: Vec<(&str, &str)> = params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let text = self.fetch("company-screener", &params_ref).await?;
        let items: Vec<FmpScreenerItem> = Self::parse(&text, "company-screener")?;

        let hits: Vec<ScreenerHit> = items
            .into_iter()
            .filter_map(FmpScreenerItem::into_hit)
            .collect();

        if hits.is_empty() {
            warn!("FMP: screener returned no results for query: {:?}", query);
        } else {
            debug!("FMP: screener returned {} hits", hits.len());
        }

        Ok(hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id_and_priority() {
        let provider = FmpProvider::new("test_key".to_string());
        assert_eq!(provider.id(), "FMP");
        assert_eq!(provider.priority(), 3);
    }

    #[test]
    fn test_provider_capabilities() {
        let provider = FmpProvider::new("test_key".to_string());
        let caps = provider.capabilities();
        assert!(caps.instrument_kinds.contains(&InstrumentKind::Equity));
        assert!(caps.supports_latest);
        assert!(caps.supports_historical);
        assert!(caps.supports_search);
        assert!(caps.supports_profile);
        assert!(caps.supports_dividends);
        assert!(caps.supports_screener);
    }

    #[test]
    fn test_rate_limit_is_conservative() {
        let provider = FmpProvider::new("test_key".to_string());
        let limit = provider.rate_limit();
        assert_eq!(limit.requests_per_minute, 10);
        assert_eq!(limit.max_concurrency, 1);
        assert_eq!(limit.min_delay, Duration::from_secs(2));
    }

    #[test]
    fn test_quote_response_parsing() {
        let json = r#"[{
            "symbol": "AAPL",
            "name": "Apple Inc.",
            "price": 232.8,
            "changePercentage": 2.1008,
            "change": 4.79,
            "volume": 44489128,
            "dayLow": 226.65,
            "dayHigh": 233.13,
            "yearHigh": 260.1,
            "yearLow": 164.08,
            "marketCap": 3500823120000,
            "priceAvg50": 240.2278,
            "priceAvg200": 219.98755,
            "exchange": "NASDAQ",
            "open": 227.2,
            "previousClose": 228.01,
            "timestamp": 1738702801
        }]"#;

        let quotes: Vec<FmpQuote> = serde_json::from_str(json).unwrap();
        assert_eq!(quotes.len(), 1);
        assert_eq!(quotes[0].price, Some(232.8));
        assert_eq!(quotes[0].open, Some(227.2));
        assert_eq!(quotes[0].day_high, Some(233.13));
        assert_eq!(quotes[0].day_low, Some(226.65));
        assert_eq!(quotes[0].volume, Some(44489128.0));
        assert_eq!(quotes[0].timestamp, Some(1738702801));
    }

    #[test]
    fn test_historical_response_parsing_flat() {
        let json = r#"[
            {
                "symbol": "AAPL",
                "date": "2025-02-04",
                "open": 227.2,
                "high": 233.13,
                "low": 226.65,
                "close": 232.8,
                "volume": 44489128,
                "change": 5.6,
                "changePercent": 2.46479,
                "vwap": 230.86
            },
            {
                "symbol": "AAPL",
                "date": "2025-02-03",
                "open": 229.99,
                "high": 231.83,
                "low": 225.7,
                "close": 228.01,
                "volume": 73063301,
                "change": -1.98,
                "changePercent": -0.86091,
                "vwap": 228.88
            }
        ]"#;

        let response: FmpHistoricalResponse = serde_json::from_str(json).unwrap();
        let bars = response.into_bars();
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].date.as_deref(), Some("2025-02-04"));
        assert_eq!(bars[0].close, Some(232.8));
        assert_eq!(bars[1].volume, Some(73063301.0));
    }

    #[test]
    fn test_historical_response_parsing_wrapped() {
        let json = r#"{
            "symbol": "AAPL",
            "historical": [
                {"date": "2025-02-04", "open": 227.2, "high": 233.13, "low": 226.65, "close": 232.8, "volume": 44489128}
            ]
        }"#;

        let response: FmpHistoricalResponse = serde_json::from_str(json).unwrap();
        let bars = response.into_bars();
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].close, Some(232.8));
    }

    #[test]
    fn test_bar_to_quote() {
        let bar = FmpHistoricalBar {
            date: Some("2025-02-04".to_string()),
            open: Some(227.2),
            high: Some(233.13),
            low: Some(226.65),
            close: Some(232.8),
            volume: Some(44489128.0),
        };

        let quote = FmpProvider::bar_to_quote(bar, "USD").unwrap();
        assert_eq!(quote.close.to_string(), "232.8");
        assert_eq!(quote.currency, "USD");
        assert_eq!(quote.source, "FMP");
        assert_eq!(quote.timestamp.date_naive().to_string(), "2025-02-04");
    }

    #[test]
    fn test_bar_to_quote_missing_close_returns_none() {
        let bar = FmpHistoricalBar {
            date: Some("2025-02-04".to_string()),
            open: None,
            high: None,
            low: None,
            close: None,
            volume: None,
        };
        assert!(FmpProvider::bar_to_quote(bar, "USD").is_none());
    }

    #[test]
    fn test_search_response_parsing() {
        let json = r#"[
            {
                "symbol": "AAPL",
                "name": "Apple Inc.",
                "currency": "USD",
                "exchangeFullName": "NASDAQ Global Select",
                "exchange": "NASDAQ"
            },
            {
                "symbol": "APC.DE",
                "name": "Apple Inc.",
                "currency": "EUR",
                "exchangeFullName": "Deutsche Börse",
                "exchange": "XETRA"
            }
        ]"#;

        let items: Vec<FmpSearchItem> = serde_json::from_str(json).unwrap();
        assert_eq!(items.len(), 2);

        let result = FmpProvider::search_item_to_result(items.into_iter().next().unwrap());
        assert_eq!(result.symbol, "AAPL");
        assert_eq!(result.name, "Apple Inc.");
        assert_eq!(result.exchange, "NASDAQ");
        assert_eq!(result.currency.as_deref(), Some("USD"));
        assert_eq!(result.data_source.as_deref(), Some("FMP"));
    }

    #[test]
    fn test_profile_response_parsing() {
        let json = r#"[{
            "symbol": "AAPL",
            "price": 232.8,
            "marketCap": 3500823120000,
            "beta": 1.24,
            "lastDividend": 0.99,
            "range": "164.08-260.10",
            "companyName": "Apple Inc.",
            "currency": "USD",
            "cik": "0000320193",
            "isin": "US0378331005",
            "cusip": "037833100",
            "exchangeFullName": "NASDAQ Global Select",
            "exchange": "NASDAQ",
            "industry": "Consumer Electronics",
            "website": "https://www.apple.com",
            "description": "Apple Inc. designs, manufactures, and markets smartphones.",
            "sector": "Technology",
            "country": "US",
            "fullTimeEmployees": "164000",
            "image": "https://images.financialmodelingprep.com/symbol/AAPL.png",
            "isEtf": false,
            "isActivelyTrading": true,
            "isAdr": false,
            "isFund": false
        }]"#;

        let profiles: Vec<FmpProfile> = serde_json::from_str(json).unwrap();
        let profile = profiles.into_iter().next().unwrap().to_asset_profile();

        assert_eq!(profile.source, Some("FMP".to_string()));
        assert_eq!(profile.name, Some("Apple Inc.".to_string()));
        assert_eq!(profile.quote_type, Some("EQUITY".to_string()));
        assert_eq!(profile.sector, Some("Technology".to_string()));
        assert_eq!(profile.industry, Some("Consumer Electronics".to_string()));
        assert_eq!(profile.country, Some("US".to_string()));
        assert_eq!(profile.employees, Some(164000));
        assert_eq!(profile.market_cap, Some(3500823120000.0));
        assert_eq!(profile.isin, Some("US0378331005".to_string()));
        assert_eq!(profile.week_52_low, Some(164.08));
        assert_eq!(profile.week_52_high, Some(260.10));
        // dividend_yield = lastDividend / price
        assert!((profile.dividend_yield.unwrap() - 0.99 / 232.8).abs() < 1e-9);
    }

    #[test]
    fn test_profile_etf_quote_type() {
        let json =
            r#"[{"symbol": "SPY", "companyName": "SPDR S&P 500", "isEtf": true, "isFund": false}]"#;
        let profiles: Vec<FmpProfile> = serde_json::from_str(json).unwrap();
        let profile = profiles.into_iter().next().unwrap().to_asset_profile();
        assert_eq!(profile.quote_type, Some("ETF".to_string()));
    }

    #[test]
    fn test_dividends_response_parsing() {
        let json = r#"[
            {
                "symbol": "AAPL",
                "date": "2025-02-10",
                "recordDate": "2025-02-10",
                "paymentDate": "2025-02-13",
                "declarationDate": "2025-01-30",
                "adjDividend": 0.25,
                "dividend": 0.25,
                "yield": 0.42955326460481097,
                "frequency": "Quarterly"
            }
        ]"#;

        let dividends: Vec<FmpDividend> = serde_json::from_str(json).unwrap();
        assert_eq!(dividends.len(), 1);
        assert_eq!(dividends[0].date.as_deref(), Some("2025-02-10"));
        assert_eq!(dividends[0].dividend, Some(0.25));
    }

    #[test]
    fn test_splits_response_parsing() {
        let json = r#"[
            {
                "symbol": "AAPL",
                "date": "2020-08-31",
                "numerator": 4,
                "denominator": 1
            }
        ]"#;

        let splits: Vec<FmpSplit> = serde_json::from_str(json).unwrap();
        assert_eq!(splits.len(), 1);
        assert_eq!(splits[0].numerator, Some(4.0));
        assert_eq!(splits[0].denominator, Some(1.0));
    }

    #[test]
    fn test_screener_response_parsing() {
        let json = r#"[
            {
                "symbol": "AAPL",
                "companyName": "Apple Inc.",
                "marketCap": 3500823120000,
                "sector": "Technology",
                "industry": "Consumer Electronics",
                "beta": 1.24,
                "price": 232.8,
                "lastAnnualDividend": 0.99,
                "volume": 44489128,
                "exchange": "NASDAQ Global Select",
                "exchangeShortName": "NASDAQ",
                "country": "US",
                "isEtf": false,
                "isFund": false,
                "isActivelyTrading": true
            }
        ]"#;

        let items: Vec<FmpScreenerItem> = serde_json::from_str(json).unwrap();
        let hit = items.into_iter().next().unwrap().into_hit().unwrap();

        assert_eq!(hit.symbol, "AAPL");
        assert_eq!(hit.name.as_deref(), Some("Apple Inc."));
        assert_eq!(hit.market_cap, Some(3500823120000.0));
        assert_eq!(hit.sector.as_deref(), Some("Technology"));
        assert_eq!(hit.industry.as_deref(), Some("Consumer Electronics"));
        assert_eq!(hit.price, Some(232.8));
        assert_eq!(hit.exchange.as_deref(), Some("NASDAQ"));
        assert_eq!(hit.country.as_deref(), Some("US"));
    }

    #[test]
    fn test_screener_item_without_symbol_is_dropped() {
        let json = r#"[{"companyName": "No Symbol Inc."}]"#;
        let items: Vec<FmpScreenerItem> = serde_json::from_str(json).unwrap();
        assert!(items.into_iter().next().unwrap().into_hit().is_none());
    }

    #[test]
    fn test_build_screener_params_full_query() {
        let query = ScreenerQuery {
            sector: Some("Technology".to_string()),
            industry: Some("Semiconductors".to_string()),
            market_cap_min: Some(1000000000.0),
            market_cap_max: Some(5000000000.0),
            price_min: Some(10.0),
            price_max: Some(500.0),
            beta_min: Some(0.5),
            beta_max: Some(1.5),
            dividend_min: Some(0.5),
            volume_min: Some(100000.0),
            exchange: Some("NASDAQ".to_string()),
            country: Some("US".to_string()),
            is_etf: Some(false),
            is_actively_trading: Some(true),
            limit: Some(25),
        };

        let params = FmpProvider::build_screener_params(&query);
        let get = |key: &str| {
            params
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
        };

        assert_eq!(get("marketCapMoreThan"), Some("1000000000"));
        assert_eq!(get("marketCapLowerThan"), Some("5000000000"));
        assert_eq!(get("priceMoreThan"), Some("10"));
        assert_eq!(get("priceLowerThan"), Some("500"));
        assert_eq!(get("betaMoreThan"), Some("0.5"));
        assert_eq!(get("betaLowerThan"), Some("1.5"));
        assert_eq!(get("dividendMoreThan"), Some("0.5"));
        assert_eq!(get("volumeMoreThan"), Some("100000"));
        assert_eq!(get("sector"), Some("Technology"));
        assert_eq!(get("industry"), Some("Semiconductors"));
        assert_eq!(get("exchange"), Some("NASDAQ"));
        assert_eq!(get("country"), Some("US"));
        assert_eq!(get("isEtf"), Some("false"));
        assert_eq!(get("isActivelyTrading"), Some("true"));
        assert_eq!(get("limit"), Some("25"));
    }

    #[test]
    fn test_build_screener_params_empty_query_only_has_limit() {
        let params = FmpProvider::build_screener_params(&ScreenerQuery::default());
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].0, "limit");
        assert_eq!(params[0].1, DEFAULT_SCREENER_LIMIT.to_string());
    }

    #[test]
    fn test_check_error_body_rate_limit() {
        let body = r#"{"Error Message": "Limit Reach . Please upgrade your plan"}"#;
        let err = FmpProvider::check_error_body(body).unwrap_err();
        assert!(matches!(err, MarketDataError::RateLimited { .. }));
    }

    #[test]
    fn test_check_error_body_generic_error() {
        let body = r#"{"Error Message": "Invalid API KEY."}"#;
        let err = FmpProvider::check_error_body(body).unwrap_err();
        assert!(matches!(err, MarketDataError::ProviderError { .. }));
    }

    #[test]
    fn test_check_error_body_ok_for_data() {
        let body = r#"[{"symbol": "AAPL", "price": 232.8}]"#;
        assert!(FmpProvider::check_error_body(body).is_ok());
    }

    #[test]
    fn test_parse_date() {
        let date = FmpProvider::parse_date("2024-01-15").unwrap();
        assert_eq!(date.date_naive().to_string(), "2024-01-15");
        assert!(FmpProvider::parse_date("not-a-date").is_none());
    }

    #[test]
    fn test_profile_parse_employees_variants() {
        let with_string: Vec<FmpProfile> =
            serde_json::from_str(r#"[{"fullTimeEmployees": "164,000"}]"#).unwrap();
        assert_eq!(with_string[0].parse_employees(), Some(164000));

        let with_number: Vec<FmpProfile> =
            serde_json::from_str(r#"[{"fullTimeEmployees": 164000}]"#).unwrap();
        assert_eq!(with_number[0].parse_employees(), Some(164000));

        let missing: Vec<FmpProfile> = serde_json::from_str(r#"[{}]"#).unwrap();
        assert_eq!(missing[0].parse_employees(), None);
    }
}
