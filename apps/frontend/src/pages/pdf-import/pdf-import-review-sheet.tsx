import { useEffect, useState } from "react";
import { usePdfImportDetail, useConfirmPdfImport, useCheckPdfImport } from "@/hooks/use-pdf-import";
import { useQuery } from "@tanstack/react-query";
import { getAccounts } from "@/adapters";
import { QueryKeys } from "@/lib/query-keys";
import type { PdfTransaction } from "@/lib/types";
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
  Skeleton,
} from "@wealthfolio/ui";

interface Props {
  importId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

function transactionsToActivityImports(transactions: PdfTransaction[], accountId: string) {
  return transactions.map((t, i) => ({
    date: t.date,
    symbol: "",
    activityType: t.type,
    quantity: null,
    unitPrice: null,
    currency: t.currency,
    fee: t.fee != null ? String(t.fee) : null,
    amount: String(t.amount),
    comment: t.description,
    accountId,
    isDraft: false,
    isValid: true,
    lineNumber: i + 1,
    forceImport: false,
  }));
}

export function PdfImportReviewSheet({ importId, open, onOpenChange }: Props) {
  const [accountId, setAccountId] = useState("");
  const { data: importData, isLoading } = usePdfImportDetail(importId);
  const { data: accounts } = useQuery({
    queryKey: [QueryKeys.ACCOUNTS],
    queryFn: () => getAccounts(),
  });
  const confirmMutation = useConfirmPdfImport();
  const checkMutation = useCheckPdfImport();

  useEffect(() => {
    if (!open) return;
    const suggestedAccountId = importData?.suggestedAccountId;
    if (suggestedAccountId && accounts?.some((account) => account.id === suggestedAccountId)) {
      setAccountId(suggestedAccountId);
      return;
    }
    setAccountId("");
  }, [accounts, importData?.suggestedAccountId, open, importId]);

  const handleCheck = () => {
    if (!accountId) return;
    checkMutation.mutate({ id: importId, request: { accountId } });
  };

  const handleConfirm = () => {
    if (!accountId || !importData) return;
    const activities = transactionsToActivityImports(importData.transactions, accountId);
    confirmMutation.mutate(
      { id: importId, request: { accountId, activities } },
      { onSuccess: () => onOpenChange(false) },
    );
  };

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" className="flex w-full flex-col sm:max-w-2xl">
        <SheetHeader>
          <SheetTitle>{importData?.fileName ?? "Review Import"}</SheetTitle>
        </SheetHeader>

        {isLoading ? (
          <div className="space-y-3 p-4">
            <Skeleton className="h-10 w-full" />
            <Skeleton className="h-40 w-full" />
          </div>
        ) : importData ? (
          <div className="flex flex-1 flex-col gap-4 overflow-hidden p-4">
            {/* Account selector */}
            <div>
              <label className="mb-1.5 block text-sm font-medium">Target Account</label>
              <Select value={accountId} onValueChange={setAccountId}>
                <SelectTrigger>
                  <SelectValue placeholder="Select an account..." />
                </SelectTrigger>
                <SelectContent>
                  {accounts?.map((a) => (
                    <SelectItem key={a.id} value={a.id}>
                      {a.name} ({a.currency})
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {importData.suggestedAccountId && (
                <p className="text-muted-foreground mt-1.5 text-xs">
                  Preselected from incoming statement metadata.
                </p>
              )}
            </div>

            {/* Transaction table */}
            <div className="flex-1 overflow-auto rounded-lg border">
              <table className="w-full text-sm">
                <thead className="bg-muted/50 sticky top-0">
                  <tr>
                    <th className="px-3 py-2 text-left font-medium">Date</th>
                    <th className="px-3 py-2 text-left font-medium">Type</th>
                    <th className="px-3 py-2 text-left font-medium">Description</th>
                    <th className="px-3 py-2 text-right font-medium">Amount</th>
                    <th className="px-3 py-2 text-left font-medium">Currency</th>
                  </tr>
                </thead>
                <tbody>
                  {importData.transactions.map((t, i) => (
                    <tr key={i} className="border-t">
                      <td className="px-3 py-1.5">{t.date}</td>
                      <td className="px-3 py-1.5">
                        <span className="bg-muted rounded px-1.5 py-0.5 text-xs font-medium">
                          {t.type}
                        </span>
                      </td>
                      <td className="max-w-[200px] truncate px-3 py-1.5">{t.description}</td>
                      <td className="px-3 py-1.5 text-right font-mono">{t.amount.toFixed(2)}</td>
                      <td className="px-3 py-1.5">{t.currency}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            {/* Check result */}
            {checkMutation.data && (
              <div className="bg-muted rounded-lg p-3 text-sm">
                {checkMutation.data.duplicateCount} of {checkMutation.data.totalCount} transactions
                may be duplicates.
              </div>
            )}

            {/* Confirm result */}
            {confirmMutation.isSuccess && (
              <div className="bg-success/10 text-success rounded-lg p-3 text-sm">
                Successfully imported {confirmMutation.data.importedCount} activities.
              </div>
            )}

            {/* Error display */}
            {(checkMutation.isError || confirmMutation.isError) && (
              <div className="bg-destructive/10 text-destructive rounded-lg p-3 text-sm">
                {(checkMutation.error ?? confirmMutation.error)?.message}
              </div>
            )}

            {/* Actions */}
            <div className="flex items-center justify-end gap-2 border-t pt-3">
              <Button
                size="sm"
                variant="outline"
                onClick={handleCheck}
                disabled={!accountId || checkMutation.isPending}
              >
                {checkMutation.isPending && <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />}
                Check Duplicates
              </Button>
              <Button
                size="sm"
                onClick={handleConfirm}
                disabled={!accountId || confirmMutation.isPending}
              >
                {confirmMutation.isPending && (
                  <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />
                )}
                Import {importData.transactions.length} Transactions
              </Button>
            </div>
          </div>
        ) : (
          <div className="flex flex-1 items-center justify-center">
            <p className="text-muted-foreground text-sm">Import not found or expired.</p>
          </div>
        )}
      </SheetContent>
    </Sheet>
  );
}
