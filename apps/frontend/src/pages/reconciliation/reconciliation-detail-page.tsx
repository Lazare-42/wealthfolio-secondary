import { useReconciliationDetail, useReconciliationResolve } from "@/hooks/use-reconciliation";
import type { MatchStatus, ReconciliationItem } from "@/lib/types";
import { Badge, Button, Icons, Page, PageContent, PageHeader, Skeleton } from "@wealthfolio/ui";
import { cn } from "@wealthfolio/ui/lib/utils";
import { useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

const STATUS_CONFIG: Record<MatchStatus, { label: string; color: string; rowColor: string }> = {
  MATCHED: {
    label: "Matched",
    color: "bg-success/15 text-success",
    rowColor: "",
  },
  UNMATCHED: {
    label: "Unmatched",
    color: "bg-blue-500/15 text-blue-600 dark:text-blue-400",
    rowColor: "bg-blue-500/5",
  },
  CONFLICT: {
    label: "Conflict",
    color: "bg-warning/15 text-warning",
    rowColor: "bg-warning/5",
  },
  MISSING: {
    label: "Missing",
    color: "bg-destructive/15 text-destructive",
    rowColor: "bg-destructive/5",
  },
};

function formatAmount(amount: number, currency: string) {
  return new Intl.NumberFormat(undefined, {
    style: "currency",
    currency,
    minimumFractionDigits: 2,
  }).format(amount);
}

export default function ReconciliationDetailPage() {
  const { runId } = useParams<{ runId: string }>();
  const navigate = useNavigate();
  const { data: result, isLoading, error } = useReconciliationDetail(runId ?? "");
  const resolveMutation = useReconciliationResolve();

  const [selected, setSelected] = useState<Set<string>>(new Set());

  const actionableItems =
    result?.items.filter(
      (item) =>
        item.draftActivityId &&
        (item.matchStatus === "UNMATCHED" || item.matchStatus === "CONFLICT"),
    ) ?? [];

  const toggleSelect = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const selectAll = () => {
    const allIds = actionableItems.map((i) => i.draftActivityId!);
    setSelected(new Set(allIds));
  };

  const selectNone = () => setSelected(new Set());

  const handleAcceptSelected = () => {
    if (!runId) return;
    const acceptIds = Array.from(selected);
    const allActionable = actionableItems.map((i) => i.draftActivityId!);
    const rejectIds = allActionable.filter((id) => !selected.has(id));
    resolveMutation.mutate(
      { importRunId: runId, acceptIds, rejectIds },
      { onSuccess: () => navigate("/reconciliation") },
    );
  };

  const handleRejectAll = () => {
    if (!runId) return;
    const rejectIds = actionableItems.map((i) => i.draftActivityId!);
    resolveMutation.mutate(
      { importRunId: runId, acceptIds: [], rejectIds },
      { onSuccess: () => navigate("/reconciliation") },
    );
  };

  if (error) {
    return (
      <Page>
        <PageHeader heading="Reconciliation Detail" />
        <PageContent className="pt-4">
          <div className="flex min-h-[300px] flex-col items-center justify-center">
            <Icons.AlertCircle className="text-destructive mb-4 h-8 w-8" />
            <p className="text-muted-foreground text-sm">{error.message}</p>
          </div>
        </PageContent>
      </Page>
    );
  }

  const headerActions = result && actionableItems.length > 0 && (
    <div className="flex items-center gap-2">
      <span className="text-muted-foreground text-xs">
        {selected.size}/{actionableItems.length} selected
      </span>
      <Button variant="outline" size="sm" onClick={selectAll}>
        Select All
      </Button>
      <Button variant="outline" size="sm" onClick={selectNone}>
        Clear
      </Button>
      <Button
        variant="destructive"
        size="sm"
        onClick={handleRejectAll}
        disabled={resolveMutation.isPending}
      >
        Reject All
      </Button>
      <Button
        size="sm"
        onClick={handleAcceptSelected}
        disabled={resolveMutation.isPending || selected.size === 0}
      >
        {resolveMutation.isPending ? <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" /> : null}
        Accept Selected ({selected.size})
      </Button>
    </div>
  );

  return (
    <Page>
      <PageHeader
        heading={result?.fileName ?? "Reconciliation Detail"}
        text={
          result ? `${result.items.length} rows \u00b7 Account: ${result.accountId}` : undefined
        }
        actions={headerActions || undefined}
      />
      <PageContent className="mt-6">
        {isLoading ? (
          <div className="space-y-2">
            <Skeleton className="h-10 rounded-lg" />
            <Skeleton className="h-12 rounded-lg" />
            <Skeleton className="h-12 rounded-lg" />
            <Skeleton className="h-12 rounded-lg" />
          </div>
        ) : result ? (
          <div className="bg-card overflow-hidden rounded-lg border">
            <table className="w-full text-sm">
              <thead>
                <tr className="bg-muted/50 border-b text-left">
                  <th className="w-10 px-3 py-2" />
                  <th className="px-3 py-2 font-medium">#</th>
                  <th className="px-3 py-2 font-medium">Date</th>
                  <th className="px-3 py-2 font-medium">Description</th>
                  <th className="px-3 py-2 text-right font-medium">Bank Amount</th>
                  <th className="px-3 py-2 font-medium">Existing Activity</th>
                  <th className="px-3 py-2 font-medium">Status</th>
                </tr>
              </thead>
              <tbody className="divide-y">
                {result.items.map((item) => (
                  <ReconciliationRow
                    key={item.rowIndex}
                    item={item}
                    isSelected={selected.has(item.draftActivityId ?? "")}
                    onToggle={() => item.draftActivityId && toggleSelect(item.draftActivityId)}
                  />
                ))}
              </tbody>
            </table>
          </div>
        ) : null}
      </PageContent>
    </Page>
  );
}

function ReconciliationRow({
  item,
  isSelected,
  onToggle,
}: {
  item: ReconciliationItem;
  isSelected: boolean;
  onToggle: () => void;
}) {
  const config = STATUS_CONFIG[item.matchStatus];
  const isActionable =
    item.draftActivityId && (item.matchStatus === "UNMATCHED" || item.matchStatus === "CONFLICT");

  return (
    <tr className={cn("transition-colors", config.rowColor)}>
      <td className="px-3 py-2">
        {isActionable ? (
          <input
            type="checkbox"
            checked={isSelected}
            onChange={onToggle}
            className="h-4 w-4 rounded"
          />
        ) : null}
      </td>
      <td className="text-muted-foreground px-3 py-2">{item.rowIndex}</td>
      <td className="px-3 py-2">{item.activityDate}</td>
      <td className="max-w-[200px] truncate px-3 py-2">{item.description ?? "-"}</td>
      <td className="px-3 py-2 text-right font-mono">{formatAmount(item.amount, item.currency)}</td>
      <td className="px-3 py-2">
        {item.matchedActivity ? (
          <span className="text-muted-foreground text-xs">
            {item.matchedActivity.activityDate}{" "}
            {item.matchedActivity.amount
              ? formatAmount(Number(item.matchedActivity.amount), item.matchedActivity.currency)
              : "-"}
          </span>
        ) : (
          <span className="text-muted-foreground text-xs">-</span>
        )}
      </td>
      <td className="px-3 py-2">
        <Badge className={`${config.color} text-[10px] font-medium`}>{config.label}</Badge>
      </td>
    </tr>
  );
}
