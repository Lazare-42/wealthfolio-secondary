-- Drop index on activities
DROP INDEX IF EXISTS idx_activities_import_session_id;

-- Drop index on import_sessions
DROP INDEX IF EXISTS idx_import_sessions_account_id;

-- SQLite doesn't support DROP COLUMN directly, so we need to recreate the table
-- For now, we'll leave the column (it's nullable and won't affect anything)
-- In production, you'd need a more complex migration

-- Drop import_sessions table
DROP TABLE IF EXISTS import_sessions;
