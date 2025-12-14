import { getRunEnv, invokeTauri, invokeWeb, logger, RUN_ENV } from "@/adapters";

// AI Provider Configuration Types
export interface AIProviderConfig {
  type: 'disabled' | 'anthropic' | 'openai' | 'openrouter' | 'ollama' | 'mistral';
  model?: string;
  baseUrl?: string;
}

// CSV Mapping Types
export interface MappingSuggestion {
  field: string;
  suggested_column: string;
  confidence: number;
}

export interface MappingSuggestions {
  suggestions: MappingSuggestion[];
}

export interface SuggestMappingRequest {
  headers: string[];
  sample_rows: Record<string, string>[];
}

export interface SuggestMappingResponse {
  success: boolean;
  suggestions?: MappingSuggestions;
  error?: string;
}

// Command Functions

export const suggestCsvColumnMapping = async (
  request: SuggestMappingRequest
): Promise<SuggestMappingResponse> => {
  try {
    switch (getRunEnv()) {
      case RUN_ENV.DESKTOP:
        return invokeTauri("suggest_csv_column_mapping", { request });
      case RUN_ENV.WEB:
        return invokeWeb("suggest_csv_column_mapping", request as unknown as Record<string, unknown>);
      default:
        throw new Error(`Unsupported environment for AI CSV mapping`);
    }
  } catch (error) {
    logger.error("Error suggesting CSV column mapping.");
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Unknown error',
    };
  }
};

// Placeholder functions for future implementation
export const getAiProviderConfig = async (): Promise<AIProviderConfig> => {
  try {
    switch (getRunEnv()) {
      case RUN_ENV.DESKTOP:
        return invokeTauri("get_ai_provider_config");
      case RUN_ENV.WEB:
        return invokeWeb("get_ai_provider_config");
      default:
        throw new Error(`Unsupported environment`);
    }
  } catch (_error) {
    logger.error("Error fetching AI provider config.");
    // Return disabled as default
    return { type: 'disabled' };
  }
};

export const setAiProviderConfig = async (config: AIProviderConfig): Promise<void> => {
  try {
    switch (getRunEnv()) {
      case RUN_ENV.DESKTOP:
        await invokeTauri("set_ai_provider_config", { newConfig: config });
        break;
      case RUN_ENV.WEB:
        await invokeWeb("set_ai_provider_config", { config });
        break;
      default:
        throw new Error(`Unsupported environment`);
    }
  } catch (error) {
    logger.error("Error setting AI provider config.");
    throw error;
  }
};

export const setAiApiKey = async (provider: string, key: string): Promise<void> => {
  try {
    switch (getRunEnv()) {
      case RUN_ENV.DESKTOP:
        await invokeTauri("set_ai_api_key", { provider, key });
        break;
      case RUN_ENV.WEB:
        await invokeWeb("set_ai_api_key", { provider, key });
        break;
      default:
        throw new Error(`Unsupported environment`);
    }
  } catch (error) {
    logger.error("Error setting AI API key.");
    throw error;
  }
};

export const hasAiApiKey = async (provider: string): Promise<boolean> => {
  try {
    switch (getRunEnv()) {
      case RUN_ENV.DESKTOP:
        return invokeTauri("has_ai_api_key", { provider });
      case RUN_ENV.WEB:
        return invokeWeb("has_ai_api_key", { provider });
      default:
        throw new Error(`Unsupported environment`);
    }
  } catch (_error) {
    logger.error("Error checking AI API key.");
    return false;
  }
};

export const testAiConnection = async (): Promise<{
  success: boolean;
  message: string;
  model_used?: string;
}> => {
  try {
    switch (getRunEnv()) {
      case RUN_ENV.DESKTOP:
        return invokeTauri("test_ai_connection");
      case RUN_ENV.WEB:
        return invokeWeb("test_ai_connection");
      default:
        throw new Error(`Unsupported environment`);
    }
  } catch (error) {
    logger.error("Error testing AI connection.");
    return {
      success: false,
      message: error instanceof Error ? error.message : 'Connection test failed',
    };
  }
};
