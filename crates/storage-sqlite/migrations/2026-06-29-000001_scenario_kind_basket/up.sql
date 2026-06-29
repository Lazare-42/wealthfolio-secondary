ALTER TABLE portfolio_scenarios ADD COLUMN kind TEXT NOT NULL DEFAULT 'comparison';
ALTER TABLE portfolio_scenarios ADD COLUMN basket_json TEXT NOT NULL DEFAULT '[]';
