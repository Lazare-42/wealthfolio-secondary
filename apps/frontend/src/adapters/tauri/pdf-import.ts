// Tauri-specific PDF import commands
// PDF upload is server-only, so this is a no-op for desktop builds.
import type { StagedImport } from "../shared/pdf-import";

export const uploadPdf = async (_file: File): Promise<StagedImport> => {
  throw new Error(
    "PDF upload is only available in web/server mode. Use the pdf-inbox folder instead.",
  );
};
