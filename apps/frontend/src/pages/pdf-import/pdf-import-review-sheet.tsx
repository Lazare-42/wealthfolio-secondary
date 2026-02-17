import { useState, useEffect } from "react";
import { usePdfImportDetail, useConfirmPdfImport, useCheckPdfImport } from "@/hooks/use-pdf-import";
import { useAccounts } from "@/hooks/use-accounts";
import type { ActivityImport } from "@/lib/types";
import { ACTIVITY_TYPES } from "@/lib/constants";
import {
  Button,
  Icons,
  Input,
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
  const [editedActivities, setEditedActivities] = useState<ActivityImport[] | null>(null);
  const [checkedActivities, setCheckedActivities] = useState<ActivityImport[] | null>(null);
  const { toast } = useToast();

  // Initialize editable copy when import data loads
  useEffect(() => {
    if (importData?.activities && !editedActivities) {
      setEditedActivities([...importData.activities]);
    }
  }, [importData, editedActivities]);

  const activities = checkedActivities ?? editedActivities ?? importData?.activities ?? [];
  const validActivities = activities.filter((a) => !a.errors || Object.keys(a.errors).length === 0);
  const errorCount = activities.length - validActivities.length;
  const isValidated = checkedActivities !== null;

  const updateActivity = (index: number, field: keyof ActivityImport, value: string | null) => {
    const updated = [...activities];
    updated[index] = { ...updated[index], [field]: value };
    setEditedActivities(updated);
    // Clear validation since data changed
    setCheckedActivities(null);
  };

  const removeActivity = (index: number) => {
    const updated = activities.filter((_, i) => i !== index);
    setEditedActivities(updated);
    setCheckedActivities(null);
  };

  const handleCheck = () => {
    if (!accountId || activities.length === 0) return;
    checkMutation.mutate(
      { id: importId, request: { accountId, activities } },
      {
        onSuccess: (data) => {
          setCheckedActivities(data);
          const errCount = data.filter((a) => a.errors && Object.keys(a.errors).length > 0).length;
          if (errCount > 0) {
            toast({
              title: "Validation complete",
              description: `${errCount} activity(s) have errors. Fix or remove them, or import only valid ones.`,
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
    if (!accountId || validActivities.length === 0) return;
    confirmMutation.mutate(
      { id: importId, request: { accountId, activities: validActivities } },
      {
        onSuccess: () => {
          toast({
            title: "Import complete",
            description: `${validActivities.length} activities imported.${errorCount > 0 ? ` ${errorCount} skipped.` : ""}`,
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
      <SheetContent className="w-full overflow-y-auto sm:max-w-4xl">
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
          {accountId && (
            <div className="space-y-1">
              <Button
                size="sm"
                variant="outline"
                onClick={handleCheck}
                disabled={checkMutation.isPending || activities.length === 0}
              >
                {checkMutation.isPending && <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />}
                {isValidated ? "Re-validate" : "Validate Activities"}
              </Button>
              <p className="text-muted-foreground text-xs">
                {isValidated
                  ? "Re-validate after editing to update status."
                  : "Validate before importing to check for duplicates and resolve symbols."}
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
                    <th className="px-2 py-2">Date</th>
                    <th className="px-2 py-2">Type</th>
                    <th className="px-2 py-2">Symbol</th>
                    <th className="px-2 py-2">Qty</th>
                    <th className="px-2 py-2">Price</th>
                    <th className="px-2 py-2">Amount</th>
                    <th className="px-2 py-2">Ccy</th>
                    <th className="px-2 py-2">Status</th>
                    <th className="w-8 px-1 py-2"></th>
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
                        <td className="px-1 py-1">
                          <Input
                            className="h-7 w-[100px] text-xs"
                            value={String(a.date ?? "")}
                            onChange={(e) => updateActivity(i, "date", e.target.value)}
                          />
                        </td>
                        <td className="px-1 py-1">
                          <Select
                            value={a.activityType}
                            onValueChange={(v) => updateActivity(i, "activityType", v)}
                          >
                            <SelectTrigger className="h-7 w-[100px] text-xs">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              {ACTIVITY_TYPES.map((t) => (
                                <SelectItem key={t} value={t}>
                                  {t}
                                </SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                        </td>
                        <td className="px-1 py-1">
                          <Input
                            className="h-7 w-[90px] text-xs"
                            value={a.symbol ?? ""}
                            onChange={(e) => updateActivity(i, "symbol", e.target.value || null)}
                          />
                        </td>
                        <td className="px-1 py-1">
                          <Input
                            className="h-7 w-[70px] text-xs"
                            value={a.quantity ?? ""}
                            onChange={(e) => updateActivity(i, "quantity", e.target.value || null)}
                          />
                        </td>
                        <td className="px-1 py-1">
                          <Input
                            className="h-7 w-[70px] text-xs"
                            value={a.unitPrice ?? ""}
                            onChange={(e) => updateActivity(i, "unitPrice", e.target.value || null)}
                          />
                        </td>
                        <td className="px-1 py-1">
                          <Input
                            className="h-7 w-[80px] text-xs"
                            value={a.amount ?? ""}
                            onChange={(e) => updateActivity(i, "amount", e.target.value || null)}
                          />
                        </td>
                        <td className="px-1 py-1">
                          <Input
                            className="h-7 w-[50px] text-xs"
                            value={a.currency ?? ""}
                            onChange={(e) => updateActivity(i, "currency", e.target.value)}
                          />
                        </td>
                        <td className="px-1 py-1">
                          {rowErrors.length > 0 ? (
                            <span className="text-destructive text-xs" title={rowErrors.join(", ")}>
                              {rowErrors.join(", ")}
                            </span>
                          ) : rowWarnings.length > 0 ? (
                            <span className="text-warning text-xs" title={rowWarnings.join(", ")}>
                              {rowWarnings.join(", ")}
                            </span>
                          ) : checkedActivities ? (
                            <span className="text-success text-xs">OK</span>
                          ) : (
                            <span className="text-muted-foreground text-xs">-</span>
                          )}
                        </td>
                        <td className="px-1 py-1">
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-7 w-7 p-0"
                            onClick={() => removeActivity(i)}
                            title="Remove activity"
                          >
                            <Icons.Close className="h-3 w-3" />
                          </Button>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}

          {/* Validation status */}
          {isValidated && errorCount > 0 && (
            <p className="text-muted-foreground text-sm">
              {errorCount} activity(s) with errors will be skipped. {validActivities.length} valid
              activity(s) will be imported.
            </p>
          )}

          {/* Actions */}
          <div className="flex items-center justify-end gap-2 pt-2">
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleConfirm}
              disabled={
                !accountId ||
                !isValidated ||
                confirmMutation.isPending ||
                validActivities.length === 0
              }
            >
              {confirmMutation.isPending && <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />}
              Import {validActivities.length} of {activities.length} Activities
            </Button>
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}
