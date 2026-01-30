//! OpenFIGI market data provider for ISIN resolution and symbol search.
//!
//! This provider integrates with the OpenFIGI v3 API to:
//! - Resolve ISIN codes to ticker symbols (via /mapping endpoint)
//! - Search for securities by name or symbol (via /search endpoint)
//!
//! OpenFIGI does NOT provide market quotes — it is a metadata/identity
//! provider only. The `get_latest_quote` and `get_historical_quotes`
//! methods return `NotSupported`.
//!
//! API key is optional (free tier: ~20 req/min; with key: ~120 req/min).

use async_trait::async_trait;
use log::{debug, warn};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::errors::MarketDataError;
use crate::models::{
    AssetProfile, Coverage, InstrumentKind, ProviderInstrument, Quote, QuoteContext, SearchResult,
};
use crate::provider::{MarketDataProvider, ProviderCapabilities, RateLimit};

const BASE_URL: &str = "https://api.openfigi.com/v3";
const PROVIDER_ID: &str = "OPENFIGI";

/// OpenFIGI provider for ISIN-to-ticker resolution and security search.
pub struct OpenFigiProvider {
    client: Client,
    api_key: Option<String>,
}

impl OpenFigiProvider {
    /// Create a new OpenFIGI provider.
    ///
    /// `api_key` is optional. Without it, you get ~20 requests/minute.
    /// With a free API key, you get ~120 requests/minute.
    pub fn new(api_key: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client, api_key }
    }

    /// Map an ISIN to a ticker symbol via the /mapping endpoint.
    async fn map_isin_to_ticker(&self, isin: &str) -> Result<FigiResult, MarketDataError> {
        let url = format!("{}/mapping", BASE_URL);

        let request_body = vec![MappingJob {
            id_type: "ID_ISIN".to_string(),
            id_value: isin.to_string(),
            ..Default::default()
        }];

        let mut req = self.client.post(&url).json(&request_body);
        if let Some(key) = &self.api_key {
            req = req.header("X-OPENFIGI-APIKEY", key);
        }

        let response = req.send().await.map_err(|e| MarketDataError::ProviderError {
            provider: PROVIDER_ID.to_string(),
            message: format!("Request failed: {}", e),
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if status == StatusCode::TOO_MANY_REQUESTS {
                return Err(MarketDataError::RateLimited {
                    provider: PROVIDER_ID.to_string(),
                });
            }

            return Err(MarketDataError::ProviderError {
                provider: PROVIDER_ID.to_string(),
                message: format!("API error {}: {}", status, body),
            });
        }

        let results: Vec<MappingResponse> =
            response
                .json()
                .await
                .map_err(|e| MarketDataError::ProviderError {
                    provider: PROVIDER_ID.to_string(),
                    message: format!("Failed to parse response: {}", e),
                })?;

        if let Some(first) = results.first() {
            if let Some(error) = &first.error {
                return Err(MarketDataError::ProviderError {
                    provider: PROVIDER_ID.to_string(),
                    message: format!("Mapping error: {}", error),
                });
            }
            if let Some(data) = &first.data {
                if let Some(figi_result) = data.first() {
                    return Ok(figi_result.clone());
                }
            }
        }

        Err(MarketDataError::SymbolNotFound(format!(
            "No FIGI mapping found for ISIN: {}",
            isin
        )))
    }

    /// Search for securities via the /search endpoint.
    async fn search_securities(&self, query: &str) -> Result<Vec<FigiResult>, MarketDataError> {
        let url = format!("{}/search", BASE_URL);

        let request_body = SearchRequest {
            query: query.to_string(),
            ..Default::default()
        };

        let mut req = self.client.post(&url).json(&request_body);
        if let Some(key) = &self.api_key {
            req = req.header("X-OPENFIGI-APIKEY", key);
        }

        let response = req.send().await.map_err(|e| MarketDataError::ProviderError {
            provider: PROVIDER_ID.to_string(),
            message: format!("Search request failed: {}", e),
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if status == StatusCode::TOO_MANY_REQUESTS {
                return Err(MarketDataError::RateLimited {
                    provider: PROVIDER_ID.to_string(),
                });
            }

            return Err(MarketDataError::ProviderError {
                provider: PROVIDER_ID.to_string(),
                message: format!("Search error {}: {}", status, body),
            });
        }

        let search_response: SearchResponse =
            response
                .json()
                .await
                .map_err(|e| MarketDataError::ProviderError {
                    provider: PROVIDER_ID.to_string(),
                    message: format!("Failed to parse search response: {}", e),
                })?;

        if let Some(error) = search_response.error {
            return Err(MarketDataError::ProviderError {
                provider: PROVIDER_ID.to_string(),
                message: format!("Search error: {}", error),
            });
        }

        Ok(search_response.data.unwrap_or_default())
    }

    /// Map OpenFIGI exchange code to Yahoo Finance suffix.
    fn yahoo_suffix_for_exchange(exch_code: &str) -> &'static str {
        match exch_code {
            // United States
            "US" | "UN" | "UQ" | "UW" | "UR" | "UA" | "U3" | "U1" | "U2" | "U9" | "U0" => "",
            // Europe
            "LN" | "L" => ".L",
            "GY" | "GR" => ".DE",
            "FP" | "PA" => ".PA",
            "AV" | "AS" => ".AS",
            "BB" | "BR" => ".BR",
            "LI" | "LS" => ".LS",
            "ID" | "IR" => ".IR",
            "IM" | "MI" => ".MI",
            "SM" | "MC" => ".MC",
            "SW" | "S" => ".SW",
            "VX" => ".VX",
            "NO" | "OL" => ".OL",
            "SS" | "ST" => ".ST",
            "CP" | "CO" => ".CO",
            "HE" | "FH" => ".HE",
            "VI" | "VA" => ".VI",
            // Asia Pacific
            "HK" | "H" => ".HK",
            "JP" | "TK" | "T" => ".T",
            "AU" | "AX" => ".AX",
            "NZ" => ".NZ",
            "KS" | "KO" => ".KS",
            "KQ" => ".KQ",
            "SI" | "SP" => ".SI",
            "TW" => ".TW",
            "SH" => ".SS",
            "SZ" => ".SZ",
            "BK" => ".BK",
            "JK" => ".JK",
            "KL" => ".KL",
            // Americas (Non-US)
            "CN" | "TO" => ".TO",
            "CV" | "V" => ".V",
            "MX" | "MM" => ".MX",
            "SA" | "SN" => ".SA",
            "BA" | "BC" => ".BA",
            // Middle East / Africa
            "TA" => ".TA",
            "JO" | "SJ" => ".JO",
            _ => {
                debug!(
                    "OpenFIGI: Unknown exchange code '{}', using no suffix",
                    exch_code
                );
                ""
            }
        }
    }

    /// Convert a FigiResult to a Yahoo-compatible ticker.
    fn to_yahoo_ticker(ticker: &str, exch_code: Option<&str>) -> String {
        match exch_code {
            Some(code) => format!("{}{}", ticker, Self::yahoo_suffix_for_exchange(code)),
            None => ticker.to_string(),
        }
    }
}

#[async_trait]
impl MarketDataProvider for OpenFigiProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    fn priority(&self) -> u8 {
        20 // Low priority — metadata-only provider
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            instrument_kinds: &[InstrumentKind::Equity],
            coverage: Coverage::global_best_effort(),
            supports_latest: false,
            supports_historical: false,
            supports_search: true,
            supports_profile: true,
        }
    }

    fn rate_limit(&self) -> RateLimit {
        if self.api_key.is_some() {
            RateLimit {
                requests_per_minute: 120,
                max_concurrency: 5,
                min_delay: Duration::from_millis(500),
            }
        } else {
            RateLimit {
                requests_per_minute: 20,
                max_concurrency: 2,
                min_delay: Duration::from_secs(3),
            }
        }
    }

    async fn get_latest_quote(
        &self,
        _context: &QuoteContext,
        _instrument: ProviderInstrument,
    ) -> Result<Quote, MarketDataError> {
        Err(MarketDataError::NotSupported {
            operation: "quotes".to_string(),
            provider: PROVIDER_ID.to_string(),
        })
    }

    async fn get_historical_quotes(
        &self,
        _context: &QuoteContext,
        _instrument: ProviderInstrument,
        _start: chrono::DateTime<chrono::Utc>,
        _end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Quote>, MarketDataError> {
        Err(MarketDataError::NotSupported {
            operation: "quotes".to_string(),
            provider: PROVIDER_ID.to_string(),
        })
    }

    async fn search(&self, query: &str) -> Result<Vec<SearchResult>, MarketDataError> {
        debug!("OpenFIGI: Searching for: {}", query);

        let is_isin = query.len() == 12 && query.chars().all(|c| c.is_alphanumeric());

        if is_isin {
            debug!("OpenFIGI: Detected ISIN format, using mapping API");
            match self.map_isin_to_ticker(query).await {
                Ok(figi_result) => {
                    if let Some(ticker) = &figi_result.ticker {
                        let yahoo_ticker =
                            Self::to_yahoo_ticker(ticker, figi_result.exch_code.as_deref());
                        debug!(
                            "OpenFIGI: Resolved ISIN {} -> {} (FIGI: {})",
                            query, yahoo_ticker, figi_result.figi
                        );
                        return Ok(vec![SearchResult::new(
                            &yahoo_ticker,
                            figi_result
                                .name
                                .as_deref()
                                .unwrap_or(&yahoo_ticker),
                            figi_result.exch_code.as_deref().unwrap_or(""),
                            figi_result.security_type.as_deref().unwrap_or("EQUITY"),
                        )
                        .with_score(1.0)
                        .with_data_source(PROVIDER_ID)]);
                    }
                    Err(MarketDataError::SymbolNotFound(format!(
                        "ISIN {} resolved but no ticker found",
                        query
                    )))
                }
                Err(e) => {
                    debug!("OpenFIGI: ISIN mapping failed for {}: {:?}", query, e);
                    Err(e)
                }
            }
        } else {
            let results = self.search_securities(query).await?;
            let search_results: Vec<SearchResult> = results
                .iter()
                .filter_map(|r| {
                    r.ticker.as_ref().map(|ticker| {
                        SearchResult::new(
                            ticker,
                            r.name.as_deref().unwrap_or(ticker),
                            r.exch_code.as_deref().unwrap_or(""),
                            r.security_type.as_deref().unwrap_or("EQUITY"),
                        )
                        .with_data_source(PROVIDER_ID)
                    })
                })
                .collect();
            Ok(search_results)
        }
    }

    async fn get_profile(&self, symbol: &str) -> Result<AssetProfile, MarketDataError> {
        let is_isin = symbol.len() == 12 && symbol.chars().all(|c| c.is_alphanumeric());

        if is_isin {
            debug!("OpenFIGI: Resolving ISIN profile: {}", symbol);
            let figi_result = self.map_isin_to_ticker(symbol).await?;
            let ticker = figi_result.ticker.as_deref().ok_or_else(|| {
                MarketDataError::SymbolNotFound(format!("No ticker for ISIN: {}", symbol))
            })?;
            let yahoo_ticker =
                Self::to_yahoo_ticker(ticker, figi_result.exch_code.as_deref());

            Ok(AssetProfile {
                source: Some(PROVIDER_ID.to_string()),
                name: figi_result.name,
                quote_type: figi_result.security_type,
                ..Default::default()
            })
        } else {
            debug!("OpenFIGI: Searching profile for: {}", symbol);
            let results = self.search_securities(symbol).await?;
            let first = results.first().ok_or_else(|| {
                MarketDataError::SymbolNotFound(format!("No results for: {}", symbol))
            })?;

            Ok(AssetProfile {
                source: Some(PROVIDER_ID.to_string()),
                name: first.name.clone(),
                quote_type: first.security_type.clone(),
                ..Default::default()
            })
        }
    }
}

// ============================================================================
// OpenFIGI API Request/Response Structures (v3 API)
// ============================================================================

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct MappingJob {
    id_type: String,
    id_value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    exch_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mic_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    market_sec_des: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    security_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    security_type2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_unlisted_equities: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MappingResponse {
    #[serde(default)]
    data: Option<Vec<FigiResult>>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    warning: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FigiResult {
    figi: String,
    #[serde(default)]
    security_type: Option<String>,
    #[serde(default)]
    market_sector: Option<String>,
    #[serde(default)]
    ticker: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    exch_code: Option<String>,
    #[serde(default)]
    share_class_figi: Option<String>,
    #[serde(default)]
    composite_figi: Option<String>,
    #[serde(default)]
    security_type2: Option<String>,
    #[serde(default)]
    security_description: Option<String>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct SearchRequest {
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exch_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mic_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    market_sec_des: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    security_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    security_type2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_unlisted_equities: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    data: Option<Vec<FigiResult>>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    next: Option<String>,
    #[serde(default)]
    total: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yahoo_suffix_mapping() {
        assert_eq!(OpenFigiProvider::yahoo_suffix_for_exchange("US"), "");
        assert_eq!(OpenFigiProvider::yahoo_suffix_for_exchange("LN"), ".L");
        assert_eq!(OpenFigiProvider::yahoo_suffix_for_exchange("CN"), ".TO");
        assert_eq!(OpenFigiProvider::yahoo_suffix_for_exchange("HK"), ".HK");
        assert_eq!(OpenFigiProvider::yahoo_suffix_for_exchange("JP"), ".T");
    }

    #[test]
    fn test_to_yahoo_ticker() {
        assert_eq!(
            OpenFigiProvider::to_yahoo_ticker("AAPL", Some("US")),
            "AAPL"
        );
        assert_eq!(
            OpenFigiProvider::to_yahoo_ticker("SHOP", Some("CN")),
            "SHOP.TO"
        );
        assert_eq!(
            OpenFigiProvider::to_yahoo_ticker("VOD", Some("LN")),
            "VOD.L"
        );
        assert_eq!(OpenFigiProvider::to_yahoo_ticker("AAPL", None), "AAPL");
    }

    #[test]
    fn test_isin_detection() {
        // Valid ISIN format (12 alphanumeric)
        let isin = "US0378331005";
        assert_eq!(isin.len(), 12);
        assert!(isin.chars().all(|c| c.is_alphanumeric()));

        // Not an ISIN
        let ticker = "AAPL";
        assert_ne!(ticker.len(), 12);
    }
}
