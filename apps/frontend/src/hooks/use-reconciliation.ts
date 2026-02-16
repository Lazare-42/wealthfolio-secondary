import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { QueryKeys } from "@/lib/query-keys";
import type { ReconciliationConfig, ResolveRequest } from "@/lib/types";
import {
  reconciliationScan,
  getReconciliationPending,
  getReconciliationDetail,
  reconciliationResolve,
  getReconciliationConfig,
  updateReconciliationConfig,
} from "@/adapters";

export function useReconciliationPending() {
  return useQuery({
    queryKey: [QueryKeys.RECONCILIATION_PENDING],
    queryFn: getReconciliationPending,
    staleTime: 30 * 1000,
  });
}

export function useReconciliationDetail(runId: string) {
  return useQuery({
    queryKey: QueryKeys.reconciliationDetail(runId),
    queryFn: () => getReconciliationDetail(runId),
    enabled: !!runId,
  });
}

export function useReconciliationConfig() {
  return useQuery({
    queryKey: [QueryKeys.RECONCILIATION_CONFIG],
    queryFn: getReconciliationConfig,
  });
}

export function useReconciliationScan() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: reconciliationScan,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.RECONCILIATION_PENDING] });
    },
  });
}

export function useReconciliationResolve() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: ResolveRequest) => reconciliationResolve(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.RECONCILIATION_PENDING] });
      queryClient.invalidateQueries({ queryKey: [QueryKeys.ACTIVITIES] });
    },
  });
}

export function useUpdateReconciliationConfig() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (config: ReconciliationConfig) => updateReconciliationConfig(config),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.RECONCILIATION_CONFIG] });
    },
  });
}
