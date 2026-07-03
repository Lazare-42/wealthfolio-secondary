import type {
  NewPortfolioScenario,
  PortfolioScenario,
  ScenarioPerformanceResult,
} from "@/lib/types";
import { invoke, logger } from "./platform";

export const getScenarios = async (): Promise<PortfolioScenario[]> => {
  try {
    return await invoke<PortfolioScenario[]>("get_scenarios");
  } catch (error) {
    logger.error("Error fetching scenarios.");
    throw error;
  }
};

export const getScenario = async (scenarioId: string): Promise<PortfolioScenario> => {
  try {
    return await invoke<PortfolioScenario>("get_scenario", { scenarioId });
  } catch (error) {
    logger.error("Error fetching scenario.");
    throw error;
  }
};

export const getScenarioPerformance = async (
  scenarioId: string,
  startDate?: string,
  endDate?: string,
): Promise<ScenarioPerformanceResult> => {
  try {
    return await invoke<ScenarioPerformanceResult>("calculate_scenario_performance", {
      scenarioId,
      startDate,
      endDate,
    });
  } catch (error) {
    logger.error("Error replaying scenario performance.");
    throw error;
  }
};

export const createScenario = async (
  scenario: NewPortfolioScenario,
): Promise<PortfolioScenario> => {
  try {
    return await invoke<PortfolioScenario>("create_scenario", { scenario });
  } catch (error) {
    logger.error("Error creating scenario.");
    throw error;
  }
};

export const updateScenarioEntry = async (
  scenario: PortfolioScenario,
): Promise<PortfolioScenario> => {
  try {
    const payload: NewPortfolioScenario = {
      name: scenario.name,
      description: scenario.description,
      kind: scenario.kind,
      accountScope: scenario.accountScope,
      asOfDate: scenario.asOfDate,
      benchmarkSymbols: scenario.benchmarkSymbols,
      basket: scenario.basket,
      assumptions: scenario.assumptions,
    };
    return await invoke<PortfolioScenario>("update_scenario_entry", {
      scenarioId: scenario.id,
      scenario: payload,
    });
  } catch (error) {
    logger.error("Error updating scenario.");
    throw error;
  }
};

export const deleteScenario = async (scenarioId: string): Promise<void> => {
  try {
    await invoke<void>("delete_scenario_entry", { scenarioId });
  } catch (error) {
    logger.error("Error deleting scenario.");
    throw error;
  }
};
