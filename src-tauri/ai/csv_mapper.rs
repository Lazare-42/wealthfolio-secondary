use super::AIProvider;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct MappingSuggestion {
    pub field: String,
    pub suggested_column: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MappingSuggestions {
    pub suggestions: Vec<MappingSuggestion>,
}

/// Generate AI-powered column mapping suggestions
pub async fn suggest_column_mappings(
    provider: &dyn AIProvider,
    headers: Vec<String>,
    sample_rows: Vec<HashMap<String, String>>,
) -> Result<MappingSuggestions, Box<dyn std::error::Error + Send + Sync>> {
    let prompt = build_mapping_prompt(&headers, &sample_rows);
    let response = provider.complete(prompt).await?;

    // Parse the AI response
    parse_mapping_response(&response, &headers)
}

fn build_mapping_prompt(headers: &[String], sample_rows: &[HashMap<String, String>]) -> String {
    let headers_list = headers.join(", ");

    let sample_data = sample_rows
        .iter()
        .take(3)
        .enumerate()
        .map(|(i, row)| {
            let row_data: Vec<String> = headers
                .iter()
                .map(|h| format!("{}: {}", h, row.get(h).unwrap_or(&"".to_string())))
                .collect();
            format!("Row {}: {}", i + 1, row_data.join(" | "))
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"You are a financial data mapping assistant. Given CSV headers and sample data, map them to Wealthfolio's import format.

CSV Headers: {headers_list}

Sample Data:
{sample_data}

Required Fields:
- date: Transaction date (ISO 8601: YYYY-MM-DD)
- activityType: Transaction type (BUY, SELL, DIVIDEND, DEPOSIT, WITHDRAWAL, FEE, INTEREST)
- symbol: Ticker symbol or ISIN
- quantity: Number of shares
- unitPrice: Price per share
- currency: 3-letter currency code
- fee: Transaction fee
- amount: Total transaction amount
- comment: Optional notes

Optional Fields:
- isin: ISIN code (if different from symbol column)
- name: Asset name
- account: Account identifier

Respond ONLY with a JSON object in this exact format:
{{
  "mappings": [
    {{"field": "date", "column": "transaction_date", "confidence": 0.95}},
    {{"field": "symbol", "column": "isin", "confidence": 0.90}}
  ]
}}

Rules:
1. Only map to columns that exist in the CSV headers
2. Confidence should be 0-1 (higher = more certain)
3. If a field like "isin" exists and "symbol" doesn't, map "symbol" to "isin"
4. Return ONLY the JSON, no explanation"#
    )
}

fn parse_mapping_response(
    response: &str,
    available_headers: &[String],
) -> Result<MappingSuggestions, Box<dyn std::error::Error + Send + Sync>> {
    // Extract JSON from potential markdown code blocks
    let json_str = response
        .trim()
        .strip_prefix("```json")
        .unwrap_or(response)
        .strip_prefix("```")
        .unwrap_or(response)
        .strip_suffix("```")
        .unwrap_or(response)
        .trim();

    #[derive(Deserialize)]
    struct AIResponse {
        mappings: Vec<AIMapping>,
    }

    #[derive(Deserialize)]
    struct AIMapping {
        field: String,
        column: String,
        confidence: f32,
    }

    let parsed: AIResponse = serde_json::from_str(json_str)?;

    // Validate that suggested columns actually exist
    let suggestions = parsed
        .mappings
        .into_iter()
        .filter(|m| available_headers.contains(&m.column))
        .map(|m| MappingSuggestion {
            field: m.field,
            suggested_column: m.column,
            confidence: m.confidence,
        })
        .collect();

    Ok(MappingSuggestions { suggestions })
}
