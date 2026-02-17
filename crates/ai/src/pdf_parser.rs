//! PDF statement parser using LLM providers.
//!
//! Extracts structured transaction data from PDF text using the configured AI provider.
//! Follows the same provider dispatch pattern as `title_generator.rs`.

use async_trait::async_trait;
use log::{debug, error, warn};
use reqwest::Client as HttpClient;
use rig::{
    client::{CompletionClient, Nothing},
    completion::{message::Message, Prompt},
    providers::{anthropic, gemini, groq, ollama, openai, openrouter},
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::env::AiEnvironment;
use crate::error::AiError;
use crate::providers::ProviderService;
use wealthfolio_core::activities::ActivityImport;

/// Raw transaction parsed from PDF by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawPdfTransaction {
    pub date: String,
    #[serde(default)]
    pub symbol: Option<String>,
    pub activity_type: String,
    #[serde(default)]
    pub quantity: Option<Decimal>,
    #[serde(default)]
    pub unit_price: Option<Decimal>,
    pub currency: String,
    #[serde(default)]
    pub fee: Option<Decimal>,
    #[serde(default)]
    pub amount: Option<Decimal>,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub account_name: Option<String>,
}

/// Shared instruction text for both text and vision parsing paths.
/// Also used by `apps/server/src/pdf_import.rs` vision message builder.
pub const PARSE_INSTRUCTIONS: &str = r#"You are a financial document parser. Extract ALL financial line items from this bank or broker document.

The document may be:
- A transaction history / account statement ("relevé de compte") → extract each transaction
- A holdings snapshot / statement of assets ("état des biens", "relevé de portefeuille") → extract each position as a BUY with the statement date, the position's quantity, unit price, and total value as amount

Return ONLY a JSON array. No markdown, no explanation, no code blocks, no ```json wrapper.

Each object must have these fields:
- "date": ISO date "YYYY-MM-DD". Must be a valid calendar date. If unsure of exact day, use the 1st of the month.
- "symbol": the security identifier exactly as it appears in the document. Use ISIN (e.g. "FR0000120628", "US0378331005") if present, otherwise use the ticker (e.g. "AAPL"). Set to null for pure cash transactions (deposits, withdrawals, fees, interest with no security).
- "activityType": one of "BUY", "SELL", "DIVIDEND", "INTEREST", "DEPOSIT", "WITHDRAWAL", "TRANSFER_IN", "TRANSFER_OUT", "FEE", "TAX"
- "quantity": number of shares/units as a number, or null for cash-only transactions
- "unitPrice": price per share/unit as a number, or null
- "currency": 3-letter ISO currency code (e.g. "USD", "EUR", "GBP"). This is the transaction currency, NOT a symbol.
- "fee": transaction fee/commission as a number, or null
- "amount": total transaction value as a number, or null
- "comment": brief description from the document (e.g. fund name, transaction label), or null
- "accountName": account name or number if mentioned, or null

Critical rules:
- NEVER use a currency code (EUR, USD, GBP, CHF, etc.) as the "symbol" field. Currency codes go in "currency" only.
- For holdings snapshots: each line item is a BUY with the statement date, the held quantity, unit price, and total market value as amount.
- For dividends/interest on a specific security, include the security's symbol.
- Dates must be valid (e.g. month 1-12, day 1-28/29/30/31). Do NOT invent dates.
- Use positive numbers for quantities, prices, and amounts.
- European number formats: treat comma as decimal separator (e.g. "1.234,56" = 1234.56, "45,63" = 45.63).
- Include ALL line items from the document, not just trades."#;

/// Trait for PDF transaction parsing.
#[async_trait]
pub trait PdfTransactionParserTrait: Send + Sync {
    async fn parse_transactions(
        &self,
        pdf_text: &str,
        provider_id: &str,
        model_id: &str,
    ) -> Result<Vec<ActivityImport>, AiError>;

    async fn parse_transactions_vision(
        &self,
        message: Message,
        provider_id: &str,
        model_id: &str,
    ) -> Result<Vec<ActivityImport>, AiError>;
}

/// PDF transaction parser using LLM providers.
pub struct PdfTransactionParser<E: AiEnvironment> {
    env: Arc<E>,
}

impl<E: AiEnvironment> PdfTransactionParser<E> {
    pub fn new(env: Arc<E>) -> Self {
        Self { env }
    }

    fn build_prompt(pdf_text: &str) -> String {
        // Cap text to ~50 pages worth (~3000 chars/page)
        let max_chars = 150_000;
        let text = if pdf_text.len() > max_chars {
            &pdf_text[..max_chars]
        } else {
            pdf_text
        };

        format!("{}\n\nStatement text:\n{}", PARSE_INSTRUCTIONS, text)
    }

    async fn call_llm(
        &self,
        prompt: &str,
        provider_id: &str,
        model_id: &str,
    ) -> Result<String, AiError> {
        let provider_service = ProviderService::new(self.env.clone());
        let api_key = provider_service.get_api_key(provider_id)?;
        let provider_url = provider_service.get_provider_url(provider_id);

        debug!(
            "Parsing PDF with provider {} model {}",
            provider_id, model_id
        );

        let response = match provider_id {
            "anthropic" => {
                let key = api_key.ok_or_else(|| AiError::MissingApiKey(provider_id.to_string()))?;
                let mut builder = anthropic::Client::<HttpClient>::builder().api_key(&key);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&url);
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(model_id)
                    .max_tokens(16384)
                    .build()
                    .prompt(prompt)
                    .await
                    .map_err(|e| AiError::Provider(e.to_string()))?
            }
            "gemini" | "google" => {
                let key = api_key.ok_or_else(|| AiError::MissingApiKey(provider_id.to_string()))?;
                let mut builder = gemini::Client::<HttpClient>::builder().api_key(&key);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&url);
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(model_id)
                    .build()
                    .prompt(prompt)
                    .await
                    .map_err(|e| AiError::Provider(e.to_string()))?
            }
            "groq" => {
                let key = api_key.ok_or_else(|| AiError::MissingApiKey(provider_id.to_string()))?;
                let mut builder = groq::Client::<HttpClient>::builder().api_key(&key);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&url);
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(model_id)
                    .build()
                    .prompt(prompt)
                    .await
                    .map_err(|e| AiError::Provider(e.to_string()))?
            }
            "ollama" => {
                let mut builder = ollama::Client::<HttpClient>::builder().api_key(Nothing);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&url);
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(model_id)
                    .build()
                    .prompt(prompt)
                    .await
                    .map_err(|e| AiError::Provider(e.to_string()))?
            }
            "openrouter" => {
                let key = api_key.ok_or_else(|| AiError::MissingApiKey(provider_id.to_string()))?;
                let mut builder = openrouter::Client::<HttpClient>::builder().api_key(&key);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&url);
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(model_id)
                    .build()
                    .prompt(prompt)
                    .await
                    .map_err(|e| AiError::Provider(e.to_string()))?
            }
            _ => {
                // Default to OpenAI-compatible
                let key = api_key.ok_or_else(|| AiError::MissingApiKey(provider_id.to_string()))?;
                let mut builder = openai::Client::<HttpClient>::builder().api_key(&key);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&url);
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(model_id)
                    .build()
                    .prompt(prompt)
                    .await
                    .map_err(|e| AiError::Provider(e.to_string()))?
            }
        };

        Ok(response)
    }

    async fn call_llm_message(
        &self,
        message: Message,
        provider_id: &str,
        model_id: &str,
    ) -> Result<String, AiError> {
        let provider_service = ProviderService::new(self.env.clone());
        let api_key = provider_service.get_api_key(provider_id)?;
        let provider_url = provider_service.get_provider_url(provider_id);

        debug!(
            "Parsing PDF (vision) with provider {} model {}",
            provider_id, model_id
        );

        let map_llm_err = |e: rig::completion::PromptError| -> AiError {
            error!("LLM vision call failed: {:?}", e);
            AiError::Provider(e.to_string())
        };

        let response = match provider_id {
            "anthropic" => {
                let key = api_key.ok_or_else(|| AiError::MissingApiKey(provider_id.to_string()))?;
                let mut builder = anthropic::Client::<HttpClient>::builder().api_key(&key);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&url);
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(model_id)
                    .max_tokens(16384)
                    .build()
                    .prompt(message)
                    .await
                    .map_err(map_llm_err)?
            }
            "gemini" | "google" => {
                let key = api_key.ok_or_else(|| AiError::MissingApiKey(provider_id.to_string()))?;
                let mut builder = gemini::Client::<HttpClient>::builder().api_key(&key);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&url);
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(model_id)
                    .build()
                    .prompt(message)
                    .await
                    .map_err(map_llm_err)?
            }
            "groq" => {
                let key = api_key.ok_or_else(|| AiError::MissingApiKey(provider_id.to_string()))?;
                let mut builder = groq::Client::<HttpClient>::builder().api_key(&key);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&url);
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(model_id)
                    .build()
                    .prompt(message)
                    .await
                    .map_err(map_llm_err)?
            }
            "ollama" => {
                let mut builder = ollama::Client::<HttpClient>::builder().api_key(Nothing);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&url);
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(model_id)
                    .build()
                    .prompt(message)
                    .await
                    .map_err(map_llm_err)?
            }
            "openrouter" => {
                let key = api_key.ok_or_else(|| AiError::MissingApiKey(provider_id.to_string()))?;
                let mut builder = openrouter::Client::<HttpClient>::builder().api_key(&key);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&url);
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(model_id)
                    .build()
                    .prompt(message)
                    .await
                    .map_err(map_llm_err)?
            }
            _ => {
                // Default to OpenAI-compatible
                let key = api_key.ok_or_else(|| AiError::MissingApiKey(provider_id.to_string()))?;
                let mut builder = openai::Client::<HttpClient>::builder().api_key(&key);
                if let Some(url) = provider_url {
                    builder = builder.base_url(&url);
                }
                let client = builder
                    .build()
                    .map_err(|e| AiError::Provider(e.to_string()))?;
                client
                    .agent(model_id)
                    .build()
                    .prompt(message)
                    .await
                    .map_err(map_llm_err)?
            }
        };

        Ok(response)
    }
}

/// Extract JSON array from LLM response, handling markdown code blocks.
fn extract_json(raw: &str) -> Result<Vec<RawPdfTransaction>, AiError> {
    let trimmed = raw.trim();

    // Try direct parse
    if let Ok(txns) = serde_json::from_str::<Vec<RawPdfTransaction>>(trimmed) {
        return Ok(txns);
    }

    // Strip markdown code blocks
    let stripped = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .map(|s| s.trim());

    if let Some(json_str) = stripped {
        if let Ok(txns) = serde_json::from_str::<Vec<RawPdfTransaction>>(json_str) {
            return Ok(txns);
        }
    }

    Err(AiError::Provider(format!(
        "Failed to parse LLM response as JSON transaction array: {}",
        &trimmed[..trimmed.len().min(200)]
    )))
}

/// Convert raw parsed transactions to ActivityImport structs.
fn to_activity_imports(raw: Vec<RawPdfTransaction>, line_start: i32) -> Vec<ActivityImport> {
    raw.into_iter()
        .enumerate()
        .map(|(i, tx)| ActivityImport {
            id: None,
            date: tx.date,
            symbol: tx.symbol.unwrap_or_default(),
            activity_type: tx.activity_type,
            quantity: tx.quantity,
            unit_price: tx.unit_price,
            currency: tx.currency,
            fee: tx.fee,
            amount: tx.amount,
            comment: tx.comment,
            account_id: None,
            account_name: tx.account_name,
            symbol_name: None,
            exchange_mic: None,
            quote_ccy: None,
            instrument_type: None,
            errors: None,
            warnings: None,
            duplicate_of_id: None,
            duplicate_of_line_number: None,
            is_draft: true,
            is_valid: false,
            line_number: Some(line_start + i as i32),
            fx_rate: None,
            subtype: None,
        })
        .collect()
}

#[async_trait]
impl<E: AiEnvironment + 'static> PdfTransactionParserTrait for PdfTransactionParser<E> {
    async fn parse_transactions(
        &self,
        pdf_text: &str,
        provider_id: &str,
        model_id: &str,
    ) -> Result<Vec<ActivityImport>, AiError> {
        let prompt = Self::build_prompt(pdf_text);

        let response = self.call_llm(&prompt, provider_id, model_id).await?;

        let raw = extract_json(&response).map_err(|e| {
            warn!("PDF parse failed: {}", e);
            e
        })?;

        debug!("Parsed {} transactions from PDF", raw.len());

        Ok(to_activity_imports(raw, 1))
    }

    async fn parse_transactions_vision(
        &self,
        message: Message,
        provider_id: &str,
        model_id: &str,
    ) -> Result<Vec<ActivityImport>, AiError> {
        let response = self.call_llm_message(message, provider_id, model_id).await?;

        let raw = extract_json(&response).map_err(|e| {
            warn!("PDF vision parse failed: {}", e);
            e
        })?;

        debug!("Parsed {} transactions from PDF (vision)", raw.len());

        Ok(to_activity_imports(raw, 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_direct() {
        let json = r#"[{"date":"2024-01-15","symbol":"AAPL","activityType":"BUY","quantity":10,"unitPrice":185.50,"currency":"USD","fee":4.95,"amount":1859.95,"comment":"Buy AAPL","accountName":"Brokerage"}]"#;
        let result = extract_json(json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].date, "2024-01-15");
    }

    #[test]
    fn test_extract_json_code_block() {
        let json = "```json\n[{\"date\":\"2024-01-15\",\"symbol\":null,\"activityType\":\"DEPOSIT\",\"quantity\":null,\"unitPrice\":null,\"currency\":\"USD\",\"fee\":null,\"amount\":5000,\"comment\":\"Wire transfer\",\"accountName\":null}]\n```";
        let result = extract_json(json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].activity_type, "DEPOSIT");
    }

    #[test]
    fn test_extract_json_invalid() {
        let result = extract_json("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn test_to_activity_imports() {
        let raw = vec![RawPdfTransaction {
            date: "2024-01-15".to_string(),
            symbol: Some("AAPL".to_string()),
            activity_type: "BUY".to_string(),
            quantity: Some(Decimal::new(10, 0)),
            unit_price: Some(Decimal::new(18550, 2)),
            currency: "USD".to_string(),
            fee: Some(Decimal::new(495, 2)),
            amount: Some(Decimal::new(185995, 2)),
            comment: None,
            account_name: None,
        }];
        let imports = to_activity_imports(raw, 1);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].symbol, "AAPL");
        assert!(imports[0].is_draft);
        assert!(!imports[0].is_valid);
        assert_eq!(imports[0].line_number, Some(1));
    }
}
