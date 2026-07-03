-- Idempotent: this migration's version changed (was 2026-06-28-000001, which
-- collided with an upstream same-version migration); deployments that already
-- created the table under the old version must not fail re-applying it.
CREATE TABLE IF NOT EXISTS portfolio_scenarios (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    account_scope_json TEXT NOT NULL,
    resolved_account_ids_json TEXT NOT NULL,
    as_of_date TEXT,
    benchmark_symbols_json TEXT NOT NULL,
    assumptions_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_portfolio_scenarios_updated_at
    ON portfolio_scenarios(updated_at DESC);
