import { useState } from 'react';
import type { BackendActivityImport } from '@/lib/types';

export function useDocumentParser() {
  const [isAiParsing, setIsAiParsing] = useState(false);
  const [parseError, setParseError] = useState<string | null>(null);

  const parseDocument = async (file: File): Promise<BackendActivityImport[]> => {
    setIsAiParsing(true);
    setParseError(null);

    try {
      // 1. Validate file
      const maxSize = 32 * 1024 * 1024; // 32MB
      if (file.size > maxSize) {
        throw new Error('File too large. Maximum size is 32MB.');
      }

      // 2. Read file as bytes
      // const arrayBuffer = await file.arrayBuffer();
      // const fileBytes = Array.from(new Uint8Array(arrayBuffer));

      // 3. Determine document type hint
      // const documentType = inferDocumentType(file.name);

      // 4. Call backend
      // TODO: Implement parseFinancialDocument command
      // const result = await parseFinancialDocument({
      //   file_bytes: fileBytes,
      //   file_type: file.type || inferMimeType(file.name),
      //   document_type: documentType,
      // });

      // For now, throw error until command is implemented
      throw new Error('AI document parsing not yet implemented. Please configure AI settings first.');

      // return result;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      setParseError(message);
      throw error;
    } finally {
      setIsAiParsing(false);
    }
  };

  return { parseDocument, isAiParsing, parseError };
}

// Helper functions for future use
// function inferDocumentType(filename: string): string {
//   const lower = filename.toLowerCase();
//   if (lower.includes('loan') || lower.includes('mortgage') || lower.includes('amortization')) {
//     return 'loan_schedule';
//   }
//   if (lower.includes('statement') || lower.includes('bank')) {
//     return 'bank_statement';
//   }
//   if (lower.includes('broker') || lower.includes('trading')) {
//     return 'brokerage_report';
//   }
//   return 'auto';
// }

// function inferMimeType(filename: string): string {
//   const ext = filename.split('.').pop()?.toLowerCase();
//   const mimeTypes: Record<string, string> = {
//     pdf: 'application/pdf',
//     png: 'image/png',
//     jpg: 'image/jpeg',
//     jpeg: 'image/jpeg',
//     webp: 'image/webp',
//   };
//   return mimeTypes[ext || ''] || 'application/octet-stream';
// }
