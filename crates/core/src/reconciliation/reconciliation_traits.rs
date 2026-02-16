//! Reconciliation service trait.

use crate::Result;
use async_trait::async_trait;

use super::{ReconciliationConfig, ReconciliationResult, ResolveRequest, ScanResult};
use crate::sync::ImportRun;

#[async_trait]
pub trait ReconciliationServiceTrait: Send + Sync {
    /// Scan the configured statements directory, parse new CSVs, run reconciliation.
    async fn scan_directory(&self) -> Result<ScanResult>;

    /// List pending reconciliations (ImportRuns with source_system="STATEMENT" and status=NeedsReview).
    async fn get_pending_reconciliations(&self) -> Result<Vec<ReconciliationResult>>;

    /// Get a single reconciliation by import_run_id.
    fn get_reconciliation(&self, import_run_id: &str) -> Result<ReconciliationResult>;

    /// Accept/reject proposals: promote Draft→Posted or Draft→Void, update ImportRun.
    async fn resolve(&self, request: ResolveRequest) -> Result<ImportRun>;

    /// Get reconciliation config from app_settings.
    fn get_config(&self) -> Result<ReconciliationConfig>;

    /// Save reconciliation config to app_settings.
    async fn save_config(&self, config: ReconciliationConfig) -> Result<()>;
}
