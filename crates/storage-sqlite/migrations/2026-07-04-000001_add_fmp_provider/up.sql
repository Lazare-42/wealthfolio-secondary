-- Add Financial Modeling Prep (FMP) market data provider.
-- Disabled by default: requires an API key (free tier ~250 requests/day).
INSERT OR IGNORE INTO market_data_providers (id, name, description, url, priority, enabled, logo_filename, last_synced_at, last_sync_status, last_sync_error)
VALUES
    ('FMP', 'Financial Modeling Prep', 'Financial Modeling Prep provides stock quotes, historical prices, company profiles, dividends, and a stock screener. Free tier includes ~250 API calls/day.', 'https://financialmodelingprep.com/', 5, FALSE, 'fmp.png', NULL, NULL, NULL);
