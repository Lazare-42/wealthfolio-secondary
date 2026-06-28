use async_trait::async_trait;
use diesel::prelude::*;
use diesel::r2d2::{self, Pool};
use diesel::sqlite::SqliteConnection;
use std::sync::Arc;

use crate::db::{get_connection, WriteHandle};
use crate::errors::StorageError;
use crate::schema::portfolio_scenarios;
use wealthfolio_core::errors::Result;
use wealthfolio_core::scenarios::{
    PortfolioScenario, PortfolioScenarioRecord, PortfolioScenarioRepositoryTrait,
};

use super::model::PortfolioScenarioDB;

pub struct PortfolioScenarioRepository {
    pool: Arc<Pool<r2d2::ConnectionManager<SqliteConnection>>>,
    writer: WriteHandle,
}

impl PortfolioScenarioRepository {
    pub fn new(
        pool: Arc<Pool<r2d2::ConnectionManager<SqliteConnection>>>,
        writer: WriteHandle,
    ) -> Self {
        Self { pool, writer }
    }
}

#[async_trait]
impl PortfolioScenarioRepositoryTrait for PortfolioScenarioRepository {
    async fn create(&self, record: PortfolioScenarioRecord) -> Result<PortfolioScenario> {
        let scenario_db = PortfolioScenarioDB::from_record(record)?;
        self.writer
            .exec_tx(move |tx| {
                diesel::insert_into(portfolio_scenarios::table)
                    .values(&scenario_db)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                scenario_db.into_domain()
            })
            .await
    }

    async fn update(&self, record: PortfolioScenarioRecord) -> Result<PortfolioScenario> {
        let scenario_db = PortfolioScenarioDB::from_record(record)?;
        let scenario_id = scenario_db.id.clone();
        self.writer
            .exec_tx(move |tx| {
                diesel::update(portfolio_scenarios::table.find(&scenario_id))
                    .set(&scenario_db)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                scenario_db.into_domain()
            })
            .await
    }

    async fn delete(&self, id: &str) -> Result<usize> {
        let id_owned = id.to_string();
        self.writer
            .exec_tx(move |tx| {
                let affected = diesel::delete(portfolio_scenarios::table.find(&id_owned))
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                Ok(affected)
            })
            .await
    }

    fn get_by_id(&self, id: &str) -> Result<PortfolioScenario> {
        let mut conn = get_connection(&self.pool)?;
        portfolio_scenarios::table
            .find(id)
            .select(PortfolioScenarioDB::as_select())
            .first::<PortfolioScenarioDB>(&mut conn)
            .map_err(StorageError::from)?
            .into_domain()
    }

    fn list(&self) -> Result<Vec<PortfolioScenario>> {
        let mut conn = get_connection(&self.pool)?;
        portfolio_scenarios::table
            .select(PortfolioScenarioDB::as_select())
            .order((
                portfolio_scenarios::updated_at.desc(),
                portfolio_scenarios::name.asc(),
            ))
            .load::<PortfolioScenarioDB>(&mut conn)
            .map_err(StorageError::from)?
            .into_iter()
            .map(PortfolioScenarioDB::into_domain)
            .collect()
    }
}
