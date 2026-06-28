use std::sync::Arc;

use crate::accounts::AccountPurpose;
use crate::portfolios::PortfolioServiceTrait;
use crate::scenarios::{
    NewPortfolioScenario, PortfolioScenario, PortfolioScenarioRecord,
    PortfolioScenarioRepositoryTrait, PortfolioScenarioServiceTrait,
};
use crate::Result;

pub struct PortfolioScenarioService {
    repository: Arc<dyn PortfolioScenarioRepositoryTrait>,
    portfolio_service: Arc<dyn PortfolioServiceTrait>,
    base_currency: String,
}

impl PortfolioScenarioService {
    pub fn new(
        repository: Arc<dyn PortfolioScenarioRepositoryTrait>,
        portfolio_service: Arc<dyn PortfolioServiceTrait>,
        base_currency: impl Into<String>,
    ) -> Self {
        Self {
            repository,
            portfolio_service,
            base_currency: base_currency.into(),
        }
    }
}

#[async_trait::async_trait]
impl PortfolioScenarioServiceTrait for PortfolioScenarioService {
    async fn create_scenario(&self, mut new: NewPortfolioScenario) -> Result<PortfolioScenario> {
        new.name = new.name.trim().to_string();
        new.validate()?;
        let resolved = self.portfolio_service.resolve_account_scope_for_purpose(
            &new.account_scope,
            &self.base_currency,
            AccountPurpose::Performance,
        )?;
        let record = PortfolioScenarioRecord::new(new, resolved.account_ids);
        self.repository.create(record).await
    }

    async fn update_scenario(
        &self,
        id: &str,
        mut update: NewPortfolioScenario,
    ) -> Result<PortfolioScenario> {
        let existing = self.repository.get_by_id(id)?;
        update.name = update.name.trim().to_string();
        update.validate()?;
        let resolved = self.portfolio_service.resolve_account_scope_for_purpose(
            &update.account_scope,
            &self.base_currency,
            AccountPurpose::Performance,
        )?;
        let mut record = PortfolioScenarioRecord::new(update, resolved.account_ids);
        record.id = existing.id;
        record.created_at = existing.created_at;
        self.repository.update(record).await
    }

    async fn delete_scenario(&self, id: &str) -> Result<()> {
        self.repository.delete(id).await?;
        Ok(())
    }

    fn get_scenario(&self, id: &str) -> Result<PortfolioScenario> {
        self.repository.get_by_id(id)
    }

    fn list_scenarios(&self) -> Result<Vec<PortfolioScenario>> {
        self.repository.list()
    }
}
