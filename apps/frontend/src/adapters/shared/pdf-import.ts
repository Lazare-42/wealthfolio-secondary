import type {
  PdfImportCheckRequest,
  PdfImportCheckResponse,
  PdfImportConfirmRequest,
  PdfImportConfirmResponse,
  StagedImport,
  StagedImportSummary,
} from "@/lib/types";
import { invoke } from "./platform";

export const getPdfImportsStaged = async (): Promise<StagedImportSummary[]> => {
  return invoke<StagedImportSummary[]>("get_pdf_imports_staged");
};

export const getPdfImportDetail = async (id: string): Promise<StagedImport> => {
  return invoke<StagedImport>("get_pdf_import_detail", { id });
};

export const deletePdfImportStaged = async (id: string): Promise<void> => {
  return invoke<void>("delete_pdf_import_staged", { id });
};

export const confirmPdfImport = async (
  id: string,
  request: PdfImportConfirmRequest,
): Promise<PdfImportConfirmResponse> => {
  return invoke<PdfImportConfirmResponse>("confirm_pdf_import", { id, request });
};

export const checkPdfImport = async (
  id: string,
  request: PdfImportCheckRequest,
): Promise<PdfImportCheckResponse> => {
  return invoke<PdfImportCheckResponse>("check_pdf_import", { id, request });
};

/**
 * Upload a PDF file for processing. Uses raw fetch with multipart/form-data
 * since the standard invoke() sends JSON.
 */
export const uploadPdfImport = async (file: File): Promise<StagedImport> => {
  const formData = new FormData();
  formData.append("file", file);

  const res = await fetch("/api/v1/pdf-imports/upload", {
    method: "POST",
    body: formData,
    credentials: "same-origin",
  });

  if (!res.ok) {
    let msg = res.statusText;
    try {
      const err = await res.json();
      msg = (err?.message ?? msg) as string;
    } catch {
      void 0;
    }
    throw new Error(msg);
  }

  return res.json() as Promise<StagedImport>;
};
