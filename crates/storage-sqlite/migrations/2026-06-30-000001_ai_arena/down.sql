DROP INDEX IF EXISTS idx_company_theses_symbol_created;
DROP INDEX IF EXISTS idx_arena_results_challenge_rank;
DROP INDEX IF EXISTS idx_arena_snapshots_participant_date;
DROP INDEX IF EXISTS idx_arena_trades_participant_executed;
DROP INDEX IF EXISTS idx_arena_runs_challenge_started;
DROP INDEX IF EXISTS idx_arena_participants_challenge;
DROP INDEX IF EXISTS idx_arena_challenges_status;
DROP INDEX IF EXISTS idx_arena_agents_updated_at;

DROP TABLE IF EXISTS company_theses;
DROP TABLE IF EXISTS arena_results;
DROP TABLE IF EXISTS arena_snapshots;
DROP TABLE IF EXISTS arena_trades;
DROP TABLE IF EXISTS arena_runs;
DROP TABLE IF EXISTS arena_participants;
DROP TABLE IF EXISTS arena_challenges;
DROP TABLE IF EXISTS arena_agents;
