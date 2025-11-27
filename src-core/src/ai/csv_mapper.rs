use ai_lib::{AiClient, ChatCompletionRequest, Content, Message, Role};
use schemars::schema_for;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::activities::ActivityImport;

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
    client: &AiClient,
    model: &str,
    headers: Vec<String>,
    sample_rows: Vec<HashMap<String, String>>,
) -> Result<MappingSuggestions, Box<dyn std::error::Error + Send + Sync>> {
    let prompt = build_mapping_prompt(&headers, &sample_rows);

    let request = ChatCompletionRequest::new(
        model.to_string(),
        vec![Message {
            role: Role::User,
            content: Content::Text(prompt),
            function_call: None,
        }],
    );

    let response = client.chat_completion(request).await?;
    let content = response
        .choices
        .first()
        .map(|c| c.message.content.as_text())
        .unwrap_or_default();

    // Parse the AI response
    parse_mapping_response(&content, &headers)
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

    // Generate JSON schema from ActivityImport struct
    let schema = schema_for!(ActivityImport);
    let schema_json = serde_json::to_string_pretty(&schema).unwrap_or_default();

    format!(
        r#"You are a financial data mapping assistant. Given CSV headers and sample data, map them to Wealthfolio's ActivityImport schema.

CSV Headers: {headers_list}

Sample Data:
{sample_data}

TARGET SCHEMA (ActivityImport):
The following JSON Schema defines the exact structure and types expected:

{schema_json}

KEY FIELD DESCRIPTIONS:
- date (String): Transaction date in ISO 8601 format (YYYY-MM-DD)
- symbol (String): Asset ticker symbol or ISIN
- activity_type (String): Type of activity - must be one of: BUY, SELL, DIVIDEND, INTEREST, FEE, TRANSFER_IN, TRANSFER_OUT, CONVERSION_IN, CONVERSION_OUT, DEPOSIT, WITHDRAWAL, TAX, SPLIT
- quantity (Decimal): Number of shares/units (numeric)
- unit_price (Decimal): Price per share/unit (numeric)
- currency (String): 3-letter ISO currency code (USD, EUR, GBP, etc.)
- fee (Decimal): Transaction fee amount (numeric, default 0)
- amount (Decimal, optional): Total transaction amount (numeric)
- comment (String, optional): Additional notes or description
- account_id (String, optional): Internal account identifier
- account_name (String, optional): Human-readable account name
- symbol_name (String, optional): Asset name or description

Respond ONLY with a JSON object in this exact format:
{{
  "mappings": [
    {{"field": "date", "column": "transaction_date", "confidence": 0.95}},
    {{"field": "symbol", "column": "isin", "confidence": 0.90}}
  ]
}}

MAPPING RULES:
1. Only map to columns that exist in the CSV headers
2. Confidence should be 0.0-1.0 (higher = more certain)
3. If a CSV has "isin" but no "symbol" column, map "symbol" to "isin"
4. For activity_type, look for columns like "type", "transaction_type", "operation"
5. For numeric fields (quantity, unit_price, fee, amount), ensure the CSV column contains numbers
6. For date, look for columns with "date", "time", "timestamp" in the name
7. Use the JSON Schema above as the authoritative source for field names and types
8. Return ONLY the JSON, no explanation or markdown formatting"#
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
