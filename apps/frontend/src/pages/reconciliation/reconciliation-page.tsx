import { useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  useReconciliationPending,
  useReconciliationScan,
  useReconciliationConfig,
} from "@/hooks/use-reconciliation";
import type { MatchStatus, ReconciliationResult } from "@/lib/types";
import { Badge, Button, Icons, Page, PageContent, PageHeader, Skeleton } from "@wealthfolio/ui";
import { ReconciliationSettingsSheet } from "./components/reconciliation-settings-sheet";

const STATUS_COLORS: Record<MatchStatus, string> = {
  MATCHED: "bg-success/15 text-success",
  UNMATCHED: "bg-blue-500/15 text-blue-600 dark:text-blue-400",
  CONFLICT: "bg-warning/15 text-warning",
  MISSING: "bg-destructive/15 text-destructive",
};

function ReconciliationCard({ result }: { result: ReconciliationResult }) {
  const navigate = useNavigate();
  const { summary } = result;

  const counts = {
    MATCHED: summary.skipped,
    UNMATCHED: summary.inserted,
    CONFLICT: summary.updated,
    MISSING: summary.removed,
  };

  return (
    <div
      className="bg-card hover:bg-muted/50 cursor-pointer rounded-lg border p-4 transition-colors"
      onClick={() => navigate(`/reconciliation/${result.importRun.id}`)}
    >
      <div className="flex items-center justify-between">
        <div className="min-w-0 flex-1">
          <h3 className="truncate text-sm font-medium">{result.fileName}</h3>
          <p className="text-muted-foreground mt-0.5 text-xs">
            {result.items.length} rows &middot; Account: {result.accountId}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          {(Object.entries(counts) as [MatchStatus, number][]).map(([status, count]) => {
            if (count === 0) return null;
            return (
              <Badge key={status} className={`${STATUS_COLORS[status]} text-[10px] font-medium`}>
                {count} {status.toLowerCase()}
              </Badge>
            );
          })}
        </div>
      </div>
    </div>
  );
}

function SetupEmptyState({ onConfigure }: { onConfigure: () => void }) {
  return (
    <div className="flex flex-col items-center justify-center py-20">
      <div className="bg-muted mb-6 flex h-16 w-16 items-center justify-center rounded-full">
        <Icons.Settings className="text-muted-foreground h-8 w-8" />
      </div>
      <h2 className="mb-2 text-lg font-semibold">Configure Reconciliation</h2>
      <p className="text-muted-foreground mb-6 max-w-sm text-center text-sm">
        Set up your statements directory and account mappings to start reconciling bank statements
        against existing activities.
      </p>
      <Button onClick={onConfigure}>
        <Icons.Settings className="mr-2 h-4 w-4" />
        Configure Settings
      </Button>
    </div>
  );
}

function EmptyState() {
  return (
    <div className="flex flex-col items-center justify-center py-20">
      <div className="bg-muted mb-6 flex h-16 w-16 items-center justify-center rounded-full">
        <Icons.FileText className="text-muted-foreground h-8 w-8" />
      </div>
      <h2 className="mb-2 text-lg font-semibold">No Pending Reconciliations</h2>
      <p className="text-muted-foreground max-w-sm text-center text-sm">
        Scan your statements directory to find new bank statement files and reconcile them against
        existing activities.
      </p>
    </div>
  );
}

export default function ReconciliationPage() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const { data: config } = useReconciliationConfig();
  const { data: pending, isLoading, error } = useReconciliationPending();
  const scanMutation = useReconciliationScan();

  const isConfigured =
    config && config.statementsDir && config.mappings && config.mappings.length > 0;

  const headerActions = (
    <div className="flex items-center gap-2">
      {isConfigured && (
        <Button size="sm" onClick={() => scanMutation.mutate()} disabled={scanMutation.isPending}>
          {scanMutation.isPending ? (
            <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <Icons.Search className="mr-2 h-4 w-4" />
          )}
          Scan Statements
        </Button>
      )}
      <Button size="sm" variant="outline" onClick={() => setSettingsOpen(true)}>
        <Icons.Settings className="mr-2 h-4 w-4" />
        Settings
      </Button>
    </div>
  );

  return (
    <Page>
      <PageHeader
        heading="Reconciliation"
        text="Match bank statements against existing activities"
        actions={headerActions}
      />
      <PageContent className="mt-6">
        {scanMutation.isSuccess && scanMutation.data && (
          <div className="bg-muted mb-4 rounded-lg p-3 text-sm">
            Scan complete: {scanMutation.data.filesFound} files found, {scanMutation.data.filesNew}{" "}
            new, {scanMutation.data.filesSkipped} skipped,{" "}
            {scanMutation.data.reconciliations.length} reconciliations created.
          </div>
        )}
        {error && (
          <div className="flex min-h-[300px] flex-col items-center justify-center">
            <div className="bg-destructive/10 mb-4 flex h-12 w-12 items-center justify-center rounded-full">
              <Icons.AlertCircle className="text-destructive h-6 w-6" />
            </div>
            <h2 className="mb-1 text-base font-medium">Failed to load reconciliations</h2>
            <p className="text-muted-foreground mb-4 text-sm">{error.message}</p>
          </div>
        )}
        {!error && !isConfigured && <SetupEmptyState onConfigure={() => setSettingsOpen(true)} />}
        {!error && isConfigured && (
          <>
            {isLoading ? (
              <div className="space-y-3">
                <Skeleton className="h-16 rounded-lg" />
                <Skeleton className="h-16 rounded-lg" />
                <Skeleton className="h-16 rounded-lg" />
              </div>
            ) : pending && pending.length > 0 ? (
              <div className="space-y-2">
                {pending.map((result) => (
                  <ReconciliationCard key={result.importRun.id} result={result} />
                ))}
              </div>
            ) : (
              <EmptyState />
            )}
          </>
        )}
      </PageContent>
      <ReconciliationSettingsSheet open={settingsOpen} onOpenChange={setSettingsOpen} />
    </Page>
  );
}
