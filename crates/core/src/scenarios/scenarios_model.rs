use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::portfolios::AccountScope;
use crate::{errors::ValidationError, Error, Result};

pub type ScenarioAssumptions = Value;

/// Max benchmark symbols a scenario may hold. Matches the comparison tools'
/// `MAX_COMPARE_SYMBOLS` so every saved scenario stays comparable.
pub const MAX_BENCHMARK_SYMBOLS: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioScenario {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub account_scope: AccountScope,
    pub resolved_account_ids: Vec<String>,
    pub as_of_date: Option<String>,
    pub benchmark_symbols: Vec<String>,
    pub assumptions: ScenarioAssumptions,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewPortfolioScenario {
    pub name: String,
    pub description: Option<String>,
    pub account_scope: AccountScope,
    #[serde(default)]
    pub as_of_date: Option<String>,
    #[serde(default)]
    pub benchmark_symbols: Vec<String>,
    #[serde(default = "default_assumptions")]
    pub assumptions: ScenarioAssumptions,
}

impl NewPortfolioScenario {
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(Error::Validation(ValidationError::InvalidInput(
                "Scenario name cannot be empty".to_string(),
            )));
        }

        if let Some(as_of_date) = self.as_of_date.as_deref().filter(|value| !value.is_empty()) {
            NaiveDate::parse_from_str(as_of_date, "%Y-%m-%d").map_err(|_| {
                Error::Validation(ValidationError::InvalidInput(
                    "asOfDate must be a date in YYYY-MM-DD format".to_string(),
                ))
            })?;
        }

        // Capped at the comparison-tool limit so a saved scenario can never
        // hold more benchmarks than `compare_saved_scenario` can replay.
        if self.benchmark_symbols.len() > MAX_BENCHMARK_SYMBOLS {
            return Err(Error::Validation(ValidationError::InvalidInput(format!(
                "Scenario can contain at most {MAX_BENCHMARK_SYMBOLS} benchmark symbols"
            ))));
        }

        let mut seen = std::collections::HashSet::new();
        for symbol in &self.benchmark_symbols {
            let trimmed = symbol.trim();
            if trimmed.is_empty() {
                return Err(Error::Validation(ValidationError::InvalidInput(
                    "Benchmark symbols cannot be empty".to_string(),
                )));
            }
            if !seen.insert(trimmed.to_ascii_uppercase()) {
                return Err(Error::Validation(ValidationError::InvalidInput(format!(
                    "Duplicate benchmark symbol: {trimmed}",
                ))));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PortfolioScenarioRecord {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub account_scope: AccountScope,
    pub resolved_account_ids: Vec<String>,
    pub as_of_date: Option<String>,
    pub benchmark_symbols: Vec<String>,
    pub assumptions: ScenarioAssumptions,
    pub created_at: String,
    pub updated_at: String,
}

impl PortfolioScenarioRecord {
    pub fn new(new: NewPortfolioScenario, resolved_account_ids: Vec<String>) -> Self {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        Self {
            id: Uuid::new_v4().to_string(),
            name: new.name.trim().to_string(),
            description: new
                .description
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            account_scope: new.account_scope,
            resolved_account_ids,
            as_of_date: new
                .as_of_date
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            benchmark_symbols: normalize_benchmark_symbols(new.benchmark_symbols),
            assumptions: new.assumptions,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

impl From<PortfolioScenarioRecord> for PortfolioScenario {
    fn from(value: PortfolioScenarioRecord) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            account_scope: value.account_scope,
            resolved_account_ids: value.resolved_account_ids,
            as_of_date: value.as_of_date,
            benchmark_symbols: value.benchmark_symbols,
            assumptions: value.assumptions,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

fn default_assumptions() -> ScenarioAssumptions {
    Value::Object(serde_json::Map::new())
}

fn normalize_benchmark_symbols(symbols: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for symbol in symbols {
        let trimmed = symbol.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_ascii_uppercase();
        if seen.insert(key) {
            out.push(trimmed.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario(name: &str, symbols: &[&str]) -> NewPortfolioScenario {
        NewPortfolioScenario {
            name: name.to_string(),
            description: None,
            account_scope: AccountScope::All,
            as_of_date: None,
            benchmark_symbols: symbols.iter().map(|s| s.to_string()).collect(),
            assumptions: default_assumptions(),
        }
    }

    #[test]
    fn rejects_blank_name() {
        assert!(scenario("   ", &[]).validate().is_err());
    }

    #[test]
    fn rejects_bad_as_of_date() {
        let mut s = scenario("ok", &[]);
        s.as_of_date = Some("2026/06/28".to_string());
        assert!(s.validate().is_err());
        s.as_of_date = Some("2026-06-28".to_string());
        assert!(s.validate().is_ok());
        // Empty string is treated as "no date", not an error.
        s.as_of_date = Some(String::new());
        assert!(s.validate().is_ok());
    }

    #[test]
    fn enforces_benchmark_cap() {
        assert!(scenario("ok", &["A", "B", "C", "D", "E"])
            .validate()
            .is_ok());
        assert!(scenario("too many", &["A", "B", "C", "D", "E", "F"])
            .validate()
            .is_err());
    }

    #[test]
    fn rejects_duplicate_and_empty_symbols() {
        assert!(scenario("dup", &["SPY", "spy"]).validate().is_err());
        assert!(scenario("blank", &["SPY", "  "]).validate().is_err());
    }

    #[test]
    fn record_normalizes_symbols_and_trims_fields() {
        let mut s = scenario("  Core  ", &["spy", "QQQ", "spy"]);
        s.description = Some("  notes  ".to_string());
        let record = PortfolioScenarioRecord::new(s, vec!["acct-1".to_string()]);
        assert_eq!(record.name, "Core");
        assert_eq!(record.description.as_deref(), Some("notes"));
        // Case-insensitive dedup keeps first occurrence's casing.
        assert_eq!(record.benchmark_symbols, vec!["spy", "QQQ"]);
        assert_eq!(record.resolved_account_ids, vec!["acct-1"]);
    }
}
