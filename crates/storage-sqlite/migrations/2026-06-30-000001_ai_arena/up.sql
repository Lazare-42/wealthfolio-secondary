CREATE TABLE arena_agents (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    persona TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    schedule_enabled INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE arena_challenges (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL,
    market TEXT NOT NULL,
    scoring_method TEXT NOT NULL,
    initial_cash TEXT NOT NULL,
    max_position_pct TEXT NOT NULL,
    max_drawdown_pct TEXT NOT NULL,
    run_cadence TEXT NOT NULL,
    scheduled_time_local TEXT,
    universe_json TEXT NOT NULL,
    start_at TEXT,
    end_at TEXT,
    settled_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE arena_participants (
    id TEXT PRIMARY KEY NOT NULL,
    challenge_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    status TEXT NOT NULL,
    joined_at TEXT NOT NULL,
    starting_cash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(challenge_id) REFERENCES arena_challenges(id) ON DELETE CASCADE,
    FOREIGN KEY(agent_id) REFERENCES arena_agents(id) ON DELETE CASCADE,
    UNIQUE(challenge_id, agent_id)
);

CREATE TABLE arena_runs (
    id TEXT PRIMARY KEY NOT NULL,
    challenge_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    participant_id TEXT NOT NULL,
    run_type TEXT NOT NULL,
    status TEXT NOT NULL,
    idempotency_key TEXT,
    prompt TEXT NOT NULL,
    raw_response TEXT,
    parsed_json TEXT,
    error TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    FOREIGN KEY(challenge_id) REFERENCES arena_challenges(id) ON DELETE CASCADE,
    FOREIGN KEY(agent_id) REFERENCES arena_agents(id) ON DELETE CASCADE,
    FOREIGN KEY(participant_id) REFERENCES arena_participants(id) ON DELETE CASCADE,
    UNIQUE(idempotency_key)
);

CREATE TABLE arena_trades (
    id TEXT PRIMARY KEY NOT NULL,
    challenge_id TEXT NOT NULL,
    participant_id TEXT NOT NULL,
    run_id TEXT,
    symbol TEXT NOT NULL,
    side TEXT NOT NULL,
    quantity TEXT NOT NULL,
    price TEXT NOT NULL,
    notional TEXT NOT NULL,
    status TEXT NOT NULL,
    rationale TEXT,
    rejection_reason TEXT,
    executed_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(challenge_id) REFERENCES arena_challenges(id) ON DELETE CASCADE,
    FOREIGN KEY(participant_id) REFERENCES arena_participants(id) ON DELETE CASCADE,
    FOREIGN KEY(run_id) REFERENCES arena_runs(id) ON DELETE SET NULL
);

CREATE TABLE arena_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    challenge_id TEXT NOT NULL,
    participant_id TEXT NOT NULL,
    snapshot_date TEXT NOT NULL,
    total_value TEXT NOT NULL,
    cash TEXT NOT NULL,
    return_pct TEXT NOT NULL,
    max_drawdown_pct TEXT NOT NULL,
    positions_json TEXT NOT NULL,
    equity_curve_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(challenge_id) REFERENCES arena_challenges(id) ON DELETE CASCADE,
    FOREIGN KEY(participant_id) REFERENCES arena_participants(id) ON DELETE CASCADE,
    UNIQUE(participant_id, snapshot_date)
);

CREATE TABLE arena_results (
    id TEXT PRIMARY KEY NOT NULL,
    challenge_id TEXT NOT NULL,
    participant_id TEXT NOT NULL,
    return_pct TEXT NOT NULL,
    max_drawdown_pct TEXT NOT NULL,
    risk_adjusted_score TEXT NOT NULL,
    final_score TEXT NOT NULL,
    rank INTEGER,
    trade_count INTEGER NOT NULL,
    disqualified_reason TEXT,
    metrics_json TEXT NOT NULL,
    settled_at TEXT NOT NULL,
    FOREIGN KEY(challenge_id) REFERENCES arena_challenges(id) ON DELETE CASCADE,
    FOREIGN KEY(participant_id) REFERENCES arena_participants(id) ON DELETE CASCADE,
    UNIQUE(challenge_id, participant_id)
);

CREATE TABLE company_theses (
    id TEXT PRIMARY KEY NOT NULL,
    symbol TEXT NOT NULL,
    agent_id TEXT,
    challenge_id TEXT,
    run_id TEXT,
    rating TEXT,
    confidence TEXT,
    horizon TEXT,
    thesis TEXT NOT NULL,
    risks_json TEXT NOT NULL,
    catalysts_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(agent_id) REFERENCES arena_agents(id) ON DELETE SET NULL,
    FOREIGN KEY(challenge_id) REFERENCES arena_challenges(id) ON DELETE SET NULL,
    FOREIGN KEY(run_id) REFERENCES arena_runs(id) ON DELETE SET NULL
);

CREATE INDEX idx_arena_agents_updated_at ON arena_agents(updated_at DESC);
CREATE INDEX idx_arena_challenges_status ON arena_challenges(status);
CREATE INDEX idx_arena_participants_challenge ON arena_participants(challenge_id);
CREATE INDEX idx_arena_runs_challenge_started ON arena_runs(challenge_id, started_at DESC);
CREATE INDEX idx_arena_trades_participant_executed ON arena_trades(participant_id, executed_at, id);
CREATE INDEX idx_arena_snapshots_participant_date ON arena_snapshots(participant_id, snapshot_date);
CREATE INDEX idx_arena_results_challenge_rank ON arena_results(challenge_id, rank);
CREATE INDEX idx_company_theses_symbol_created ON company_theses(symbol, created_at DESC);
