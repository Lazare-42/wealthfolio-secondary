import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { QueryKeys } from "@/lib/query-keys";
import {
  getPdfImportsStaged,
  getPdfImportDetail,
  confirmPdfImport,
  checkPdfImport,
  discardPdfImport,
  uploadPdf,
} from "@/adapters";
import type { PdfImportConfirmRequest, PdfImportCheckRequest } from "@/adapters";

export function usePdfImportsStaged() {
  return useQuery({
    queryKey: [QueryKeys.PDF_IMPORTS_STAGED],
    queryFn: getPdfImportsStaged,
    refetchInterval: 30_000, // poll every 30s for new staged imports
  });
}

export function usePdfImportDetail(id: string) {
  return useQuery({
    queryKey: QueryKeys.pdfImportDetail(id),
    queryFn: () => getPdfImportDetail(id),
    enabled: !!id,
  });
}

export function useConfirmPdfImport() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, request }: { id: string; request: PdfImportConfirmRequest }) =>
      confirmPdfImport(id, request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.PDF_IMPORTS_STAGED] });
      queryClient.invalidateQueries({ queryKey: [QueryKeys.ACTIVITIES] });
    },
  });
}

export function useCheckPdfImport() {
  return useMutation({
    mutationFn: ({ id, request }: { id: string; request: PdfImportCheckRequest }) =>
      checkPdfImport(id, request),
  });
}

export function useUploadPdf() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (file: File) => uploadPdf(file),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.PDF_IMPORTS_STAGED] });
    },
  });
}

export function useDiscardPdfImport() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => discardPdfImport(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.PDF_IMPORTS_STAGED] });
    },
  });
}
