use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::errors::StorageError;
use wealthfolio_core::portfolios::AccountScope;
use wealthfolio_core::scenarios::{
    BasketPosition, PortfolioScenario, PortfolioScenarioRecord, ScenarioKind,
};
use wealthfolio_core::{Error, Result};

#[derive(
    Queryable,
    Identifiable,
    Insertable,
    AsChangeset,
    Selectable,
    Serialize,
    Deserialize,
    Debug,
    Clone,
)]
#[diesel(table_name = crate::schema::portfolio_scenarios)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PortfolioScenarioDB {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub account_scope_json: String,
    pub resolved_account_ids_json: String,
    pub as_of_date: Option<String>,
    pub benchmark_symbols_json: String,
    pub assumptions_json: String,
    pub created_at: String,
    pub updated_at: String,
    pub kind: String,
    pub basket_json: String,
}

impl PortfolioScenarioDB {
    pub fn from_record(record: PortfolioScenarioRecord) -> Result<Self> {
        Ok(Self {
            id: record.id,
            name: record.name,
            description: record.description,
            account_scope_json: serde_json::to_string(&record.account_scope)
                .map_err(|e| Error::from(StorageError::SerializationError(e.to_string())))?,
            resolved_account_ids_json: serde_json::to_string(&record.resolved_account_ids)
                .map_err(|e| Error::from(StorageError::SerializationError(e.to_string())))?,
            as_of_date: record.as_of_date,
            benchmark_symbols_json: serde_json::to_string(&record.benchmark_symbols)
                .map_err(|e| Error::from(StorageError::SerializationError(e.to_string())))?,
            assumptions_json: serde_json::to_string(&record.assumptions)
                .map_err(|e| Error::from(StorageError::SerializationError(e.to_string())))?,
            created_at: record.created_at,
            updated_at: record.updated_at,
            kind: record.kind.as_str().to_string(),
            basket_json: serde_json::to_string(&record.basket)
                .map_err(|e| Error::from(StorageError::SerializationError(e.to_string())))?,
        })
    }

    pub fn into_domain(self) -> Result<PortfolioScenario> {
        Ok(PortfolioScenario {
            id: self.id,
            name: self.name,
            description: self.description,
            account_scope: serde_json::from_str::<AccountScope>(&self.account_scope_json)
                .map_err(|e| Error::from(StorageError::SerializationError(e.to_string())))?,
            resolved_account_ids: serde_json::from_str::<Vec<String>>(
                &self.resolved_account_ids_json,
            )
            .map_err(|e| Error::from(StorageError::SerializationError(e.to_string())))?,
            as_of_date: self.as_of_date,
            benchmark_symbols: serde_json::from_str::<Vec<String>>(&self.benchmark_symbols_json)
                .map_err(|e| Error::from(StorageError::SerializationError(e.to_string())))?,
            basket: serde_json::from_str::<Vec<BasketPosition>>(&self.basket_json)
                .map_err(|e| Error::from(StorageError::SerializationError(e.to_string())))?,
            assumptions: serde_json::from_str(&self.assumptions_json)
                .map_err(|e| Error::from(StorageError::SerializationError(e.to_string())))?,
            kind: ScenarioKind::parse(&self.kind),
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn record() -> PortfolioScenarioRecord {
        PortfolioScenarioRecord {
            id: "s-1".to_string(),
            name: "Core".to_string(),
            description: Some("notes".to_string()),
            account_scope: AccountScope::Accounts {
                account_ids: vec!["a-1".to_string(), "a-2".to_string()],
            },
            kind: ScenarioKind::Basket,
            resolved_account_ids: vec!["a-1".to_string(), "a-2".to_string()],
            as_of_date: Some("2026-06-28".to_string()),
            benchmark_symbols: vec!["SPY".to_string(), "QQQ".to_string()],
            basket: vec![
                BasketPosition {
                    symbol: "SPY".to_string(),
                    weight: 60.0,
                },
                BasketPosition {
                    symbol: "QQQ".to_string(),
                    weight: 40.0,
                },
            ],
            assumptions: json!({ "note": "rebalance quarterly" }),
            created_at: "2026-06-28T00:00:00Z".to_string(),
            updated_at: "2026-06-28T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn record_db_domain_roundtrip_preserves_json_fields() {
        let original = record();
        let db = PortfolioScenarioDB::from_record(original.clone()).unwrap();
        let domain = db.into_domain().unwrap();

        assert_eq!(domain.id, original.id);
        assert_eq!(domain.name, original.name);
        assert_eq!(domain.description, original.description);
        // AccountScope has no PartialEq; compare its serialized form.
        assert_eq!(
            serde_json::to_value(&domain.account_scope).unwrap(),
            serde_json::to_value(&original.account_scope).unwrap(),
        );
        assert_eq!(domain.resolved_account_ids, original.resolved_account_ids);
        assert_eq!(domain.as_of_date, original.as_of_date);
        assert_eq!(domain.benchmark_symbols, original.benchmark_symbols);
        assert_eq!(domain.assumptions, original.assumptions);
        assert_eq!(domain.kind, original.kind);
        assert_eq!(
            serde_json::to_value(&domain.basket).unwrap(),
            serde_json::to_value(&original.basket).unwrap(),
        );
    }

    #[test]
    fn into_domain_rejects_corrupt_scope_json() {
        let mut db = PortfolioScenarioDB::from_record(record()).unwrap();
        db.account_scope_json = "not json".to_string();
        assert!(db.into_domain().is_err());
    }
}
