import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { QueryKeys } from "@/lib/query-keys";
import type { PdfImportConfirmRequest, PdfImportCheckRequest } from "@/lib/types";
import {
  getPdfImportsStaged,
  getPdfImportDetail,
  deletePdfImportStaged,
  confirmPdfImport,
  checkPdfImport,
  uploadPdfImport,
} from "@/adapters";

export function usePdfImportsStaged() {
  return useQuery({
    queryKey: [QueryKeys.PDF_IMPORTS_STAGED],
    queryFn: getPdfImportsStaged,
    refetchInterval: 30_000,
  });
}

export function usePdfImportDetail(id: string) {
  return useQuery({
    queryKey: [QueryKeys.PDF_IMPORTS_STAGED, id],
    queryFn: () => getPdfImportDetail(id),
    enabled: !!id,
  });
}

export function useUploadPdf() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (file: File) => uploadPdfImport(file),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.PDF_IMPORTS_STAGED] });
    },
  });
}

export function useDeletePdfImport() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deletePdfImportStaged(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.PDF_IMPORTS_STAGED] });
    },
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
