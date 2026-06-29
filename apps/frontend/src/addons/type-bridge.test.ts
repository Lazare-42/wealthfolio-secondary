import { vi, describe, it, expect } from "vitest";
import { createSDKHostAPIBridge, type InternalHostAPI } from "./type-bridge";

describe("Addon Type Bridge", () => {
  describe("createSDKHostAPIBridge", () => {
    it("should create logger with addon prefix", () => {
      // Mock the internal API logger functions
      const mockLogError = vi.fn();
      const mockLogInfo = vi.fn();
      const mockLogWarn = vi.fn();
      const mockLogTrace = vi.fn();
      const mockLogDebug = vi.fn();

      // Create a minimal mock internal API with just the logger functions
      const mockInternalAPI: Partial<InternalHostAPI> = {
        logError: mockLogError,
        logInfo: mockLogInfo,
        logWarn: mockLogWarn,
        logTrace: mockLogTrace,
        logDebug: mockLogDebug,
      };

      // Create the SDK bridge with a test addon ID
      const sdkAPI = createSDKHostAPIBridge(mockInternalAPI as InternalHostAPI, "test-addon");

      // Test that logger methods add the addon prefix
      sdkAPI.logger.error("test error message");
      sdkAPI.logger.info("test info message");
      sdkAPI.logger.warn("test warning message");
      sdkAPI.logger.trace("test trace message");
      sdkAPI.logger.debug("test debug message");

      // Verify the logger functions were called with prefixed messages
      expect(mockLogError).toHaveBeenCalledWith("[test-addon] test error message");
      expect(mockLogInfo).toHaveBeenCalledWith("[test-addon] test info message");
      expect(mockLogWarn).toHaveBeenCalledWith("[test-addon] test warning message");
      expect(mockLogTrace).toHaveBeenCalledWith("[test-addon] test trace message");
      expect(mockLogDebug).toHaveBeenCalledWith("[test-addon] test debug message");
    });

    it("should use default addon ID when none provided", () => {
      const mockLogInfo = vi.fn();

      const mockInternalAPI: Partial<InternalHostAPI> = {
        logInfo: mockLogInfo,
      };

      // Create the SDK bridge without addon ID
      const sdkAPI = createSDKHostAPIBridge(mockInternalAPI as InternalHostAPI);

      sdkAPI.logger.info("test message");

      // Should use default addon ID
      expect(mockLogInfo).toHaveBeenCalledWith("[unknown-addon] test message");
    });

    it("should handle empty addon ID", () => {
      const mockLogInfo = vi.fn();

      const mockInternalAPI: Partial<InternalHostAPI> = {
        logInfo: mockLogInfo,
      };

      // Create the SDK bridge with empty addon ID
      const sdkAPI = createSDKHostAPIBridge(mockInternalAPI as InternalHostAPI, "");

      sdkAPI.logger.info("test message");

      // Should fallback to default addon ID for empty string
      expect(mockLogInfo).toHaveBeenCalledWith("[unknown-addon] test message");
    });

    it("should expose saved scenario host APIs", async () => {
      const mockGetScenarios = vi.fn().mockResolvedValue([]);
      const mockGetScenario = vi.fn().mockResolvedValue({ id: "scenario-1" });
      const mockCreateScenario = vi.fn().mockResolvedValue({ id: "scenario-2" });
      const mockUpdateScenarioEntry = vi.fn().mockResolvedValue({ id: "scenario-3" });
      const mockDeleteScenario = vi.fn().mockResolvedValue(undefined);

      const mockInternalAPI: Partial<InternalHostAPI> = {
        getScenarios: mockGetScenarios,
        getScenario: mockGetScenario,
        createScenario: mockCreateScenario,
        updateScenarioEntry: mockUpdateScenarioEntry,
        deleteScenario: mockDeleteScenario,
      };

      const sdkAPI = createSDKHostAPIBridge(mockInternalAPI as InternalHostAPI, "scenario-addon");

      await expect(sdkAPI.scenarios.getAll()).resolves.toEqual([]);
      await expect(sdkAPI.scenarios.getById("scenario-1")).resolves.toEqual({
        id: "scenario-1",
      });
      await sdkAPI.scenarios.create({
        name: "Scenario",
        accountScope: { type: "all" },
      });
      await sdkAPI.scenarios.update({
        id: "scenario-1",
        name: "Scenario",
        kind: "comparison",
        accountScope: { type: "all" },
        resolvedAccountIds: [],
        benchmarkSymbols: [],
        basket: [],
        assumptions: {},
        createdAt: "2026-06-28T00:00:00Z",
        updatedAt: "2026-06-28T00:00:00Z",
      });
      await sdkAPI.scenarios.delete("scenario-1");

      expect(mockGetScenarios).toHaveBeenCalledOnce();
      expect(mockGetScenario).toHaveBeenCalledWith("scenario-1");
      expect(mockCreateScenario).toHaveBeenCalledWith({
        name: "Scenario",
        accountScope: { type: "all" },
      });
      expect(mockUpdateScenarioEntry).toHaveBeenCalledWith({
        id: "scenario-1",
        name: "Scenario",
        kind: "comparison",
        accountScope: { type: "all" },
        resolvedAccountIds: [],
        benchmarkSymbols: [],
        basket: [],
        assumptions: {},
        createdAt: "2026-06-28T00:00:00Z",
        updatedAt: "2026-06-28T00:00:00Z",
      });
      expect(mockDeleteScenario).toHaveBeenCalledWith("scenario-1");
    });
  });
});
