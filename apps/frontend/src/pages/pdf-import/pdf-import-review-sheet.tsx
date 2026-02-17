import { useState } from "react";
import { usePdfImportDetail, useConfirmPdfImport, useCheckPdfImport } from "@/hooks/use-pdf-import";
import { useAccounts } from "@/hooks/use-accounts";
import type { ActivityImport } from "@/lib/types";
import {
  Button,
  Icons,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@wealthfolio/ui";
import { useToast } from "@wealthfolio/ui/components/ui/use-toast";

interface PdfImportReviewSheetProps {
  importId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function PdfImportReviewSheet({ importId, open, onOpenChange }: PdfImportReviewSheetProps) {
  const { data: importData, isLoading } = usePdfImportDetail(importId);
  const { accounts = [] } = useAccounts();
  const confirmMutation = useConfirmPdfImport();
  const checkMutation = useCheckPdfImport();
  const [accountId, setAccountId] = useState<string>("");
  const [checkedActivities, setCheckedActivities] = useState<ActivityImport[] | null>(null);
  const { toast } = useToast();

  const activities = checkedActivities ?? importData?.activities ?? [];
  const hasErrors = activities.some((a) => a.errors && Object.keys(a.errors).length > 0);
  const isValidated = checkedActivities !== null;

  const handleCheck = () => {
    if (!accountId || !importData) return;
    checkMutation.mutate(
      { id: importId, request: { accountId, activities: importData.activities } },
      {
        onSuccess: (data) => {
          setCheckedActivities(data);
          const errorCount = data.filter(
            (a) => a.errors && Object.keys(a.errors).length > 0,
          ).length;
          if (errorCount > 0) {
            toast({
              title: "Validation complete",
              description: `${errorCount} activity(s) have errors. Fix or remove them before importing.`,
              variant: "destructive",
            });
          } else {
            toast({
              title: "Validation passed",
              description: `All ${data.length} activities are valid.`,
            });
          }
        },
        onError: (err) => {
          toast({ title: "Validation failed", description: String(err), variant: "destructive" });
        },
      },
    );
  };

  const handleConfirm = () => {
    if (!accountId) return;
    confirmMutation.mutate(
      { id: importId, request: { accountId, activities } },
      {
        onSuccess: () => {
          toast({
            title: "Import complete",
            description: `${activities.length} activities imported.`,
          });
          onOpenChange(false);
        },
        onError: (err) => {
          toast({ title: "Import failed", description: String(err), variant: "destructive" });
        },
      },
    );
  };

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full overflow-y-auto sm:max-w-3xl">
        <SheetHeader className="px-6">
          <SheetTitle>Review PDF Import: {importData?.filename}</SheetTitle>
        </SheetHeader>

        <div className="space-y-4 px-6 py-4">
          {/* Account selector */}
          <div className="space-y-2">
            <label className="text-sm font-medium">Target Account</label>
            <Select value={accountId} onValueChange={setAccountId}>
              <SelectTrigger>
                <SelectValue placeholder="Select account..." />
              </SelectTrigger>
              <SelectContent>
                {accounts.map((acc) => (
                  <SelectItem key={acc.id} value={acc.id}>
                    {acc.name} ({acc.currency})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Validate button */}
          {accountId && !checkedActivities && (
            <div className="space-y-1">
              <Button
                size="sm"
                variant="outline"
                onClick={handleCheck}
                disabled={checkMutation.isPending}
              >
                {checkMutation.isPending && <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />}
                Validate Activities
              </Button>
              <p className="text-muted-foreground text-xs">
                Validate before importing to check for duplicates and resolve symbols.
              </p>
            </div>
          )}

          {/* Activity table */}
          {isLoading ? (
            <p className="text-muted-foreground text-sm">Loading...</p>
          ) : (
            <div className="max-h-[60vh] overflow-auto rounded border">
              <table className="w-full text-left text-sm">
                <thead className="bg-muted/50 sticky top-0">
                  <tr>
                    <th className="px-3 py-2">Date</th>
                    <th className="px-3 py-2">Type</th>
                    <th className="px-3 py-2">Symbol</th>
                    <th className="px-3 py-2">Qty</th>
                    <th className="px-3 py-2">Price</th>
                    <th className="px-3 py-2">Amount</th>
                    <th className="px-3 py-2">Currency</th>
                    <th className="px-3 py-2">Status</th>
                  </tr>
                </thead>
                <tbody>
                  {activities.map((a, i) => {
                    const rowErrors = a.errors ? Object.values(a.errors).flat() : [];
                    const rowWarnings = a.warnings ? Object.values(a.warnings).flat() : [];
                    return (
                      <tr
                        key={i}
                        className={`border-t ${rowErrors.length > 0 ? "bg-destructive/5" : ""}`}
                      >
                        <td className="px-3 py-1.5">{String(a.date ?? "")}</td>
                        <td className="px-3 py-1.5">{a.activityType}</td>
                        <td className="px-3 py-1.5">{a.symbol || "-"}</td>
                        <td className="px-3 py-1.5">{a.quantity ?? "-"}</td>
                        <td className="px-3 py-1.5">{a.unitPrice ?? "-"}</td>
                        <td className="px-3 py-1.5">{a.amount ?? "-"}</td>
                        <td className="px-3 py-1.5">{a.currency}</td>
                        <td className="px-3 py-1.5">
                          {rowErrors.length > 0 ? (
                            <span className="text-destructive text-xs" title={rowErrors.join(", ")}>
                              {rowErrors.length} error(s)
                            </span>
                          ) : rowWarnings.length > 0 ? (
                            <span className="text-warning text-xs" title={rowWarnings.join(", ")}>
                              {rowWarnings.length} warning(s)
                            </span>
                          ) : checkedActivities ? (
                            <span className="text-success text-xs">OK</span>
                          ) : (
                            <span className="text-muted-foreground text-xs">Unchecked</span>
                          )}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}

          {/* Validation status message */}
          {hasErrors && isValidated && (
            <p className="text-destructive text-sm">
              Some activities have errors. Fix or remove them before importing.
            </p>
          )}

          {/* Actions */}
          <div className="flex items-center justify-end gap-2 pt-2">
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleConfirm}
              disabled={!accountId || !isValidated || confirmMutation.isPending || hasErrors}
            >
              {confirmMutation.isPending && <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />}
              Confirm Import ({activities.length})
            </Button>
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}
