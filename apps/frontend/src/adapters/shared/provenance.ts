import type {
  ActivitySource,
  ChatSourceEmail,
  NewActivitySource,
  NewChatSourceEmail,
} from "@/lib/types";
import { invoke, logger } from "./platform";

export const recordActivitySource = async (
  source: NewActivitySource,
): Promise<ActivitySource> => {
  try {
    return await invoke<ActivitySource>("record_activity_source", { source });
  } catch (error) {
    logger.error("Error recording activity source.");
    throw error;
  }
};

export const getActivitySources = async (activityId: string): Promise<ActivitySource[]> => {
  try {
    return await invoke<ActivitySource[]>("get_activity_sources", { activityId });
  } catch (error) {
    logger.error("Error loading activity sources.");
    throw error;
  }
};

export const saveSourceEmail = async (email: NewChatSourceEmail): Promise<ChatSourceEmail> => {
  try {
    return await invoke<ChatSourceEmail>("save_source_email", { email });
  } catch (error) {
    logger.error("Error saving source email.");
    throw error;
  }
};

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
