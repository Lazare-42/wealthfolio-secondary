import {
  ActivityImport,
  ImportMappingData,
  ImportSession,
  ImportSessionSummary,
  ImportWithSessionResponse,
} from "@/lib/types";
import { getRunEnv, RUN_ENV, invokeTauri, invokeWeb } from "@/adapters";
import { logger } from "@/adapters";

export const importActivities = async ({
  activities,
}: {
  activities: ActivityImport[];
}): Promise<ActivityImport[]> => {
  try {
    switch (getRunEnv()) {
      case RUN_ENV.DESKTOP:
        return invokeTauri("import_activities", {
          accountId: activities[0].accountId,
          activities: activities,
        });
      case RUN_ENV.WEB:
        return invokeWeb("import_activities", {
          accountId: activities[0].accountId,
          activities,
        });
      default:
        throw new Error(`Unsupported`);
    }
  } catch (error) {
    logger.error("Error checking activities import.");
    throw error;
  }
};

export const checkActivitiesImport = async ({
  account_id,
  activities,
}: {
  account_id: string;
  activities: ActivityImport[];
}): Promise<ActivityImport[]> => {
  try {
    switch (getRunEnv()) {
      case RUN_ENV.DESKTOP:
        return invokeTauri("check_activities_import", {
          accountId: account_id,
          activities: activities,
        });
      case RUN_ENV.WEB:
        return invokeWeb("check_activities_import", {
          accountId: account_id,
          activities,
        });
      default:
        throw new Error(`Unsupported`);
    }
  } catch (error) {
    logger.error("Error checking activities import.");
    throw error;
  }
};

export const getAccountImportMapping = async (accountId: string): Promise<ImportMappingData> => {
  try {
    switch (getRunEnv()) {
      case RUN_ENV.DESKTOP:
        return invokeTauri("get_account_import_mapping", { accountId });
      case RUN_ENV.WEB:
        return invokeWeb("get_account_import_mapping", { accountId });
      default:
        throw new Error(`Unsupported`);
    }
  } catch (error) {
    logger.error("Error fetching mapping.");
    throw error;
  }
};

export const saveAccountImportMapping = async (
  mapping: ImportMappingData,
): Promise<ImportMappingData> => {
  try {
    switch (getRunEnv()) {
      case RUN_ENV.DESKTOP:
        return invokeTauri("save_account_import_mapping", {
          mapping,
        });
      case RUN_ENV.WEB:
        return invokeWeb("save_account_import_mapping", { mapping });
      default:
        throw new Error(`Unsupported`);
    }
  } catch (error) {
    logger.error("Error saving mapping.");
    throw error;
  }
};

// Import session commands

export const importActivitiesWithSession = async ({
  activities,
  fileName,
}: {
  activities: ActivityImport[];
  fileName?: string;
}): Promise<ImportWithSessionResponse> => {
  try {
    switch (getRunEnv()) {
      case RUN_ENV.DESKTOP:
        return invokeTauri("import_activities_with_session", {
          accountId: activities[0].accountId,
          activities,
          fileName: fileName ?? null,
        });
      case RUN_ENV.WEB:
        return invokeWeb("import_activities_with_session", {
          accountId: activities[0].accountId,
          activities,
          fileName: fileName ?? null,
        });
      default:
        throw new Error(`Unsupported`);
    }
  } catch (error) {
    logger.error("Error importing activities with session.");
    throw error;
  }
};

export const getImportSessions = async (): Promise<ImportSessionSummary[]> => {
  try {
    switch (getRunEnv()) {
      case RUN_ENV.DESKTOP:
        return invokeTauri("get_import_sessions", {});
      case RUN_ENV.WEB:
        return invokeWeb("get_import_sessions", {});
      default:
        throw new Error(`Unsupported`);
    }
  } catch (error) {
    logger.error("Error fetching import sessions.");
    throw error;
  }
};

export const getImportSessionsByAccount = async (
  accountId: string,
): Promise<ImportSessionSummary[]> => {
  try {
    switch (getRunEnv()) {
      case RUN_ENV.DESKTOP:
        return invokeTauri("get_import_sessions_by_account", { accountId });
      case RUN_ENV.WEB:
        return invokeWeb("get_import_sessions_by_account", { accountId });
      default:
        throw new Error(`Unsupported`);
    }
  } catch (error) {
    logger.error("Error fetching import sessions by account.");
    throw error;
  }
};

export const getImportSession = async (sessionId: string): Promise<ImportSession> => {
  try {
    switch (getRunEnv()) {
      case RUN_ENV.DESKTOP:
        return invokeTauri("get_import_session", { sessionId });
      case RUN_ENV.WEB:
        return invokeWeb("get_import_session", { sessionId });
      default:
        throw new Error(`Unsupported`);
    }
  } catch (error) {
    logger.error("Error fetching import session.");
    throw error;
  }
};

export const deleteImportSession = async (sessionId: string): Promise<number> => {
  try {
    switch (getRunEnv()) {
      case RUN_ENV.DESKTOP:
        return invokeTauri("delete_import_session", { sessionId });
      case RUN_ENV.WEB:
        return invokeWeb("delete_import_session", { sessionId });
      default:
        throw new Error(`Unsupported`);
    }
  } catch (error) {
    logger.error("Error deleting import session.");
    throw error;
  }
};
