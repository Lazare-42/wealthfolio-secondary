INSERT INTO market_data_providers (id, name, description, url, priority, enabled, logo_filename, last_synced_at, last_sync_status, last_sync_error)
VALUES
    ('OPENFIGI', 'OpenFIGI', 'OpenFIGI resolves ISINs and other identifiers to ticker symbols. Search-only provider — does not provide quotes. Free tier works without an API key.', 'https://www.openfigi.com/', 20, TRUE, NULL, NULL, NULL, NULL)
ON CONFLICT(id) DO NOTHING;
