//! PDF transaction parser using rig-core LLM providers.
//!
//! Extracts structured transaction data from PDF text by sending it to an LLM
//! and parsing the JSON response.

use async_trait::async_trait;
use log::{debug, warn};
use reqwest::Client as HttpClient;
use rig::{
    client::{CompletionClient, Nothing},
    completion::Prompt,
    providers::{anthropic, gemini, groq, ollama, openai, openrouter},
};
use serde::{Deserialize, Serialize};

use crate::env::AiEnvironment;
use crate::error::AiError;
use crate::providers::ProviderService;
use std::sync::Arc;

/// A single parsed transaction from a PDF statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfTransaction {
    pub date: String,
    #[serde(rename = "type")]
    pub activity_type: String,
    pub description: String,
    pub amount: f64,
    pub currency: String,
    #[serde(default)]
    pub fee: Option<f64>,
}

/// Trait for parsing PDF text into transactions.
#[async_trait]
pub trait PdfTransactionParserTrait: Send + Sync {
    async fn parse_transactions(
        &self,
        pdf_text: &str,
        provider_id: &str,
        model_id: &str,
    ) -> Result<Vec<PdfTransaction>, AiError>;
}

/// LLM-based PDF transaction parser.
pub struct PdfTransactionParser<E: AiEnvironment> {
    env: Arc<E>,
}

impl<E: AiEnvironment> PdfTransactionParser<E> {
    pub fn new(env: Arc<E>) -> Self {
        Self { env }
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
                    .max_tokens(4096)
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
                // Default: OpenAI-compatible
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
}

/// Build the prompt for PDF transaction extraction.
fn build_extraction_prompt(pdf_text: &str) -> String {
    format!(
        r#"Extract all financial transactions from this bank/brokerage statement text.

Return a JSON array of objects with these fields:
- "date": ISO date string (YYYY-MM-DD)
- "type": one of "BUY", "SELL", "DEPOSIT", "WITHDRAWAL", "DIVIDEND", "INTEREST", "FEE", "TRANSFER_IN", "TRANSFER_OUT"
- "description": short description of the transaction
- "amount": numeric amount (positive for inflows, negative for outflows)
- "currency": 3-letter currency code (e.g. "EUR", "USD")
- "fee": optional numeric fee amount

Return ONLY the JSON array, no markdown fences, no explanation.
If no transactions are found, return an empty array: []

Statement text:
---
{pdf_text}
---"#
    )
}

/// Parse the LLM response into transactions, handling common formatting issues.
fn parse_llm_response(response: &str) -> Result<Vec<PdfTransaction>, AiError> {
    // Strip markdown code fences if present
    let cleaned = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    serde_json::from_str::<Vec<PdfTransaction>>(cleaned)
        .map_err(|e| AiError::Internal(format!("Failed to parse LLM response as JSON: {}", e)))
}

#[async_trait]
impl<E: AiEnvironment + 'static> PdfTransactionParserTrait for PdfTransactionParser<E> {
    async fn parse_transactions(
        &self,
        pdf_text: &str,
        provider_id: &str,
        model_id: &str,
    ) -> Result<Vec<PdfTransaction>, AiError> {
        if pdf_text.trim().len() < 50 {
            warn!(
                "PDF text too short ({} chars), likely a scanned document",
                pdf_text.len()
            );
            return Err(AiError::Internal(
                "PDF text too short — the document may be scanned/image-based. \
                 Please use a text-based PDF statement."
                    .to_string(),
            ));
        }

        let prompt = build_extraction_prompt(pdf_text);
        let response = self.call_llm(&prompt, provider_id, model_id).await?;
        parse_llm_response(&response)
    }
}
