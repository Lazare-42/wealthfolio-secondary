import type {
  ReconciliationConfig,
  ReconciliationResult,
  ResolveRequest,
  ScanResult,
  ImportRun,
} from "@/lib/types";
import { invoke } from "./platform";

export const reconciliationScan = async (): Promise<ScanResult> => {
  return invoke<ScanResult>("reconciliation_scan");
};

export const getReconciliationPending = async (): Promise<ReconciliationResult[]> => {
  return invoke<ReconciliationResult[]>("reconciliation_pending");
};

export const getReconciliationDetail = async (runId: string): Promise<ReconciliationResult> => {
  return invoke<ReconciliationResult>("reconciliation_detail", { runId });
};

export const reconciliationResolve = async (request: ResolveRequest): Promise<ImportRun> => {
  return invoke<ImportRun>("reconciliation_resolve", { request });
};

export const getReconciliationConfig = async (): Promise<ReconciliationConfig> => {
  return invoke<ReconciliationConfig>("get_reconciliation_config");
};

export const updateReconciliationConfig = async (
  config: ReconciliationConfig,
): Promise<ReconciliationConfig> => {
  return invoke<ReconciliationConfig>("update_reconciliation_config", { config });
};
