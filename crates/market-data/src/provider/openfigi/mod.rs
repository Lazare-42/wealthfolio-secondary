use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::debug;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::errors::MarketDataError;
use crate::models::{
    Coverage, InstrumentKind, ProviderInstrument, Quote, QuoteContext, SearchResult,
};
use crate::provider::{MarketDataProvider, ProviderCapabilities, RateLimit};

const BASE_URL: &str = "https://api.openfigi.com/v3";
const PROVIDER_ID: &str = "OPENFIGI";

pub struct OpenFigiProvider {
    client: Client,
    api_key: Option<String>,
}

impl OpenFigiProvider {
    pub fn new(api_key: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { client, api_key }
    }

    async fn map_isin(&self, isin: &str) -> Result<FigiResult, MarketDataError> {
        let url = format!("{}/mapping", BASE_URL);
        let body = vec![MappingJob {
            id_type: "ID_ISIN".to_string(),
            id_value: isin.to_string(),
            ..Default::default()
        }];

        let mut req = self.client.post(&url).json(&body);
        if let Some(key) = &self.api_key {
            req = req.header("X-OPENFIGI-APIKEY", key);
        }

        let resp = req.send().await.map_err(|e| MarketDataError::ProviderError {
            provider: PROVIDER_ID.to_string(),
            message: format!("Request failed: {}", e),
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if status == StatusCode::TOO_MANY_REQUESTS {
                return Err(MarketDataError::RateLimited {
                    provider: PROVIDER_ID.to_string(),
                });
            }
            return Err(MarketDataError::ProviderError {
                provider: PROVIDER_ID.to_string(),
                message: format!("API error {}: {}", status, text),
            });
        }

        let results: Vec<MappingResponse> =
            resp.json().await.map_err(|e| MarketDataError::ProviderError {
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
                if let Some(result) = data.first() {
                    return Ok(result.clone());
                }
            }
        }

        Err(MarketDataError::SymbolNotFound(format!(
            "No FIGI mapping found for ISIN: {}",
            isin
        )))
    }

    async fn search_securities(&self, query: &str) -> Result<Vec<FigiResult>, MarketDataError> {
        let url = format!("{}/search", BASE_URL);
        let body = SearchRequest {
            query: query.to_string(),
            ..Default::default()
        };

        let mut req = self.client.post(&url).json(&body);
        if let Some(key) = &self.api_key {
            req = req.header("X-OPENFIGI-APIKEY", key);
        }

        let resp = req.send().await.map_err(|e| MarketDataError::ProviderError {
            provider: PROVIDER_ID.to_string(),
            message: format!("Search request failed: {}", e),
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if status == StatusCode::TOO_MANY_REQUESTS {
                return Err(MarketDataError::RateLimited {
                    provider: PROVIDER_ID.to_string(),
                });
            }
            return Err(MarketDataError::ProviderError {
                provider: PROVIDER_ID.to_string(),
                message: format!("Search error {}: {}", status, text),
            });
        }

        let search_resp: SearchResponse =
            resp.json().await.map_err(|e| MarketDataError::ProviderError {
                provider: PROVIDER_ID.to_string(),
                message: format!("Failed to parse search response: {}", e),
            })?;

        if let Some(error) = search_resp.error {
            return Err(MarketDataError::ProviderError {
                provider: PROVIDER_ID.to_string(),
                message: format!("Search error: {}", error),
            });
        }

        Ok(search_resp.data.unwrap_or_default())
    }

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
        20
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            instrument_kinds: &[InstrumentKind::Equity],
            coverage: Coverage::global_best_effort(),
            supports_latest: false,
            supports_historical: false,
            supports_search: true,
            supports_profile: false,
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
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> Result<Vec<Quote>, MarketDataError> {
        Err(MarketDataError::NotSupported {
            operation: "quotes".to_string(),
            provider: PROVIDER_ID.to_string(),
        })
    }

    async fn search(&self, query: &str) -> Result<Vec<SearchResult>, MarketDataError> {
        let is_isin = query.len() == 12 && query.chars().all(|c| c.is_alphanumeric());

        if is_isin {
            let figi = self.map_isin(query).await?;
            if let Some(ticker) = &figi.ticker {
                let yahoo_ticker = Self::to_yahoo_ticker(ticker, figi.exch_code.as_deref());
                return Ok(vec![SearchResult::new(
                    &yahoo_ticker,
                    figi.name.as_deref().unwrap_or(&yahoo_ticker),
                    figi.exch_code.as_deref().unwrap_or(""),
                    figi.security_type.as_deref().unwrap_or("EQUITY"),
                )
                .with_score(1.0)
                .with_data_source(PROVIDER_ID)]);
            }
            Err(MarketDataError::SymbolNotFound(format!(
                "No ticker found for ISIN: {}",
                query
            )))
        } else {
            let results = self.search_securities(query).await?;
            let search_results = results
                .iter()
                .filter_map(|r| {
                    r.ticker.as_ref().map(|ticker| {
                        let yahoo_ticker =
                            Self::to_yahoo_ticker(ticker, r.exch_code.as_deref());
                        SearchResult::new(
                            &yahoo_ticker,
                            r.name.as_deref().unwrap_or(&yahoo_ticker),
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
}

// =============================================================================
// OpenFIGI API types
// =============================================================================

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
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MappingResponse {
    #[serde(default)]
    data: Option<Vec<FigiResult>>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FigiResult {
    #[allow(dead_code)]
    figi: String,
    #[serde(default)]
    security_type: Option<String>,
    #[serde(default)]
    ticker: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    exch_code: Option<String>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct SearchRequest {
    query: String,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    data: Option<Vec<FigiResult>>,
    #[serde(default)]
    error: Option<String>,
}
