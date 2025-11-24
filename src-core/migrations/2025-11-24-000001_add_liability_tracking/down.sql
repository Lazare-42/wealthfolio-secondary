-- Remove liability tracking fields from holdings_snapshots table
ALTER TABLE holdings_snapshots DROP COLUMN total_assets;
ALTER TABLE holdings_snapshots DROP COLUMN total_liabilities;
