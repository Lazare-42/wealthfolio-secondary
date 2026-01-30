import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { format } from "date-fns";
import { Trash2, FileText, AlertTriangle, CheckCircle2, Loader2, XCircle } from "lucide-react";
import { Button } from "@wealthfolio/ui/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@wealthfolio/ui/components/ui/card";
import { Badge } from "@wealthfolio/ui/components/ui/badge";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@wealthfolio/ui/components/ui/alert-dialog";
import { toast } from "@wealthfolio/ui/components/ui/use-toast";
import { QueryKeys } from "@/lib/query-keys";
import { getImportRuns, deleteImportRun } from "@/features/wealthfolio-connect/services/broker-service";
import type { ImportRun, ImportRunStatus } from "@/features/wealthfolio-connect/types";

const statusConfig: Record<
  ImportRunStatus,
  { label: string; variant: "default" | "secondary" | "destructive" | "outline"; icon: typeof CheckCircle2 }
> = {
  RUNNING: { label: "Running", variant: "outline", icon: Loader2 },
  APPLIED: { label: "Applied", variant: "default", icon: CheckCircle2 },
  NEEDS_REVIEW: { label: "Needs Review", variant: "destructive", icon: AlertTriangle },
  FAILED: { label: "Failed", variant: "destructive", icon: XCircle },
  CANCELLED: { label: "Cancelled", variant: "secondary", icon: XCircle },
};

export default function ImportHistoryPage() {
  const queryClient = useQueryClient();
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const { data: importRuns = [], isLoading } = useQuery({
    queryKey: [QueryKeys.IMPORT_RUNS, "IMPORT", 100],
    queryFn: () => getImportRuns("IMPORT", 100, 0),
  });

  const deleteMutation = useMutation({
    mutationFn: (runId: string) => deleteImportRun(runId),
    onSuccess: (deletedCount) => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.IMPORT_RUNS] });
      queryClient.invalidateQueries({ queryKey: [QueryKeys.ACTIVITIES] });
      toast({
        title: "Import deleted",
        description: `Successfully deleted ${deletedCount} activities.`,
      });
      setDeletingId(null);
    },
    onError: (error) => {
      toast({
        title: "Error deleting import",
        description: String(error),
        variant: "destructive",
      });
      setDeletingId(null);
    },
  });

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-12">
        <Loader2 className="text-muted-foreground h-8 w-8 animate-spin" />
      </div>
    );
  }

  if (importRuns.length === 0) {
    return (
      <div className="space-y-4 p-4">
        <h1 className="text-2xl font-bold">Import History</h1>
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12">
            <FileText className="text-muted-foreground mb-4 h-12 w-12" />
            <p className="text-muted-foreground text-lg">No CSV imports yet</p>
            <p className="text-muted-foreground text-sm">
              Import activities from a CSV file to see them here.
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-4 p-4">
      <div>
        <h1 className="text-2xl font-bold">Import History</h1>
        <p className="text-muted-foreground text-sm">
          View and manage your past CSV imports. Deleting an import will remove all activities from that import.
        </p>
      </div>

      <div className="space-y-3">
        {importRuns.map((run) => (
          <ImportRunCard
            key={run.id}
            run={run}
            isDeleting={deletingId === run.id}
            onDelete={() => {
              setDeletingId(run.id);
              deleteMutation.mutate(run.id);
            }}
          />
        ))}
      </div>
    </div>
  );
}

function ImportRunCard({
  run,
  isDeleting,
  onDelete,
}: {
  run: ImportRun;
  isDeleting: boolean;
  onDelete: () => void;
}) {
  const config = statusConfig[run.status];
  const StatusIcon = config.icon;

  return (
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <FileText className="text-muted-foreground h-5 w-5" />
            <div>
              <CardTitle className="text-base">
                {format(new Date(run.startedAt), "MMM d, yyyy 'at' h:mm a")}
              </CardTitle>
              <CardDescription className="text-xs">
                Account: {run.accountId}
              </CardDescription>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Badge variant={config.variant} className="gap-1">
              <StatusIcon className={`h-3 w-3 ${run.status === "RUNNING" ? "animate-spin" : ""}`} />
              {config.label}
            </Badge>
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="text-destructive hover:text-destructive h-8 w-8"
                  disabled={isDeleting || run.status === "RUNNING"}
                >
                  {isDeleting ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Trash2 className="h-4 w-4" />
                  )}
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>Delete this import?</AlertDialogTitle>
                  <AlertDialogDescription>
                    This will permanently delete all {run.summary?.inserted ?? 0} activities
                    from this import. This action cannot be undone.
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>Cancel</AlertDialogCancel>
                  <AlertDialogAction onClick={onDelete}>
                    Delete Import
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </div>
        </div>
      </CardHeader>
      {run.summary && (
        <CardContent className="pt-0">
          <div className="flex gap-6 text-sm">
            <div>
              <span className="text-muted-foreground">Inserted: </span>
              <span className="font-medium text-green-600">{run.summary.inserted}</span>
            </div>
            <div>
              <span className="text-muted-foreground">Skipped: </span>
              <span className="font-medium">{run.summary.skipped}</span>
            </div>
            {run.summary.errors > 0 && (
              <div>
                <span className="text-muted-foreground">Errors: </span>
                <span className="font-medium text-red-600">{run.summary.errors}</span>
              </div>
            )}
          </div>
          {run.error && (
            <p className="mt-2 text-sm text-red-600">{run.error}</p>
          )}
        </CardContent>
      )}
    </Card>
  );
}
