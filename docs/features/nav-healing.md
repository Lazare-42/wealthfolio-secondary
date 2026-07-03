# NAV healing — auto-refresh manually-priced holdings

## Problem

Illiquid holdings (private-equity feeders, structured products / autocallables,
funds with no provider quote) are `quote_mode = MANUAL`. No market provider can
refresh them, so their last quote goes stale (months → years), which triggers
Data Health issues:

- **Price updates needed** (price staleness; warn 24h / critical 72h).
- **N generated valuation rows are missing** (no quote on a snapshot date).

Their NAVs _do_ arrive — in bank-statement emails and PDFs (HSBC holdings
exports, Neuflize _relevés titres_, etc.). This feature turns those into quotes
automatically.

## Pipeline

```
bank statement (email / PDF / CSV)
        │  producer (tools/nav-envelope.py, or any extractor)
        ▼
{data_root}/nav-inbox/nav-YYYY-MM-DD-*.json     ← NAV envelope (version 1)
        │  nav_healing watcher (polls every 30s)
        ▼
quote_service.update_quote(MANUAL)  per holding   ← idempotent per (asset, day)
        │
        ▼
portfolio recalc (snapshots + valuations, no market sync)
        │
        ▼
Data Health "price updates needed" / "missing valuation" clear
```

Mirrors the existing `email_order_import` / `pdf_import` watchers: processed
files move to `nav-processed/`, failures to `nav-failed/`.

## Envelope format (version 1)

```json
{
  "version": 1,
  "source": "20260625_All holdings_63319.csv",
  "asOf": "2026-06-25",
  "prices": [
    {
      "isin": "LU1645746105",
      "nav": 158.15,
      "currency": "EUR",
      "name": "AGIF EU EQ GR"
    },
    { "assetId": "4f2a5076-...", "nav": 98.1, "asOf": "2026-06-27" }
  ]
}
```

Each price resolves to an asset by `assetId`, then `isin`/`symbol` (matched
against `assets.instrument_symbol`), then exact `name` — case-insensitive.
`asOf` is per-envelope; a price may override it. `currency` defaults to the
asset's `quote_ccy`. Unmatched / non-positive / bad-date prices are skipped with
a warning; a file with zero applied NAVs goes to `nav-failed/`.

Quotes are written with `data_source = MANUAL`; `update_quote` regenerates the
deterministic id `{asset_id}-{YYYY-MM-DD}` and upserts that row, so re-dropping
the same statement is a no-op. Provider rows for the same day are left
untouched.

## Producer

`tools/nav-envelope.py` builds an envelope from a holdings export:

```bash
# HSBC "All holdings" export (UTF-16, tab-delimited)
python3 tools/nav-envelope.py hsbc "All holdings_63319.csv" \
    --out /data/lazrossi/wealthfolio-data/nav-inbox/

# Any delimited file
python3 tools/nav-envelope.py csv statement.csv \
    --isin-col ISIN --nav-col Price --ccy-col Cur --name-col Name \
    --out -
```

As-of date is taken from a `YYYYMMDD` token in the filename, or `--as-of`.

For PDF statements (Neuflize _relevés titres_), extract `[{isin, nav, asOf}]`
with the existing LLM PDF path (`crates/ai/pdf_parser`) or any parser and emit
the same envelope — the watcher is format-agnostic.

## What this does / doesn't fix

- ✅ MANUAL price staleness + missing valuation rows for any holding whose NAV
  appears in a statement.
- ❌ Not for `MARKET`-mode assets — those self-heal on the 6h provider sync.
- ❌ Not the transfer-integrity issues (unpaired `TRANSFER_IN/OUT`, cost basis);
  those are an activity-modeling problem, separate from pricing.

## Files

- `apps/server/src/nav_healing.rs` — watcher + envelope parsing + asset
  resolution.
- `apps/server/src/main.rs`, `lib.rs` — module wiring + watcher startup.
- `tools/nav-envelope.py` — statement → envelope producer.
