//! NAV healing pipeline.
//!
//! Illiquid, manually-priced holdings (private-equity feeders, structured
//! products, funds with no provider quote) go stale because no market provider
//! can refresh them. Their NAVs *do* arrive — in bank statement emails and PDFs
//! that the email/PDF watchers already ingest. This watcher consumes a simple
//! per-holding NAV envelope (one JSON file per statement) and writes a `MANUAL`
//! quote per holding, then triggers a portfolio recalculation so valuations and
//! the data-health "price updates needed" / "missing valuation rows" issues
//! resolve automatically.
//!
//! Mirrors `email_order_import`: poll `nav-inbox/`, move to `nav-processed/` or
//! `nav-failed/`. Envelope shape (version 1):
//!
//! ```json
//! {
//!   "version": 1,
//!   "source": "neuflize-statement-2026-06-30.pdf",
//!   "asOf": "2026-06-30",
//!   "prices": [
//!     { "isin": "LU0187079347", "nav": 142.83, "currency": "EUR" },
//!     { "assetId": "4f2a5076-...", "nav": 98.10, "asOf": "2026-06-27" }
//!   ]
//! }
//! ```
//!
//! Each price resolves to an asset by `assetId`, `isin`/`symbol`
//! (matched against `instrument_symbol`), or exact `name`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::{NaiveDate, TimeZone, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use tracing::{error, info, warn};

use wealthfolio_core::portfolio::{snapshot::SnapshotRecalcMode, valuation::ValuationRecalcMode};
use wealthfolio_core::quotes::{MarketSyncMode, Quote, DATA_SOURCE_MANUAL};

use crate::api::shared::{enqueue_portfolio_job, PortfolioJobConfig};
use crate::main_lib::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NavEnvelope {
    version: u32,
    #[serde(default)]
    source: Option<String>,
    /// Statement date applied to every price unless a price overrides it.
    as_of: String,
    prices: Vec<NavPrice>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NavPrice {
    #[serde(default)]
    asset_id: Option<String>,
    #[serde(default)]
    isin: Option<String>,
    #[serde(default)]
    symbol: Option<String>,
    #[serde(default)]
    name: Option<String>,
    nav: Decimal,
    #[serde(default)]
    currency: Option<String>,
    /// Per-line statement date override (YYYY-MM-DD).
    #[serde(default)]
    as_of: Option<String>,
}

/// A flat lookup from any identifier to (asset_id, quote_ccy).
struct AssetIndex(HashMap<String, (String, String)>);

impl AssetIndex {
    fn build(state: &AppState) -> Self {
        let mut map: HashMap<String, (String, String)> = HashMap::new();
        let assets = state.asset_service.get_assets().unwrap_or_default();
        for asset in assets {
            let value = (asset.id.clone(), asset.quote_ccy.clone());
            map.insert(asset.id.to_uppercase(), value.clone());
            if let Some(symbol) = &asset.instrument_symbol {
                map.entry(symbol.trim().to_uppercase())
                    .or_insert_with(|| value.clone());
            }
            if let Some(name) = &asset.name {
                map.entry(name.trim().to_uppercase())
                    .or_insert_with(|| value.clone());
            }
        }
        AssetIndex(map)
    }

    fn resolve(&self, price: &NavPrice) -> Option<(String, String)> {
        for key in [&price.asset_id, &price.isin, &price.symbol, &price.name]
            .into_iter()
            .flatten()
        {
            let key = key.trim();
            if key.is_empty() {
                continue;
            }
            if let Some(found) = self.0.get(&key.to_uppercase()) {
                return Some(found.clone());
            }
        }
        None
    }
}

pub fn start_nav_healing_watcher(state: Arc<AppState>) {
    tokio::spawn(async move {
        let data_root = state.data_root.clone();
        let inbox = PathBuf::from(&data_root).join("nav-inbox");
        let processed = PathBuf::from(&data_root).join("nav-processed");
        let failed = PathBuf::from(&data_root).join("nav-failed");

        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;

            for dir in [&inbox, &processed, &failed] {
                if let Err(err) = tokio::fs::create_dir_all(dir).await {
                    warn!("NAV healing: failed to create directory {:?}: {}", dir, err);
                }
            }

            let mut read_dir = match tokio::fs::read_dir(&inbox).await {
                Ok(rd) => rd,
                Err(_) => continue,
            };

            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let path = entry.path();
                if !is_json(&path) {
                    continue;
                }
                let file_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                info!("NAV healing: processing {}", file_name);

                match process_file(&state, &path).await {
                    Ok(applied) if applied > 0 => {
                        info!("NAV healing: applied {} NAV(s) from {}", applied, file_name);
                        // Recompute snapshots + valuations so the new manual
                        // quotes flow into account history and clear the health
                        // issues. No market sync — these assets have no provider.
                        enqueue_portfolio_job(
                            state.clone(),
                            PortfolioJobConfig {
                                account_ids: None,
                                market_sync_mode: MarketSyncMode::None,
                                snapshot_mode: SnapshotRecalcMode::Full,
                                valuation_mode: ValuationRecalcMode::Full,
                                since_date: None,
                            },
                        );
                        move_file(&path, &processed.join(&file_name)).await;
                    }
                    Ok(_) => {
                        warn!(
                            "NAV healing: no NAVs matched any asset in {}, moving to failed",
                            file_name
                        );
                        move_file(&path, &failed.join(&file_name)).await;
                    }
                    Err(err) => {
                        error!("NAV healing: failed to process {}: {}", file_name, err);
                        move_file(&path, &failed.join(&file_name)).await;
                    }
                }
            }
        }
    });
}

/// Parse one envelope and write a MANUAL quote per resolvable price.
/// Returns the number of NAVs applied. Errors only on unrecoverable problems
/// (unreadable/unparseable file, unsupported version); individual unmatched or
/// bad-date prices are skipped with a warning.
async fn process_file(state: &AppState, path: &Path) -> Result<usize, String> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| format!("read error: {e}"))?;
    let envelope: NavEnvelope =
        serde_json::from_str(&content).map_err(|e| format!("parse error: {e}"))?;

    if envelope.version != 1 {
        return Err(format!("unsupported envelope version {}", envelope.version));
    }
    if envelope.prices.is_empty() {
        return Err("envelope contains no prices".to_string());
    }

    let index = AssetIndex::build(state);
    let source = envelope
        .source
        .clone()
        .unwrap_or_else(|| "nav-inbox".into());
    let mut applied = 0usize;

    for price in &envelope.prices {
        let (asset_id, asset_ccy) = match index.resolve(price) {
            Some(found) => found,
            None => {
                warn!(
                    "NAV healing: no asset matched price (isin={:?}, symbol={:?}, name={:?})",
                    price.isin, price.symbol, price.name
                );
                continue;
            }
        };

        let day_str = price.as_of.as_deref().unwrap_or(envelope.as_of.as_str());
        let day = match NaiveDate::parse_from_str(day_str.trim(), "%Y-%m-%d") {
            Ok(day) => day,
            Err(_) => {
                warn!("NAV healing: invalid date {:?} for {}", day_str, asset_id);
                continue;
            }
        };
        if price.nav <= Decimal::ZERO {
            warn!(
                "NAV healing: non-positive NAV {} for {}",
                price.nav, asset_id
            );
            continue;
        }

        let timestamp =
            Utc.from_utc_datetime(&day.and_hms_opt(0, 0, 0).expect("00:00:00 is always valid"));
        let currency = price
            .currency
            .clone()
            .filter(|c| !c.trim().is_empty())
            .unwrap_or(asset_ccy);

        // `update_quote` regenerates the deterministic MANUAL id ({asset_id}-{day})
        // and deletes any provider row for that day, so re-healing is idempotent.
        let quote = Quote {
            id: String::new(),
            asset_id: asset_id.clone(),
            timestamp,
            open: price.nav,
            high: price.nav,
            low: price.nav,
            close: price.nav,
            adjclose: price.nav,
            volume: Decimal::ZERO,
            currency,
            data_source: DATA_SOURCE_MANUAL.to_string(),
            created_at: Utc::now(),
            notes: Some(format!("NAV healing from {source}")),
        };

        match state.quote_service.update_quote(quote).await {
            Ok(_) => applied += 1,
            Err(err) => warn!(
                "NAV healing: failed to write quote for {}: {}",
                asset_id, err
            ),
        }
    }

    Ok(applied)
}

fn is_json(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
}

async fn move_file(from: &Path, to: &Path) {
    if let Some(parent) = to.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    if let Err(err) = tokio::fs::rename(from, to).await {
        warn!(
            "NAV healing: failed to move {:?} -> {:?}: {}",
            from, to, err
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> AssetIndex {
        // (assetId, quote_ccy) keyed by upper(id|symbol|name).
        let mut map = HashMap::new();
        let value = ("uuid-robeco".to_string(), "EUR".to_string());
        map.insert("UUID-ROBECO".to_string(), value.clone());
        map.insert("LU0187079347".to_string(), value.clone());
        map.insert("ROBECO GLOBAL CONSUMER TRENDS".to_string(), value);
        AssetIndex(map)
    }

    fn price(isin: Option<&str>, asset_id: Option<&str>, name: Option<&str>) -> NavPrice {
        NavPrice {
            asset_id: asset_id.map(Into::into),
            isin: isin.map(Into::into),
            symbol: None,
            name: name.map(Into::into),
            nav: Decimal::ONE,
            currency: None,
            as_of: None,
        }
    }

    #[test]
    fn resolves_by_isin_case_insensitive() {
        let got = index().resolve(&price(Some("lu0187079347"), None, None));
        assert_eq!(got, Some(("uuid-robeco".into(), "EUR".into())));
    }

    #[test]
    fn resolves_by_asset_id_and_name() {
        assert!(index()
            .resolve(&price(None, Some("uuid-robeco"), None))
            .is_some());
        assert!(index()
            .resolve(&price(None, None, Some("Robeco Global Consumer Trends")))
            .is_some());
    }

    #[test]
    fn unmatched_or_empty_returns_none() {
        assert_eq!(
            index().resolve(&price(Some("XX0000000000"), None, None)),
            None
        );
        assert_eq!(index().resolve(&price(Some("  "), None, None)), None);
    }

    #[test]
    fn envelope_parses_with_per_line_overrides() {
        let json = r#"{
            "version": 1,
            "source": "neuflize-2026-06-30.pdf",
            "asOf": "2026-06-30",
            "prices": [
                { "isin": "LU0187079347", "nav": 142.83, "currency": "EUR" },
                { "assetId": "uuid-x", "nav": 98.1, "asOf": "2026-06-27" }
            ]
        }"#;
        let env: NavEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(env.version, 1);
        assert_eq!(env.as_of, "2026-06-30");
        assert_eq!(env.prices.len(), 2);
        assert_eq!(env.prices[0].nav, Decimal::new(14283, 2));
        assert_eq!(env.prices[1].as_of.as_deref(), Some("2026-06-27"));
    }
}
