use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::{debug, warn};
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

/// Convert ISIN characters to a digit vector for Luhn computation.
fn isin_to_digits(s: &str) -> Vec<u8> {
    let mut digits = Vec::with_capacity(16);
    for c in s.chars() {
        if c.is_ascii_digit() {
            digits.push(c as u8 - b'0');
        } else {
            let val = c.to_ascii_uppercase() as u8 - b'A' + 10;
            digits.push(val / 10);
            digits.push(val % 10);
        }
    }
    digits
}

/// Compute the Luhn checksum of a digit sequence.
fn luhn_sum(digits: &[u8]) -> u32 {
    let mut sum = 0u32;
    for (i, &d) in digits.iter().rev().enumerate() {
        let mut n = d as u32;
        if i % 2 == 1 {
            n *= 2;
            if n > 9 {
                n -= 9;
            }
        }
        sum += n;
    }
    sum
}

/// Validate an ISIN check digit using the Luhn algorithm.
fn is_valid_isin(isin: &str) -> bool {
    if isin.len() != 12 || !isin.chars().all(|c| c.is_ascii_alphanumeric()) {
        return false;
    }
    luhn_sum(&isin_to_digits(isin)) % 10 == 0
}

/// Compute the correct check digit for an 11-character ISIN base and return the full 12-char ISIN.
fn correct_isin_check_digit(isin: &str) -> Option<String> {
    if isin.len() != 12 || !isin.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    let base = &isin[..11];
    // Append '0' as placeholder check digit, compute Luhn, derive correct digit
    let mut digits = isin_to_digits(base);
    digits.push(0);
    let sum = luhn_sum(&digits);
    let check = ((10 - (sum % 10)) % 10) as u8;
    Some(format!("{}{}", base, check))
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
        let looks_like_isin = query.len() == 12 && query.chars().all(|c| c.is_alphanumeric());

        if looks_like_isin {
            let (isin, was_corrected) = if is_valid_isin(query) {
                (query.to_string(), false)
            } else if let Some(corrected) = correct_isin_check_digit(query) {
                warn!(
                    "OpenFIGI: ISIN '{}' has invalid check digit, corrected to '{}'",
                    query, corrected
                );
                (corrected, true)
            } else {
                return Err(MarketDataError::SymbolNotFound(format!(
                    "ISIN '{}' has an invalid format",
                    query
                )));
            };
            let figi = self.map_isin(&isin).await?;
            if let Some(ticker) = &figi.ticker {
                let yahoo_ticker = Self::to_yahoo_ticker(ticker, figi.exch_code.as_deref());
                let name = figi.name.as_deref().unwrap_or(&yahoo_ticker);
                let display_name = if was_corrected {
                    format!("{} (ISIN corrected: {} → {})", name, query, isin)
                } else {
                    name.to_string()
                };
                let score = if was_corrected { 0.8 } else { 1.0 };
                return Ok(vec![SearchResult::new(
                    &yahoo_ticker,
                    &display_name,
                    figi.exch_code.as_deref().unwrap_or(""),
                    figi.security_type.as_deref().unwrap_or("EQUITY"),
                )
                .with_score(score)
                .with_data_source(PROVIDER_ID)]);
            }
            Err(MarketDataError::SymbolNotFound(format!(
                "No ticker found for ISIN: {}",
                isin
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_isins() {
        assert!(is_valid_isin("LU0056508442"));
        assert!(is_valid_isin("LU0908500753"));
        assert!(is_valid_isin("IE00B3RBWM25"));
        assert!(is_valid_isin("US0378331005")); // AAPL
        assert!(is_valid_isin("FR0000120628")); // Sanofi
    }

    #[test]
    fn test_invalid_isins_bad_check_digit() {
        assert!(!is_valid_isin("LU0227064083"));
        assert!(!is_valid_isin("IE00BZV7JH90"));
        assert!(!is_valid_isin("IE00B3FX4G56"));
        assert!(!is_valid_isin("LU0856906451"));
        assert!(!is_valid_isin("LU1078790904"));
        assert!(!is_valid_isin("LU1002230284"));
    }

    #[test]
    fn test_invalid_isin_format() {
        assert!(!is_valid_isin(""));
        assert!(!is_valid_isin("SHORT"));
        assert!(!is_valid_isin("TOOLONGSTRING1"));
        assert!(!is_valid_isin("LU02270640!3"));
    }

    #[test]
    fn test_correct_check_digit() {
        // Bad check digit → corrected
        assert_eq!(
            correct_isin_check_digit("LU0856906451"),
            Some("LU0856906457".to_string())
        );
        // Already valid → same ISIN back
        assert_eq!(
            correct_isin_check_digit("LU0056508442"),
            Some("LU0056508442".to_string())
        );
        // Invalid format → None
        assert_eq!(correct_isin_check_digit("SHORT"), None);
    }
}
