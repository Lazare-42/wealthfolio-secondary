import type { ChatSourceEmail } from "@/lib/types";
import { invoke, logger } from "./platform";

export const listSourceEmails = async (params?: {
  threadId?: string;
  limit?: number;
}): Promise<ChatSourceEmail[]> => {
  try {
    return await invoke<ChatSourceEmail[]>("list_source_emails", params ?? {});
  } catch (error) {
    logger.error("Error listing source emails.");
    throw error;
  }
};
