import { useCallback, useRef, useState } from "react";
import { usePdfImportsStaged, useDiscardPdfImport, useUploadPdf } from "@/hooks/use-pdf-import";
import type { StagedImportSummary } from "@/adapters";
import { Badge, Button, Icons, Page, PageContent, PageHeader, Skeleton } from "@wealthfolio/ui";
import { PdfImportReviewSheet } from "./pdf-import-review-sheet";

function StagedImportCard({
  item,
  onReview,
  onDiscard,
  isDiscarding,
}: {
  item: StagedImportSummary;
  onReview: () => void;
  onDiscard: () => void;
  isDiscarding: boolean;
}) {
  return (
    <div className="bg-card rounded-lg border p-4">
      <div className="flex items-center justify-between">
        <div className="min-w-0 flex-1">
          <h3 className="truncate text-sm font-medium">{item.filename}</h3>
          <p className="text-muted-foreground mt-0.5 text-xs">
            {item.activityCount} transactions &middot; {new Date(item.createdAt).toLocaleString()}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Badge className="bg-blue-500/15 text-[10px] font-medium text-blue-600 dark:text-blue-400">
            {item.activityCount} pending
          </Badge>
          <Button size="sm" onClick={onReview}>
            Review
          </Button>
          <Button size="sm" variant="outline" onClick={onDiscard} disabled={isDiscarding}>
            {isDiscarding ? (
              <Icons.Spinner className="h-4 w-4 animate-spin" />
            ) : (
              <Icons.Trash className="h-4 w-4" />
            )}
          </Button>
        </div>
      </div>
    </div>
  );
}

function UploadZone({
  onUpload,
  isUploading,
}: {
  onUpload: (file: File) => void;
  isUploading: boolean;
}) {
  const [isDragOver, setIsDragOver] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setIsDragOver(false);
      const file = e.dataTransfer.files[0];
      if (file && file.name.toLowerCase().endsWith(".pdf")) {
        onUpload(file);
      }
    },
    [onUpload],
  );

  const handleFileSelect = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (file) {
        onUpload(file);
      }
      // Reset so the same file can be selected again
      e.target.value = "";
    },
    [onUpload],
  );

  return (
    <div
      className={`rounded-lg border-2 border-dashed p-8 text-center transition-colors ${
        isDragOver
          ? "border-primary bg-primary/5"
          : "border-muted-foreground/25 hover:border-muted-foreground/50"
      }`}
      onDragOver={(e) => {
        e.preventDefault();
        setIsDragOver(true);
      }}
      onDragLeave={() => setIsDragOver(false)}
      onDrop={handleDrop}
    >
      <input
        ref={inputRef}
        type="file"
        accept=".pdf"
        className="hidden"
        onChange={handleFileSelect}
      />
      {isUploading ? (
        <div className="flex flex-col items-center gap-2">
          <Icons.Spinner className="text-muted-foreground h-8 w-8 animate-spin" />
          <p className="text-muted-foreground text-sm">Parsing PDF with AI...</p>
        </div>
      ) : (
        <div className="flex flex-col items-center gap-2">
          <Icons.Upload className="text-muted-foreground h-8 w-8" />
          <p className="text-sm">
            <button
              type="button"
              className="text-primary font-medium underline-offset-4 hover:underline"
              onClick={() => inputRef.current?.click()}
            >
              Choose a PDF
            </button>{" "}
            or drag and drop
          </p>
          <p className="text-muted-foreground text-xs">
            Bank or broker statement (text-based PDF, not scanned images)
          </p>
        </div>
      )}
    </div>
  );
}

function EmptyState({
  onUpload,
  isUploading,
}: {
  onUpload: (file: File) => void;
  isUploading: boolean;
}) {
  return (
    <div className="flex flex-col items-center justify-center py-12">
      <div className="bg-muted mb-6 flex h-16 w-16 items-center justify-center rounded-full">
        <Icons.FileText className="text-muted-foreground h-8 w-8" />
      </div>
      <h2 className="mb-2 text-lg font-semibold">No Staged PDF Imports</h2>
      <p className="text-muted-foreground mb-6 max-w-md text-center text-sm">
        Upload a PDF statement below, or drop files into the{" "}
        <code className="bg-muted rounded px-1 py-0.5 text-xs">pdf-inbox</code> folder. You can also
        use <code className="bg-muted rounded px-1 py-0.5 text-xs">rsync</code> or any file sync
        tool to automate delivery. The server polls for new files every 30 seconds.
      </p>
      <div className="w-full max-w-md">
        <UploadZone onUpload={onUpload} isUploading={isUploading} />
      </div>
    </div>
  );
}

export default function PdfImportPage() {
  const { data: staged, isLoading, error } = usePdfImportsStaged();
  const discardMutation = useDiscardPdfImport();
  const uploadMutation = useUploadPdf();
  const [reviewId, setReviewId] = useState<string | null>(null);

  const handleUpload = useCallback(
    (file: File) => {
      uploadMutation.mutate(file, {
        onSuccess: (result) => {
          // Automatically open review for the newly uploaded import
          setReviewId(result.id);
        },
      });
    },
    [uploadMutation],
  );

  const hasStaged = staged && staged.length > 0;

  return (
    <Page>
      <PageHeader heading="PDF Import" text="AI-powered PDF statement parsing with staged review" />
      <PageContent className="mt-6">
        {uploadMutation.isError && (
          <div className="bg-destructive/10 text-destructive mb-4 rounded-lg p-3 text-sm">
            Upload failed: {uploadMutation.error.message}
          </div>
        )}

        {error && (
          <div className="flex min-h-[300px] flex-col items-center justify-center">
            <div className="bg-destructive/10 mb-4 flex h-12 w-12 items-center justify-center rounded-full">
              <Icons.AlertCircle className="text-destructive h-6 w-6" />
            </div>
            <h2 className="mb-1 text-base font-medium">Failed to load staged imports</h2>
            <p className="text-muted-foreground mb-4 text-sm">{error.message}</p>
          </div>
        )}

        {!error && (
          <>
            {isLoading ? (
              <div className="space-y-3">
                <Skeleton className="h-16 rounded-lg" />
                <Skeleton className="h-16 rounded-lg" />
              </div>
            ) : hasStaged ? (
              <div className="space-y-4">
                {/* Upload zone at top when there are existing staged imports */}
                <UploadZone onUpload={handleUpload} isUploading={uploadMutation.isPending} />

                <div className="space-y-2">
                  {staged.map((item) => (
                    <StagedImportCard
                      key={item.id}
                      item={item}
                      onReview={() => setReviewId(item.id)}
                      onDiscard={() => discardMutation.mutate(item.id)}
                      isDiscarding={discardMutation.isPending}
                    />
                  ))}
                </div>
              </div>
            ) : (
              <EmptyState onUpload={handleUpload} isUploading={uploadMutation.isPending} />
            )}
          </>
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
