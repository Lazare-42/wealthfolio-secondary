import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { format } from "date-fns";
import { Trash2, FileText, AlertTriangle, CheckCircle2, Loader2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { useToast } from "@/components/ui/use-toast";
import { QueryKeys } from "@/lib/query-keys";
import { getImportSessions, deleteImportSession } from "@/commands/activity-import";
import type { ImportSessionSummary } from "@/lib/types";

export default function ImportHistoryPage() {
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const [sessionToDelete, setSessionToDelete] = useState<ImportSessionSummary | null>(null);

  const {
    data: sessions = [],
    isLoading,
    error,
  } = useQuery({
    queryKey: [QueryKeys.IMPORT_SESSIONS],
    queryFn: getImportSessions,
  });

  const deleteMutation = useMutation({
    mutationFn: (sessionId: string) => deleteImportSession(sessionId),
    onSuccess: (deletedCount) => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.IMPORT_SESSIONS] });
      queryClient.invalidateQueries({ queryKey: [QueryKeys.ACTIVITIES] });
      toast({
        title: "Import deleted",
        description: `Successfully deleted ${deletedCount} activities.`,
      });
      setSessionToDelete(null);
    },
    onError: (error) => {
      toast({
        title: "Error",
        description: `Failed to delete import: ${error}`,
        variant: "destructive",
      });
    },
  });

  const formatDate = (dateStr: string) => {
    try {
      return format(new Date(dateStr), "MMM d, yyyy HH:mm");
    } catch {
      return dateStr;
    }
  };

  const getStatusBadge = (session: ImportSessionSummary) => {
    if (session.failedCount > 0) {
      return (
        <Badge variant="destructive" className="gap-1">
          <AlertTriangle className="h-3 w-3" />
          Partial
        </Badge>
      );
    }
    return (
      <Badge variant="default" className="gap-1 bg-green-600">
        <CheckCircle2 className="h-3 w-3" />
        Success
      </Badge>
    );
  };

  if (isLoading) {
    return (
      <div className="flex h-64 items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-64 items-center justify-center">
        <p className="text-destructive">Failed to load import history</p>
      </div>
    );
  }

  return (
    <div className="container mx-auto space-y-6 p-6">
      <Card>
        <CardHeader>
          <CardTitle>Import History</CardTitle>
          <CardDescription>
            View and manage your past CSV imports. Deleting an import will remove all activities
            that were imported in that session.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {sessions.length === 0 ? (
            <div className="flex h-32 flex-col items-center justify-center text-muted-foreground">
              <FileText className="mb-2 h-8 w-8" />
              <p>No import history found</p>
              <p className="text-sm">Import activities to see them here</p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Date</TableHead>
                  <TableHead>Account</TableHead>
                  <TableHead>File</TableHead>
                  <TableHead className="text-center">Activities</TableHead>
                  <TableHead className="text-center">Status</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {sessions.map((session) => (
                  <TableRow key={session.id}>
                    <TableCell className="font-medium">
                      {formatDate(session.importedAt)}
                    </TableCell>
                    <TableCell>{session.accountName}</TableCell>
                    <TableCell className="max-w-[200px] truncate">
                      {session.fileName || "-"}
                    </TableCell>
                    <TableCell className="text-center">
                      <span className="font-medium">{session.successCount}</span>
                      {session.failedCount > 0 && (
                        <span className="text-destructive"> / {session.failedCount} failed</span>
                      )}
                    </TableCell>
                    <TableCell className="text-center">{getStatusBadge(session)}</TableCell>
                    <TableCell className="text-right">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setSessionToDelete(session)}
                        disabled={deleteMutation.isPending}
                      >
                        <Trash2 className="h-4 w-4 text-destructive" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      <AlertDialog open={!!sessionToDelete} onOpenChange={() => setSessionToDelete(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Import</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete this import? This will permanently remove{" "}
              <strong>{sessionToDelete?.successCount} activities</strong> that were imported on{" "}
              {sessionToDelete && formatDate(sessionToDelete.importedAt)}.
              <br />
              <br />
              This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => sessionToDelete && deleteMutation.mutate(sessionToDelete.id)}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {deleteMutation.isPending ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Trash2 className="mr-2 h-4 w-4" />
              )}
              Delete Import
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
