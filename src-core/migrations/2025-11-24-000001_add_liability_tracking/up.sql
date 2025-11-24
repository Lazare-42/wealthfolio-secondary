-- Add fields to holdings_snapshots table for liability tracking
ALTER TABLE holdings_snapshots ADD COLUMN total_assets TEXT NOT NULL DEFAULT '0';
ALTER TABLE holdings_snapshots ADD COLUMN total_liabilities TEXT NOT NULL DEFAULT '0';
