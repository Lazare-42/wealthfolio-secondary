use async_trait::async_trait;

use crate::Result;

use super::model::{
    ArenaAgent, ArenaChallenge, ArenaDecisionRequest, ArenaLeaderboard, ArenaParticipant,
    ArenaPortfolio, ArenaResult, ArenaRun, ArenaRunUpdate, ArenaSnapshot, ArenaTrade,
    CompanyThesis, CreateArenaAgentRequest, CreateArenaChallengeRequest,
    CreateCompanyThesisRequest, RunArenaAgentRequest,
};

#[async_trait]
pub trait AiArenaDecisionRunner: Send + Sync {
    async fn run_decision(&self, request: ArenaDecisionRequest) -> Result<String>;
}

#[async_trait]
pub trait AiArenaRepositoryTrait: Send + Sync {
    async fn create_agent(&self, agent: ArenaAgent) -> Result<ArenaAgent>;
    async fn update_agent(&self, agent: ArenaAgent) -> Result<ArenaAgent>;
    async fn delete_agent(&self, id: &str) -> Result<usize>;
    fn get_agent(&self, id: &str) -> Result<ArenaAgent>;
    fn list_agents(&self) -> Result<Vec<ArenaAgent>>;

    async fn create_challenge(&self, challenge: ArenaChallenge) -> Result<ArenaChallenge>;
    async fn update_challenge(&self, challenge: ArenaChallenge) -> Result<ArenaChallenge>;
    fn get_challenge(&self, id: &str) -> Result<ArenaChallenge>;
    fn list_challenges(&self) -> Result<Vec<ArenaChallenge>>;

    async fn create_participant(&self, participant: ArenaParticipant) -> Result<ArenaParticipant>;
    fn get_participant(
        &self,
        challenge_id: &str,
        agent_id: &str,
    ) -> Result<Option<ArenaParticipant>>;
    fn get_participant_by_id(&self, id: &str) -> Result<ArenaParticipant>;
    fn list_participants(&self, challenge_id: &str) -> Result<Vec<ArenaParticipant>>;

    async fn create_run(&self, run: ArenaRun) -> Result<ArenaRun>;
    async fn update_run(&self, update: ArenaRunUpdate) -> Result<ArenaRun>;
    fn get_run_by_idempotency_key(&self, key: &str) -> Result<Option<ArenaRun>>;
    fn list_runs(&self, challenge_id: &str) -> Result<Vec<ArenaRun>>;

    async fn create_trade(&self, trade: ArenaTrade) -> Result<ArenaTrade>;
    fn list_trades_for_participant(&self, participant_id: &str) -> Result<Vec<ArenaTrade>>;
    fn list_trades_for_challenge(&self, challenge_id: &str) -> Result<Vec<ArenaTrade>>;

    async fn upsert_snapshot(&self, snapshot: ArenaSnapshot) -> Result<ArenaSnapshot>;
    fn list_snapshots_for_participant(&self, participant_id: &str) -> Result<Vec<ArenaSnapshot>>;

    async fn replace_results(
        &self,
        challenge_id: &str,
        results: Vec<ArenaResult>,
    ) -> Result<Vec<ArenaResult>>;
    fn list_results(&self, challenge_id: &str) -> Result<Vec<ArenaResult>>;

    async fn create_thesis(&self, thesis: CompanyThesis) -> Result<CompanyThesis>;
    fn list_theses(
        &self,
        symbol: Option<&str>,
        challenge_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<CompanyThesis>>;
}

#[async_trait]
pub trait AiArenaServiceTrait: Send + Sync {
    async fn create_agent(&self, input: CreateArenaAgentRequest) -> Result<ArenaAgent>;
    async fn update_agent(&self, id: &str, input: CreateArenaAgentRequest) -> Result<ArenaAgent>;
    async fn delete_agent(&self, id: &str) -> Result<()>;
    fn get_agent(&self, id: &str) -> Result<ArenaAgent>;
    fn list_agents(&self) -> Result<Vec<ArenaAgent>>;

    async fn create_challenge(&self, input: CreateArenaChallengeRequest) -> Result<ArenaChallenge>;
    fn get_challenge(&self, id: &str) -> Result<ArenaChallenge>;
    fn list_challenges(&self) -> Result<Vec<ArenaChallenge>>;
    async fn join_challenge(&self, challenge_id: &str, agent_id: &str) -> Result<ArenaParticipant>;
    fn list_participants(&self, challenge_id: &str) -> Result<Vec<ArenaParticipant>>;

    async fn run_agent(&self, request: RunArenaAgentRequest) -> Result<ArenaRun>;
    async fn run_due_scheduled(&self) -> Result<Vec<ArenaRun>>;
    async fn settle_challenge(&self, challenge_id: &str) -> Result<ArenaLeaderboard>;

    async fn get_leaderboard(&self, challenge_id: &str) -> Result<ArenaLeaderboard>;
    async fn get_portfolio(&self, participant_id: &str) -> Result<ArenaPortfolio>;
    fn list_runs(&self, challenge_id: &str) -> Result<Vec<ArenaRun>>;
    fn list_trades(&self, challenge_id: &str) -> Result<Vec<ArenaTrade>>;

    async fn create_thesis(&self, input: CreateCompanyThesisRequest) -> Result<CompanyThesis>;
    fn list_theses(
        &self,
        symbol: Option<&str>,
        challenge_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<CompanyThesis>>;
}
