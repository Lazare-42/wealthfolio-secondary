-- Create import_sessions table to track activity import batches
CREATE TABLE IF NOT EXISTS import_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL,
    file_name TEXT,
    imported_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    activity_count INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    failed_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

-- Add import_session_id column to activities table
ALTER TABLE activities ADD COLUMN import_session_id TEXT REFERENCES import_sessions(id) ON DELETE SET NULL;

-- Create index for efficient querying by account
CREATE INDEX IF NOT EXISTS idx_import_sessions_account_id ON import_sessions(account_id);

-- Create index for querying activities by import session
CREATE INDEX IF NOT EXISTS idx_activities_import_session_id ON activities(import_session_id);
