use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub const DEFAULT_ARENA_INITIAL_CASH: f64 = 100_000.0;
pub const DEFAULT_ARENA_MAX_POSITION_PCT: f64 = 50.0;
pub const DEFAULT_ARENA_MAX_DRAWDOWN_PCT: f64 = 25.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArenaChallengeStatus {
    Draft,
    Active,
    Settled,
    Cancelled,
}

impl ArenaChallengeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Settled => "settled",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "draft" => Self::Draft,
            "settled" => Self::Settled,
            "cancelled" => Self::Cancelled,
            _ => Self::Active,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArenaScoringMethod {
    ReturnOnly,
    RiskAdjusted,
}

impl ArenaScoringMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReturnOnly => "return_only",
            Self::RiskAdjusted => "risk_adjusted",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "return_only" => Self::ReturnOnly,
            _ => Self::RiskAdjusted,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArenaRunType {
    Manual,
    Scheduled,
    Thesis,
}

impl ArenaRunType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Scheduled => "scheduled",
            Self::Thesis => "thesis",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "scheduled" => Self::Scheduled,
            "thesis" => Self::Thesis,
            _ => Self::Manual,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArenaRunStatus {
    Running,
    Completed,
    CompletedWithRejections,
    Failed,
}

impl ArenaRunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::CompletedWithRejections => "completed_with_rejections",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "completed" => Self::Completed,
            "completed_with_rejections" => Self::CompletedWithRejections,
            "failed" => Self::Failed,
            _ => Self::Running,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArenaTradeSide {
    Buy,
    Sell,
}

impl ArenaTradeSide {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "buy" => Some(Self::Buy),
            "sell" => Some(Self::Sell),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArenaTradeStatus {
    Executed,
    Rejected,
}

impl ArenaTradeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Executed => "executed",
            Self::Rejected => "rejected",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "executed" => Self::Executed,
            _ => Self::Rejected,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaAgent {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub model_id: String,
    pub persona: String,
    pub enabled: bool,
    pub schedule_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateArenaAgentRequest {
    pub name: String,
    pub provider_id: String,
    pub model_id: String,
    #[serde(default)]
    pub persona: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub schedule_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaChallenge {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: ArenaChallengeStatus,
    pub market: String,
    pub scoring_method: ArenaScoringMethod,
    pub initial_cash: f64,
    pub max_position_pct: f64,
    pub max_drawdown_pct: f64,
    pub run_cadence: String,
    pub scheduled_time_local: Option<String>,
    pub universe: Vec<String>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub settled_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateArenaChallengeRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_market")]
    pub market: String,
    #[serde(default = "default_scoring_method")]
    pub scoring_method: ArenaScoringMethod,
    #[serde(default = "default_initial_cash")]
    pub initial_cash: f64,
    #[serde(default = "default_max_position_pct")]
    pub max_position_pct: f64,
    #[serde(default = "default_max_drawdown_pct")]
    pub max_drawdown_pct: f64,
    #[serde(default = "default_run_cadence")]
    pub run_cadence: String,
    #[serde(default)]
    pub scheduled_time_local: Option<String>,
    #[serde(default)]
    pub universe: Vec<String>,
    #[serde(default)]
    pub start_at: Option<String>,
    #[serde(default)]
    pub end_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaParticipant {
    pub id: String,
    pub challenge_id: String,
    pub agent_id: String,
    pub status: String,
    pub joined_at: String,
    pub starting_cash: f64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaRun {
    pub id: String,
    pub challenge_id: String,
    pub agent_id: String,
    pub participant_id: String,
    pub run_type: ArenaRunType,
    pub status: ArenaRunStatus,
    pub idempotency_key: Option<String>,
    pub prompt: String,
    pub raw_response: Option<String>,
    pub parsed_json: Option<Value>,
    pub error: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaRunUpdate {
    pub id: String,
    pub status: ArenaRunStatus,
    pub raw_response: Option<String>,
    pub parsed_json: Option<Value>,
    pub error: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaTrade {
    pub id: String,
    pub challenge_id: String,
    pub participant_id: String,
    pub run_id: Option<String>,
    pub symbol: String,
    pub side: ArenaTradeSide,
    pub quantity: f64,
    pub price: f64,
    pub notional: f64,
    pub status: ArenaTradeStatus,
    pub rationale: Option<String>,
    pub rejection_reason: Option<String>,
    pub executed_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaPosition {
    pub symbol: String,
    pub quantity: f64,
    pub avg_entry_price: f64,
    pub current_price: f64,
    pub market_value: f64,
    pub unrealized_pnl_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaEquityPoint {
    pub date: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaPortfolio {
    pub participant: ArenaParticipant,
    pub agent: ArenaAgent,
    pub challenge: ArenaChallenge,
    pub cash: f64,
    pub total_value: f64,
    pub return_pct: f64,
    pub max_drawdown_pct: f64,
    pub trade_count: usize,
    pub positions: Vec<ArenaPosition>,
    pub equity_curve: Vec<ArenaEquityPoint>,
    pub trades: Vec<ArenaTrade>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaSnapshot {
    pub id: String,
    pub challenge_id: String,
    pub participant_id: String,
    pub snapshot_date: String,
    pub total_value: f64,
    pub cash: f64,
    pub return_pct: f64,
    pub max_drawdown_pct: f64,
    pub positions: Vec<ArenaPosition>,
    pub equity_curve: Vec<ArenaEquityPoint>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaLeaderboardEntry {
    pub rank: Option<i32>,
    pub participant_id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub total_value: f64,
    pub cash: f64,
    pub return_pct: f64,
    pub max_drawdown_pct: f64,
    pub risk_adjusted_score: f64,
    pub final_score: f64,
    pub trade_count: usize,
    pub disqualified_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaLeaderboard {
    pub challenge: ArenaChallenge,
    pub entries: Vec<ArenaLeaderboardEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaResult {
    pub id: String,
    pub challenge_id: String,
    pub participant_id: String,
    pub return_pct: f64,
    pub max_drawdown_pct: f64,
    pub risk_adjusted_score: f64,
    pub final_score: f64,
    pub rank: Option<i32>,
    pub trade_count: i32,
    pub disqualified_reason: Option<String>,
    pub metrics: Value,
    pub settled_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyThesis {
    pub id: String,
    pub symbol: String,
    pub agent_id: Option<String>,
    pub challenge_id: Option<String>,
    pub run_id: Option<String>,
    pub rating: Option<String>,
    pub confidence: Option<f64>,
    pub horizon: Option<String>,
    pub thesis: String,
    pub risks: Vec<String>,
    pub catalysts: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCompanyThesisRequest {
    pub symbol: String,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub challenge_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub rating: Option<String>,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub horizon: Option<String>,
    pub thesis: String,
    #[serde(default)]
    pub risks: Vec<String>,
    #[serde(default)]
    pub catalysts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaOrderDecision {
    pub symbol: String,
    pub side: String,
    #[serde(default)]
    pub notional: Option<f64>,
    #[serde(default)]
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaThesisDecision {
    pub symbol: String,
    #[serde(default)]
    pub rating: Option<String>,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub horizon: Option<String>,
    pub thesis: String,
    #[serde(default)]
    pub risks: Vec<String>,
    #[serde(default)]
    pub catalysts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArenaAgentDecision {
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub orders: Vec<ArenaOrderDecision>,
    #[serde(default)]
    pub theses: Vec<ArenaThesisDecision>,
    #[serde(default)]
    pub memory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaDecisionRequest {
    pub provider_id: String,
    pub model_id: String,
    pub system_prompt: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunArenaAgentRequest {
    pub challenge_id: String,
    pub agent_id: String,
    #[serde(default)]
    pub run_type: Option<ArenaRunType>,
}

impl ArenaAgent {
    pub fn new(input: CreateArenaAgentRequest) -> Self {
        let now = now_string();
        Self {
            id: Uuid::new_v4().to_string(),
            name: input.name.trim().to_string(),
            provider_id: input.provider_id.trim().to_string(),
            model_id: input.model_id.trim().to_string(),
            persona: input.persona.unwrap_or_else(default_agent_persona),
            enabled: input.enabled,
            schedule_enabled: input.schedule_enabled,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

impl ArenaChallenge {
    pub fn new(input: CreateArenaChallengeRequest) -> Self {
        let now = now_string();
        Self {
            id: Uuid::new_v4().to_string(),
            name: input.name.trim().to_string(),
            description: normalize_optional(input.description),
            status: ArenaChallengeStatus::Active,
            market: input.market.trim().to_string(),
            scoring_method: input.scoring_method,
            initial_cash: input.initial_cash,
            max_position_pct: input.max_position_pct,
            max_drawdown_pct: input.max_drawdown_pct,
            run_cadence: input.run_cadence.trim().to_string(),
            scheduled_time_local: normalize_optional(input.scheduled_time_local),
            universe: normalize_symbols(input.universe),
            start_at: normalize_optional(input.start_at),
            end_at: normalize_optional(input.end_at),
            settled_at: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

impl ArenaParticipant {
    pub fn new(challenge: &ArenaChallenge, agent_id: &str) -> Self {
        let now = now_string();
        Self {
            id: Uuid::new_v4().to_string(),
            challenge_id: challenge.id.clone(),
            agent_id: agent_id.to_string(),
            status: "active".to_string(),
            joined_at: now.clone(),
            starting_cash: challenge.initial_cash,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

impl ArenaRun {
    pub fn new(
        challenge_id: String,
        agent_id: String,
        participant_id: String,
        run_type: ArenaRunType,
        idempotency_key: Option<String>,
        prompt: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            challenge_id,
            agent_id,
            participant_id,
            run_type,
            status: ArenaRunStatus::Running,
            idempotency_key,
            prompt,
            raw_response: None,
            parsed_json: None,
            error: None,
            started_at: now_string(),
            completed_at: None,
        }
    }
}

impl ArenaTrade {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        challenge_id: String,
        participant_id: String,
        run_id: Option<String>,
        symbol: String,
        side: ArenaTradeSide,
        quantity: f64,
        price: f64,
        notional: f64,
        status: ArenaTradeStatus,
        rationale: Option<String>,
        rejection_reason: Option<String>,
    ) -> Self {
        let now = now_string();
        Self {
            id: Uuid::new_v4().to_string(),
            challenge_id,
            participant_id,
            run_id,
            symbol,
            side,
            quantity,
            price,
            notional,
            status,
            rationale: normalize_optional(rationale),
            rejection_reason,
            executed_at: now.clone(),
            created_at: now,
        }
    }
}

impl ArenaSnapshot {
    pub fn from_portfolio(portfolio: &ArenaPortfolio, snapshot_date: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            challenge_id: portfolio.challenge.id.clone(),
            participant_id: portfolio.participant.id.clone(),
            snapshot_date,
            total_value: portfolio.total_value,
            cash: portfolio.cash,
            return_pct: portfolio.return_pct,
            max_drawdown_pct: portfolio.max_drawdown_pct,
            positions: portfolio.positions.clone(),
            equity_curve: portfolio.equity_curve.clone(),
            created_at: now_string(),
        }
    }
}

impl CompanyThesis {
    pub fn new(input: CreateCompanyThesisRequest) -> Self {
        let now = now_string();
        Self {
            id: Uuid::new_v4().to_string(),
            symbol: normalize_symbol(&input.symbol),
            agent_id: input.agent_id,
            challenge_id: input.challenge_id,
            run_id: input.run_id,
            rating: normalize_optional(input.rating),
            confidence: input.confidence,
            horizon: normalize_optional(input.horizon),
            thesis: input.thesis.trim().to_string(),
            risks: normalize_text_list(input.risks),
            catalysts: normalize_text_list(input.catalysts),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

pub fn now_string() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub fn normalize_symbol(symbol: &str) -> String {
    symbol.trim().to_ascii_uppercase()
}

pub fn normalize_symbols(symbols: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut normalized = Vec::new();
    for symbol in symbols {
        let symbol = normalize_symbol(&symbol);
        if !symbol.is_empty() && seen.insert(symbol.clone()) {
            normalized.push(symbol);
        }
    }
    normalized
}

pub fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn normalize_text_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

fn default_true() -> bool {
    true
}

fn default_market() -> String {
    "us-stock".to_string()
}

fn default_scoring_method() -> ArenaScoringMethod {
    ArenaScoringMethod::RiskAdjusted
}

fn default_initial_cash() -> f64 {
    DEFAULT_ARENA_INITIAL_CASH
}

fn default_max_position_pct() -> f64 {
    DEFAULT_ARENA_MAX_POSITION_PCT
}

fn default_max_drawdown_pct() -> f64 {
    DEFAULT_ARENA_MAX_DRAWDOWN_PCT
}

fn default_run_cadence() -> String {
    "manual".to_string()
}

pub fn default_agent_persona() -> String {
    "You are a long-only US stock and ETF paper-trading agent. Prefer clear theses, risk controls, and concise rationales. You may hold cash.".to_string()
}
