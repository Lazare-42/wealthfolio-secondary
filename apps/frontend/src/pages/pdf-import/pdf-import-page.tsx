import { useCallback, useState } from "react";
import { usePdfImportsStaged, useUploadPdf, useDeletePdfImport } from "@/hooks/use-pdf-import";
import type { StagedImportSummary } from "@/lib/types";
import { Badge, Button, Icons, Page, PageContent, PageHeader, Skeleton } from "@wealthfolio/ui";
import { PdfImportReviewSheet } from "./pdf-import-review-sheet";

function UploadZone({
  onUpload,
  isUploading,
}: {
  onUpload: (file: File) => void;
  isUploading: boolean;
}) {
  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      const file = e.dataTransfer.files[0];
      if (file?.type === "application/pdf") onUpload(file);
    },
    [onUpload],
  );

  const handleFileInput = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (file) onUpload(file);
      e.target.value = "";
    },
    [onUpload],
  );

  return (
    <div
      className="border-muted-foreground/25 hover:border-muted-foreground/50 flex flex-col items-center justify-center rounded-lg border-2 border-dashed p-8 transition-colors"
      onDragOver={(e) => e.preventDefault()}
      onDrop={handleDrop}
    >
      {isUploading ? (
        <>
          <Icons.Spinner className="text-muted-foreground mb-3 h-10 w-10 animate-spin" />
          <p className="text-sm font-medium">Processing PDF...</p>
          <p className="text-muted-foreground mt-1 text-xs">Extracting transactions with AI</p>
        </>
      ) : (
        <>
          <Icons.FileText className="text-muted-foreground mb-3 h-10 w-10" />
          <p className="text-sm font-medium">Drop a PDF here or click to upload</p>
          <p className="text-muted-foreground mt-1 text-xs">
            Bank or brokerage statement PDF files
          </p>
          <label className="mt-3 cursor-pointer">
            <Button size="sm" variant="outline" asChild>
              <span>
                <Icons.Upload className="mr-2 h-4 w-4" />
                Choose File
              </span>
            </Button>
            <input type="file" accept=".pdf" className="hidden" onChange={handleFileInput} />
          </label>
        </>
      )}
    </div>
  );
}

function StagedCard({
  summary,
  onReview,
  onDelete,
}: {
  summary: StagedImportSummary;
  onReview: () => void;
  onDelete: () => void;
}) {
  return (
    <div className="bg-card rounded-lg border p-4">
      <div className="flex items-center justify-between">
        <div className="min-w-0 flex-1">
          <h3 className="truncate text-sm font-medium">{summary.fileName}</h3>
          <p className="text-muted-foreground mt-0.5 text-xs">
            {summary.transactionCount} transactions &middot;{" "}
            {summary.source === "FolderWatcher" ? "Auto-detected" : "Uploaded"} &middot;{" "}
            {new Date(summary.stagedAt).toLocaleString()}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Badge variant="secondary" className="text-[10px]">
            {summary.transactionCount} txns
          </Badge>
          <Button size="sm" onClick={onReview}>
            Review
          </Button>
          <Button size="sm" variant="ghost" onClick={onDelete}>
            <Icons.Trash className="h-4 w-4" />
          </Button>
        </div>
      </div>
    </div>
  );
}

function EmptyState() {
  return (
    <div className="flex flex-col items-center justify-center py-12">
      <div className="bg-muted mb-4 flex h-14 w-14 items-center justify-center rounded-full">
        <Icons.FileText className="text-muted-foreground h-7 w-7" />
      </div>
      <h2 className="mb-1 text-base font-semibold">No Staged Imports</h2>
      <p className="text-muted-foreground max-w-sm text-center text-sm">
        Upload a PDF bank statement above, or place PDF files in the{" "}
        <code className="text-xs">pdf-inbox/</code> folder for automatic processing.
      </p>
    </div>
  );
}

export default function PdfImportPage() {
  const [reviewId, setReviewId] = useState<string | null>(null);
  const { data: staged, isLoading, error } = usePdfImportsStaged();
  const uploadMutation = useUploadPdf();
  const deleteMutation = useDeletePdfImport();

  return (
    <Page>
      <PageHeader
        heading="PDF Import"
        text="Extract transactions from bank statement PDFs using AI"
      />
      <PageContent className="mt-6 space-y-6">
        <UploadZone
          onUpload={(file) => uploadMutation.mutate(file)}
          isUploading={uploadMutation.isPending}
        />

        {uploadMutation.isError && (
          <div className="bg-destructive/10 text-destructive rounded-lg p-3 text-sm">
            {uploadMutation.error.message}
          </div>
        )}

        {error && (
          <div className="bg-destructive/10 text-destructive rounded-lg p-3 text-sm">
            {error.message}
          </div>
        )}

        {isLoading ? (
          <div className="space-y-3">
            <Skeleton className="h-16 rounded-lg" />
            <Skeleton className="h-16 rounded-lg" />
          </div>
        ) : staged && staged.length > 0 ? (
          <div className="space-y-2">
            <h2 className="text-sm font-medium">Staged Imports</h2>
            {staged.map((s) => (
              <StagedCard
                key={s.id}
                summary={s}
                onReview={() => setReviewId(s.id)}
                onDelete={() => deleteMutation.mutate(s.id)}
              />
            ))}
          </div>
        ) : (
          <EmptyState />
        )}
      </PageContent>

      {reviewId && (
        <PdfImportReviewSheet
          importId={reviewId}
          open={!!reviewId}
          onOpenChange={(open) => {
            if (!open) setReviewId(null);
          }}
        />
      )}
    </Page>
  );
}
