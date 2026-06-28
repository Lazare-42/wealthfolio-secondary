CREATE TABLE portfolio_scenarios (
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

CREATE INDEX idx_portfolio_scenarios_updated_at
    ON portfolio_scenarios(updated_at DESC);
