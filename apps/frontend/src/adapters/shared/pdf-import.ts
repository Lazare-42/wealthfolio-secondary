import type { ActivityImport } from "@/lib/types";
import { invoke } from "./platform";

export interface StagedImportSummary {
  id: string;
  filename: string;
  activityCount: number;
  createdAt: string;
}

export interface StagedImport {
  id: string;
  filename: string;
  activities: ActivityImport[];
  createdAt: string;
}

export interface PdfImportConfirmRequest {
  accountId: string;
  activities: ActivityImport[];
}

export interface PdfImportCheckRequest {
  accountId: string;
  activities: ActivityImport[];
}

export const getPdfImportsStaged = async (): Promise<StagedImportSummary[]> => {
  return invoke<StagedImportSummary[]>("pdf_imports_staged");
};

export const getPdfImportDetail = async (id: string): Promise<StagedImport> => {
  return invoke<StagedImport>("pdf_import_detail", { id });
};

export const confirmPdfImport = async (
  id: string,
  request: PdfImportConfirmRequest,
): Promise<unknown> => {
  return invoke("pdf_import_confirm", { id, request });
};

export const checkPdfImport = async (
  id: string,
  request: PdfImportCheckRequest,
): Promise<ActivityImport[]> => {
  return invoke<ActivityImport[]>("pdf_import_check", { id, request });
};

export const discardPdfImport = async (id: string): Promise<void> => {
  return invoke("pdf_import_discard", { id });
};
