use diesel::prelude::*;
use serde_json::Value;

use crate::errors::StorageError;
use wealthfolio_core::ai_arena::*;
use wealthfolio_core::{Error, Result};

#[derive(Queryable, Identifiable, Insertable, AsChangeset, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::arena_agents)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ArenaAgentDB {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub model_id: String,
    pub persona: String,
    pub enabled: i32,
    pub schedule_enabled: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ArenaAgent> for ArenaAgentDB {
    fn from(value: ArenaAgent) -> Self {
        Self {
            id: value.id,
            name: value.name,
            provider_id: value.provider_id,
            model_id: value.model_id,
            persona: value.persona,
            enabled: bool_to_i32(value.enabled),
            schedule_enabled: bool_to_i32(value.schedule_enabled),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<ArenaAgentDB> for ArenaAgent {
    fn from(value: ArenaAgentDB) -> Self {
        Self {
            id: value.id,
            name: value.name,
            provider_id: value.provider_id,
            model_id: value.model_id,
            persona: value.persona,
            enabled: value.enabled != 0,
            schedule_enabled: value.schedule_enabled != 0,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Queryable, Identifiable, Insertable, AsChangeset, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::arena_challenges)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ArenaChallengeDB {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub market: String,
    pub scoring_method: String,
    pub initial_cash: String,
    pub max_position_pct: String,
    pub max_drawdown_pct: String,
    pub run_cadence: String,
    pub scheduled_time_local: Option<String>,
    pub universe_json: String,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub settled_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl TryFrom<ArenaChallenge> for ArenaChallengeDB {
    type Error = Error;

    fn try_from(value: ArenaChallenge) -> Result<Self> {
        Ok(Self {
            id: value.id,
            name: value.name,
            description: value.description,
            status: value.status.as_str().to_string(),
            market: value.market,
            scoring_method: value.scoring_method.as_str().to_string(),
            initial_cash: value.initial_cash.to_string(),
            max_position_pct: value.max_position_pct.to_string(),
            max_drawdown_pct: value.max_drawdown_pct.to_string(),
            run_cadence: value.run_cadence,
            scheduled_time_local: value.scheduled_time_local,
            universe_json: to_json(&value.universe)?,
            start_at: value.start_at,
            end_at: value.end_at,
            settled_at: value.settled_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
        })
    }
}

impl TryFrom<ArenaChallengeDB> for ArenaChallenge {
    type Error = Error;

    fn try_from(value: ArenaChallengeDB) -> Result<Self> {
        Ok(Self {
            id: value.id,
            name: value.name,
            description: value.description,
            status: ArenaChallengeStatus::parse(&value.status),
            market: value.market,
            scoring_method: ArenaScoringMethod::parse(&value.scoring_method),
            initial_cash: parse_f64(&value.initial_cash)?,
            max_position_pct: parse_f64(&value.max_position_pct)?,
            max_drawdown_pct: parse_f64(&value.max_drawdown_pct)?,
            run_cadence: value.run_cadence,
            scheduled_time_local: value.scheduled_time_local,
            universe: from_json(&value.universe_json)?,
            start_at: value.start_at,
            end_at: value.end_at,
            settled_at: value.settled_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
        })
    }
}

#[derive(Queryable, Identifiable, Insertable, AsChangeset, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::arena_participants)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ArenaParticipantDB {
    pub id: String,
    pub challenge_id: String,
    pub agent_id: String,
    pub status: String,
    pub joined_at: String,
    pub starting_cash: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ArenaParticipant> for ArenaParticipantDB {
    fn from(value: ArenaParticipant) -> Self {
        Self {
            id: value.id,
            challenge_id: value.challenge_id,
            agent_id: value.agent_id,
            status: value.status,
            joined_at: value.joined_at,
            starting_cash: value.starting_cash.to_string(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl TryFrom<ArenaParticipantDB> for ArenaParticipant {
    type Error = Error;

    fn try_from(value: ArenaParticipantDB) -> Result<Self> {
        Ok(Self {
            id: value.id,
            challenge_id: value.challenge_id,
            agent_id: value.agent_id,
            status: value.status,
            joined_at: value.joined_at,
            starting_cash: parse_f64(&value.starting_cash)?,
            created_at: value.created_at,
            updated_at: value.updated_at,
        })
    }
}

#[derive(Queryable, Identifiable, Insertable, AsChangeset, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::arena_runs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ArenaRunDB {
    pub id: String,
    pub challenge_id: String,
    pub agent_id: String,
    pub participant_id: String,
    pub run_type: String,
    pub status: String,
    pub idempotency_key: Option<String>,
    pub prompt: String,
    pub raw_response: Option<String>,
    pub parsed_json: Option<String>,
    pub error: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

impl TryFrom<ArenaRun> for ArenaRunDB {
    type Error = Error;

    fn try_from(value: ArenaRun) -> Result<Self> {
        Ok(Self {
            id: value.id,
            challenge_id: value.challenge_id,
            agent_id: value.agent_id,
            participant_id: value.participant_id,
            run_type: value.run_type.as_str().to_string(),
            status: value.status.as_str().to_string(),
            idempotency_key: value.idempotency_key,
            prompt: value.prompt,
            raw_response: value.raw_response,
            parsed_json: value.parsed_json.map(|v| to_json(&v)).transpose()?,
            error: value.error,
            started_at: value.started_at,
            completed_at: value.completed_at,
        })
    }
}

impl TryFrom<ArenaRunDB> for ArenaRun {
    type Error = Error;

    fn try_from(value: ArenaRunDB) -> Result<Self> {
        Ok(Self {
            id: value.id,
            challenge_id: value.challenge_id,
            agent_id: value.agent_id,
            participant_id: value.participant_id,
            run_type: ArenaRunType::parse(&value.run_type),
            status: ArenaRunStatus::parse(&value.status),
            idempotency_key: value.idempotency_key,
            prompt: value.prompt,
            raw_response: value.raw_response,
            parsed_json: value.parsed_json.map(|v| from_json(&v)).transpose()?,
            error: value.error,
            started_at: value.started_at,
            completed_at: value.completed_at,
        })
    }
}

#[derive(Queryable, Identifiable, Insertable, AsChangeset, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::arena_trades)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ArenaTradeDB {
    pub id: String,
    pub challenge_id: String,
    pub participant_id: String,
    pub run_id: Option<String>,
    pub symbol: String,
    pub side: String,
    pub quantity: String,
    pub price: String,
    pub notional: String,
    pub status: String,
    pub rationale: Option<String>,
    pub rejection_reason: Option<String>,
    pub executed_at: String,
    pub created_at: String,
}

impl From<ArenaTrade> for ArenaTradeDB {
    fn from(value: ArenaTrade) -> Self {
        Self {
            id: value.id,
            challenge_id: value.challenge_id,
            participant_id: value.participant_id,
            run_id: value.run_id,
            symbol: value.symbol,
            side: value.side.as_str().to_string(),
            quantity: value.quantity.to_string(),
            price: value.price.to_string(),
            notional: value.notional.to_string(),
            status: value.status.as_str().to_string(),
            rationale: value.rationale,
            rejection_reason: value.rejection_reason,
            executed_at: value.executed_at,
            created_at: value.created_at,
        }
    }
}

impl TryFrom<ArenaTradeDB> for ArenaTrade {
    type Error = Error;

    fn try_from(value: ArenaTradeDB) -> Result<Self> {
        Ok(Self {
            id: value.id,
            challenge_id: value.challenge_id,
            participant_id: value.participant_id,
            run_id: value.run_id,
            symbol: value.symbol,
            side: ArenaTradeSide::parse(&value.side).unwrap_or(ArenaTradeSide::Buy),
            quantity: parse_f64(&value.quantity)?,
            price: parse_f64(&value.price)?,
            notional: parse_f64(&value.notional)?,
            status: ArenaTradeStatus::parse(&value.status),
            rationale: value.rationale,
            rejection_reason: value.rejection_reason,
            executed_at: value.executed_at,
            created_at: value.created_at,
        })
    }
}

#[derive(Queryable, Identifiable, Insertable, AsChangeset, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::arena_snapshots)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ArenaSnapshotDB {
    pub id: String,
    pub challenge_id: String,
    pub participant_id: String,
    pub snapshot_date: String,
    pub total_value: String,
    pub cash: String,
    pub return_pct: String,
    pub max_drawdown_pct: String,
    pub positions_json: String,
    pub equity_curve_json: String,
    pub created_at: String,
}

impl TryFrom<ArenaSnapshot> for ArenaSnapshotDB {
    type Error = Error;

    fn try_from(value: ArenaSnapshot) -> Result<Self> {
        Ok(Self {
            id: value.id,
            challenge_id: value.challenge_id,
            participant_id: value.participant_id,
            snapshot_date: value.snapshot_date,
            total_value: value.total_value.to_string(),
            cash: value.cash.to_string(),
            return_pct: value.return_pct.to_string(),
            max_drawdown_pct: value.max_drawdown_pct.to_string(),
            positions_json: to_json(&value.positions)?,
            equity_curve_json: to_json(&value.equity_curve)?,
            created_at: value.created_at,
        })
    }
}

impl TryFrom<ArenaSnapshotDB> for ArenaSnapshot {
    type Error = Error;

    fn try_from(value: ArenaSnapshotDB) -> Result<Self> {
        Ok(Self {
            id: value.id,
            challenge_id: value.challenge_id,
            participant_id: value.participant_id,
            snapshot_date: value.snapshot_date,
            total_value: parse_f64(&value.total_value)?,
            cash: parse_f64(&value.cash)?,
            return_pct: parse_f64(&value.return_pct)?,
            max_drawdown_pct: parse_f64(&value.max_drawdown_pct)?,
            positions: from_json(&value.positions_json)?,
            equity_curve: from_json(&value.equity_curve_json)?,
            created_at: value.created_at,
        })
    }
}

#[derive(Queryable, Identifiable, Insertable, AsChangeset, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::arena_results)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ArenaResultDB {
    pub id: String,
    pub challenge_id: String,
    pub participant_id: String,
    pub return_pct: String,
    pub max_drawdown_pct: String,
    pub risk_adjusted_score: String,
    pub final_score: Option<String>,
    pub rank: Option<i32>,
    pub trade_count: i32,
    pub disqualified_reason: Option<String>,
    pub metrics_json: String,
    pub settled_at: String,
}

impl TryFrom<ArenaResult> for ArenaResultDB {
    type Error = Error;

    fn try_from(value: ArenaResult) -> Result<Self> {
        Ok(Self {
            id: value.id,
            challenge_id: value.challenge_id,
            participant_id: value.participant_id,
            return_pct: value.return_pct.to_string(),
            max_drawdown_pct: value.max_drawdown_pct.to_string(),
            risk_adjusted_score: value.risk_adjusted_score.to_string(),
            final_score: value.final_score.map(|score| score.to_string()),
            rank: value.rank,
            trade_count: value.trade_count,
            disqualified_reason: value.disqualified_reason,
            metrics_json: to_json(&value.metrics)?,
            settled_at: value.settled_at,
        })
    }
}

impl TryFrom<ArenaResultDB> for ArenaResult {
    type Error = Error;

    fn try_from(value: ArenaResultDB) -> Result<Self> {
        Ok(Self {
            id: value.id,
            challenge_id: value.challenge_id,
            participant_id: value.participant_id,
            return_pct: parse_f64(&value.return_pct)?,
            max_drawdown_pct: parse_f64(&value.max_drawdown_pct)?,
            risk_adjusted_score: parse_f64(&value.risk_adjusted_score)?,
            final_score: value.final_score.as_deref().map(parse_f64).transpose()?,
            rank: value.rank,
            trade_count: value.trade_count,
            disqualified_reason: value.disqualified_reason,
            metrics: from_json(&value.metrics_json)?,
            settled_at: value.settled_at,
        })
    }
}

#[derive(Queryable, Identifiable, Insertable, AsChangeset, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::company_theses)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct CompanyThesisDB {
    pub id: String,
    pub symbol: String,
    pub agent_id: Option<String>,
    pub challenge_id: Option<String>,
    pub run_id: Option<String>,
    pub rating: Option<String>,
    pub confidence: Option<String>,
    pub horizon: Option<String>,
    pub thesis: String,
    pub risks_json: String,
    pub catalysts_json: String,
    pub created_at: String,
    pub updated_at: String,
}

impl TryFrom<CompanyThesis> for CompanyThesisDB {
    type Error = Error;

    fn try_from(value: CompanyThesis) -> Result<Self> {
        Ok(Self {
            id: value.id,
            symbol: value.symbol,
            agent_id: value.agent_id,
            challenge_id: value.challenge_id,
            run_id: value.run_id,
            rating: value.rating,
            confidence: value.confidence.map(|v| v.to_string()),
            horizon: value.horizon,
            thesis: value.thesis,
            risks_json: to_json(&value.risks)?,
            catalysts_json: to_json(&value.catalysts)?,
            created_at: value.created_at,
            updated_at: value.updated_at,
        })
    }
}

impl TryFrom<CompanyThesisDB> for CompanyThesis {
    type Error = Error;

    fn try_from(value: CompanyThesisDB) -> Result<Self> {
        Ok(Self {
            id: value.id,
            symbol: value.symbol,
            agent_id: value.agent_id,
            challenge_id: value.challenge_id,
            run_id: value.run_id,
            rating: value.rating,
            confidence: value.confidence.map(|v| parse_f64(&v)).transpose()?,
            horizon: value.horizon,
            thesis: value.thesis,
            risks: from_json(&value.risks_json)?,
            catalysts: from_json(&value.catalysts_json)?,
            created_at: value.created_at,
            updated_at: value.updated_at,
        })
    }
}

fn bool_to_i32(value: bool) -> i32 {
    if value {
        1
    } else {
        0
    }
}

fn parse_f64(value: &str) -> Result<f64> {
    value
        .parse::<f64>()
        .map_err(|e| Error::from(StorageError::SerializationError(e.to_string())))
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|e| Error::from(StorageError::SerializationError(e.to_string())))
}

fn from_json<T: serde::de::DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_str(value)
        .map_err(|e| Error::from(StorageError::SerializationError(e.to_string())))
}

#[allow(dead_code)]
fn _value_round_trip(value: Value) -> Value {
    value
}
