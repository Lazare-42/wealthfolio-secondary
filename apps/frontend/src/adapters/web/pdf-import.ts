// Web-specific PDF import commands (multipart upload)
import type { StagedImport } from "../shared/pdf-import";
import { API_PREFIX, logger } from "./core";
import { getAuthToken } from "@/lib/auth-token";

/**
 * Upload a PDF file for AI parsing.
 * Web implementation: POSTs multipart form data to /api/v1/pdf-imports/upload.
 */
export const uploadPdf = async (file: File): Promise<StagedImport> => {
  try {
    const formData = new FormData();
    formData.append("file", file);

    const headers: HeadersInit = {};
    const token = getAuthToken();
    if (token) {
      headers.Authorization = `Bearer ${token}`;
    }

    const response = await fetch(`${API_PREFIX}/pdf-imports/upload`, {
      method: "POST",
      headers,
      body: formData,
    });

    if (!response.ok) {
      const errorBody = await response.text();
      throw new Error(errorBody || `HTTP error! status: ${response.status}`);
    }

    return await response.json();
  } catch (err) {
    logger.error("Error uploading PDF file:", err);
    throw err;
  }
};
