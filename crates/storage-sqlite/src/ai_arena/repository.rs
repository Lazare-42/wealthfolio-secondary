use async_trait::async_trait;
use diesel::prelude::*;
use diesel::r2d2::{self, Pool};
use diesel::sqlite::SqliteConnection;
use std::sync::Arc;

use crate::db::{get_connection, WriteHandle};
use crate::errors::StorageError;
use crate::schema::{
    arena_agents, arena_challenges, arena_participants, arena_results, arena_runs, arena_snapshots,
    arena_trades, company_theses,
};
use wealthfolio_core::ai_arena::{
    AiArenaRepositoryTrait, ArenaAgent, ArenaChallenge, ArenaParticipant, ArenaResult, ArenaRun,
    ArenaRunUpdate, ArenaSnapshot, ArenaTrade, CompanyThesis,
};
use wealthfolio_core::Result;

use super::model::{
    ArenaAgentDB, ArenaChallengeDB, ArenaParticipantDB, ArenaResultDB, ArenaRunDB, ArenaSnapshotDB,
    ArenaTradeDB, CompanyThesisDB,
};

pub struct AiArenaRepository {
    pool: Arc<Pool<r2d2::ConnectionManager<SqliteConnection>>>,
    writer: WriteHandle,
}

impl AiArenaRepository {
    pub fn new(
        pool: Arc<Pool<r2d2::ConnectionManager<SqliteConnection>>>,
        writer: WriteHandle,
    ) -> Self {
        Self { pool, writer }
    }
}

#[async_trait]
impl AiArenaRepositoryTrait for AiArenaRepository {
    async fn create_agent(&self, agent: ArenaAgent) -> Result<ArenaAgent> {
        let db = ArenaAgentDB::from(agent);
        self.writer
            .exec_tx(move |tx| {
                diesel::insert_into(arena_agents::table)
                    .values(&db)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                Ok(ArenaAgent::from(db))
            })
            .await
    }

    async fn update_agent(&self, agent: ArenaAgent) -> Result<ArenaAgent> {
        let db = ArenaAgentDB::from(agent);
        let id = db.id.clone();
        self.writer
            .exec_tx(move |tx| {
                diesel::update(arena_agents::table.find(&id))
                    .set(&db)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                Ok(ArenaAgent::from(db))
            })
            .await
    }

    async fn delete_agent(&self, id: &str) -> Result<usize> {
        let id = id.to_string();
        self.writer
            .exec_tx(move |tx| {
                diesel::delete(arena_agents::table.find(id))
                    .execute(tx.conn())
                    .map_err(StorageError::from)
                    .map_err(Into::into)
            })
            .await
    }

    fn get_agent(&self, id: &str) -> Result<ArenaAgent> {
        let mut conn = get_connection(&self.pool)?;
        let db = arena_agents::table
            .find(id)
            .select(ArenaAgentDB::as_select())
            .first::<ArenaAgentDB>(&mut conn)
            .map_err(StorageError::from)?;
        Ok(ArenaAgent::from(db))
    }

    fn list_agents(&self) -> Result<Vec<ArenaAgent>> {
        let mut conn = get_connection(&self.pool)?;
        arena_agents::table
            .select(ArenaAgentDB::as_select())
            .order((arena_agents::updated_at.desc(), arena_agents::name.asc()))
            .load::<ArenaAgentDB>(&mut conn)
            .map_err(StorageError::from)?
            .into_iter()
            .map(ArenaAgent::from)
            .collect::<Vec<_>>()
            .pipe(Ok)
    }

    async fn create_challenge(&self, challenge: ArenaChallenge) -> Result<ArenaChallenge> {
        let db = ArenaChallengeDB::try_from(challenge)?;
        self.writer
            .exec_tx(move |tx| {
                diesel::insert_into(arena_challenges::table)
                    .values(&db)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                ArenaChallenge::try_from(db)
            })
            .await
    }

    async fn update_challenge(&self, challenge: ArenaChallenge) -> Result<ArenaChallenge> {
        let db = ArenaChallengeDB::try_from(challenge)?;
        let id = db.id.clone();
        self.writer
            .exec_tx(move |tx| {
                diesel::update(arena_challenges::table.find(&id))
                    .set(&db)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                ArenaChallenge::try_from(db)
            })
            .await
    }

    fn get_challenge(&self, id: &str) -> Result<ArenaChallenge> {
        let mut conn = get_connection(&self.pool)?;
        arena_challenges::table
            .find(id)
            .select(ArenaChallengeDB::as_select())
            .first::<ArenaChallengeDB>(&mut conn)
            .map_err(StorageError::from)?
            .try_into()
    }

    fn list_challenges(&self) -> Result<Vec<ArenaChallenge>> {
        let mut conn = get_connection(&self.pool)?;
        arena_challenges::table
            .select(ArenaChallengeDB::as_select())
            .order((
                arena_challenges::updated_at.desc(),
                arena_challenges::name.asc(),
            ))
            .load::<ArenaChallengeDB>(&mut conn)
            .map_err(StorageError::from)?
            .into_iter()
            .map(ArenaChallenge::try_from)
            .collect()
    }

    async fn create_participant(&self, participant: ArenaParticipant) -> Result<ArenaParticipant> {
        let db = ArenaParticipantDB::from(participant);
        self.writer
            .exec_tx(move |tx| {
                diesel::insert_into(arena_participants::table)
                    .values(&db)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                ArenaParticipant::try_from(db)
            })
            .await
    }

    fn get_participant(
        &self,
        challenge_id: &str,
        agent_id: &str,
    ) -> Result<Option<ArenaParticipant>> {
        let mut conn = get_connection(&self.pool)?;
        arena_participants::table
            .filter(arena_participants::challenge_id.eq(challenge_id))
            .filter(arena_participants::agent_id.eq(agent_id))
            .select(ArenaParticipantDB::as_select())
            .first::<ArenaParticipantDB>(&mut conn)
            .optional()
            .map_err(StorageError::from)?
            .map(ArenaParticipant::try_from)
            .transpose()
    }

    fn get_participant_by_id(&self, id: &str) -> Result<ArenaParticipant> {
        let mut conn = get_connection(&self.pool)?;
        arena_participants::table
            .find(id)
            .select(ArenaParticipantDB::as_select())
            .first::<ArenaParticipantDB>(&mut conn)
            .map_err(StorageError::from)?
            .try_into()
    }

    fn list_participants(&self, challenge_id: &str) -> Result<Vec<ArenaParticipant>> {
        let mut conn = get_connection(&self.pool)?;
        arena_participants::table
            .filter(arena_participants::challenge_id.eq(challenge_id))
            .select(ArenaParticipantDB::as_select())
            .order(arena_participants::joined_at.asc())
            .load::<ArenaParticipantDB>(&mut conn)
            .map_err(StorageError::from)?
            .into_iter()
            .map(ArenaParticipant::try_from)
            .collect()
    }

    async fn create_run(&self, run: ArenaRun) -> Result<ArenaRun> {
        let db = ArenaRunDB::try_from(run)?;
        self.writer
            .exec_tx(move |tx| {
                diesel::insert_into(arena_runs::table)
                    .values(&db)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                ArenaRun::try_from(db)
            })
            .await
    }

    async fn update_run(&self, update: ArenaRunUpdate) -> Result<ArenaRun> {
        let parsed_json = update
            .parsed_json
            .map(|value| serde_json::to_string(&value))
            .transpose()
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.writer
            .exec_tx(move |tx| {
                diesel::update(arena_runs::table.find(&update.id))
                    .set((
                        arena_runs::status.eq(update.status.as_str()),
                        arena_runs::raw_response.eq(update.raw_response),
                        arena_runs::parsed_json.eq(parsed_json),
                        arena_runs::error.eq(update.error),
                        arena_runs::completed_at.eq(update.completed_at),
                    ))
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                arena_runs::table
                    .find(&update.id)
                    .select(ArenaRunDB::as_select())
                    .first::<ArenaRunDB>(tx.conn())
                    .map_err(StorageError::from)?
                    .try_into()
            })
            .await
    }

    fn get_run_by_idempotency_key(&self, key: &str) -> Result<Option<ArenaRun>> {
        let mut conn = get_connection(&self.pool)?;
        arena_runs::table
            .filter(arena_runs::idempotency_key.eq(key))
            .select(ArenaRunDB::as_select())
            .first::<ArenaRunDB>(&mut conn)
            .optional()
            .map_err(StorageError::from)?
            .map(ArenaRun::try_from)
            .transpose()
    }

    fn list_runs(&self, challenge_id: &str) -> Result<Vec<ArenaRun>> {
        let mut conn = get_connection(&self.pool)?;
        arena_runs::table
            .filter(arena_runs::challenge_id.eq(challenge_id))
            .select(ArenaRunDB::as_select())
            .order(arena_runs::started_at.desc())
            .load::<ArenaRunDB>(&mut conn)
            .map_err(StorageError::from)?
            .into_iter()
            .map(ArenaRun::try_from)
            .collect()
    }

    async fn create_trade(&self, trade: ArenaTrade) -> Result<ArenaTrade> {
        let db = ArenaTradeDB::from(trade);
        self.writer
            .exec_tx(move |tx| {
                diesel::insert_into(arena_trades::table)
                    .values(&db)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                ArenaTrade::try_from(db)
            })
            .await
    }

    fn list_trades_for_participant(&self, participant_id: &str) -> Result<Vec<ArenaTrade>> {
        let mut conn = get_connection(&self.pool)?;
        arena_trades::table
            .filter(arena_trades::participant_id.eq(participant_id))
            .select(ArenaTradeDB::as_select())
            .order((arena_trades::executed_at.asc(), arena_trades::id.asc()))
            .load::<ArenaTradeDB>(&mut conn)
            .map_err(StorageError::from)?
            .into_iter()
            .map(ArenaTrade::try_from)
            .collect()
    }

    fn list_trades_for_challenge(&self, challenge_id: &str) -> Result<Vec<ArenaTrade>> {
        let mut conn = get_connection(&self.pool)?;
        arena_trades::table
            .filter(arena_trades::challenge_id.eq(challenge_id))
            .select(ArenaTradeDB::as_select())
            .order(arena_trades::executed_at.desc())
            .load::<ArenaTradeDB>(&mut conn)
            .map_err(StorageError::from)?
            .into_iter()
            .map(ArenaTrade::try_from)
            .collect()
    }

    async fn upsert_snapshot(&self, snapshot: ArenaSnapshot) -> Result<ArenaSnapshot> {
        let db = ArenaSnapshotDB::try_from(snapshot)?;
        let participant_id = db.participant_id.clone();
        let snapshot_date = db.snapshot_date.clone();
        self.writer
            .exec_tx(move |tx| {
                diesel::delete(
                    arena_snapshots::table
                        .filter(arena_snapshots::participant_id.eq(&participant_id))
                        .filter(arena_snapshots::snapshot_date.eq(&snapshot_date)),
                )
                .execute(tx.conn())
                .map_err(StorageError::from)?;
                diesel::insert_into(arena_snapshots::table)
                    .values(&db)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                ArenaSnapshot::try_from(db)
            })
            .await
    }

    fn list_snapshots_for_participant(&self, participant_id: &str) -> Result<Vec<ArenaSnapshot>> {
        let mut conn = get_connection(&self.pool)?;
        arena_snapshots::table
            .filter(arena_snapshots::participant_id.eq(participant_id))
            .select(ArenaSnapshotDB::as_select())
            .order(arena_snapshots::snapshot_date.asc())
            .load::<ArenaSnapshotDB>(&mut conn)
            .map_err(StorageError::from)?
            .into_iter()
            .map(ArenaSnapshot::try_from)
            .collect()
    }

    async fn replace_results(
        &self,
        challenge_id: &str,
        results: Vec<ArenaResult>,
    ) -> Result<Vec<ArenaResult>> {
        let challenge_id = challenge_id.to_string();
        let db_results: Vec<ArenaResultDB> = results
            .into_iter()
            .map(ArenaResultDB::try_from)
            .collect::<Result<Vec<_>>>()?;
        self.writer
            .exec_tx(move |tx| {
                diesel::delete(
                    arena_results::table.filter(arena_results::challenge_id.eq(&challenge_id)),
                )
                .execute(tx.conn())
                .map_err(StorageError::from)?;
                if !db_results.is_empty() {
                    diesel::insert_into(arena_results::table)
                        .values(&db_results)
                        .execute(tx.conn())
                        .map_err(StorageError::from)?;
                }
                db_results
                    .into_iter()
                    .map(ArenaResult::try_from)
                    .collect::<Result<Vec<_>>>()
            })
            .await
    }

    fn list_results(&self, challenge_id: &str) -> Result<Vec<ArenaResult>> {
        let mut conn = get_connection(&self.pool)?;
        arena_results::table
            .filter(arena_results::challenge_id.eq(challenge_id))
            .select(ArenaResultDB::as_select())
            .order(arena_results::rank.asc())
            .load::<ArenaResultDB>(&mut conn)
            .map_err(StorageError::from)?
            .into_iter()
            .map(ArenaResult::try_from)
            .collect()
    }

    async fn create_thesis(&self, thesis: CompanyThesis) -> Result<CompanyThesis> {
        let db = CompanyThesisDB::try_from(thesis)?;
        self.writer
            .exec_tx(move |tx| {
                diesel::insert_into(company_theses::table)
                    .values(&db)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                CompanyThesis::try_from(db)
            })
            .await
    }

    fn list_theses(
        &self,
        symbol: Option<&str>,
        challenge_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<CompanyThesis>> {
        let mut conn = get_connection(&self.pool)?;
        let mut query = company_theses::table.into_boxed();
        if let Some(symbol) = symbol.map(wealthfolio_core::ai_arena::normalize_symbol) {
            query = query.filter(company_theses::symbol.eq(symbol));
        }
        if let Some(challenge_id) = challenge_id {
            query = query.filter(company_theses::challenge_id.eq(challenge_id));
        }
        query
            .select(CompanyThesisDB::as_select())
            .order(company_theses::created_at.desc())
            .limit(limit.clamp(1, 200))
            .load::<CompanyThesisDB>(&mut conn)
            .map_err(StorageError::from)?
            .into_iter()
            .map(CompanyThesis::try_from)
            .collect()
    }
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}

impl<T> Pipe for T {}
