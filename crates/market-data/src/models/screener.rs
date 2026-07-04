//! Stock screener models.
//!
//! `ScreenerQuery` describes filter criteria for a provider-backed stock
//! screener; `ScreenerHit` is a single matching instrument.

use serde::{Deserialize, Serialize};

/// Filter criteria for a stock screener request.
///
/// All fields are optional; providers ignore filters they don't support.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenerQuery {
    /// Sector name (e.g., "Technology")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sector: Option<String>,

    /// Industry name (e.g., "Consumer Electronics")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry: Option<String>,

    /// Minimum market capitalization (in quote currency units)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_cap_min: Option<f64>,

    /// Maximum market capitalization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_cap_max: Option<f64>,

    /// Minimum share price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_min: Option<f64>,

    /// Maximum share price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_max: Option<f64>,

    /// Minimum beta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beta_min: Option<f64>,

    /// Maximum beta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beta_max: Option<f64>,

    /// Minimum dividend (per share, annual)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dividend_min: Option<f64>,

    /// Minimum average volume
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_min: Option<f64>,

    /// Exchange code (e.g., "NASDAQ", "NYSE")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange: Option<String>,

    /// Country code (e.g., "US")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Restrict to (or exclude) ETFs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_etf: Option<bool>,

    /// Restrict to actively trading instruments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_actively_trading: Option<bool>,

    /// Maximum number of results to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// A single instrument matched by a screener query.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenerHit {
    /// Ticker symbol (e.g., "AAPL")
    pub symbol: String,

    /// Company/fund name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Market capitalization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_cap: Option<f64>,

    /// Sector name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sector: Option<String>,

    /// Industry name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry: Option<String>,

    /// Latest share price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,

    /// Exchange code as reported by the provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange: Option<String>,

    /// Country code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}
