use async_trait::async_trait;

use super::scenarios_model::{NewPortfolioScenario, PortfolioScenario, PortfolioScenarioRecord};
use crate::errors::Result;

#[async_trait]
pub trait PortfolioScenarioRepositoryTrait: Send + Sync {
    async fn create(&self, record: PortfolioScenarioRecord) -> Result<PortfolioScenario>;
    async fn update(&self, record: PortfolioScenarioRecord) -> Result<PortfolioScenario>;
    async fn delete(&self, id: &str) -> Result<usize>;
    fn get_by_id(&self, id: &str) -> Result<PortfolioScenario>;
    fn list(&self) -> Result<Vec<PortfolioScenario>>;
}

#[async_trait]
pub trait PortfolioScenarioServiceTrait: Send + Sync {
    async fn create_scenario(&self, new: NewPortfolioScenario) -> Result<PortfolioScenario>;
    async fn update_scenario(
        &self,
        id: &str,
        update: NewPortfolioScenario,
    ) -> Result<PortfolioScenario>;
    async fn delete_scenario(&self, id: &str) -> Result<()>;
    fn get_scenario(&self, id: &str) -> Result<PortfolioScenario>;
    fn list_scenarios(&self) -> Result<Vec<PortfolioScenario>>;
}
