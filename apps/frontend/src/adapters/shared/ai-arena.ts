import type {
  ArenaAgent,
  ArenaChallenge,
  ArenaLeaderboard,
  ArenaParticipant,
  ArenaPortfolio,
  ArenaRun,
  ArenaTrade,
  CompanyThesis,
  CreateArenaAgentRequest,
  CreateArenaChallengeRequest,
  CreateCompanyThesisRequest,
  RunArenaAgentRequest,
} from "@/lib/types";
import { invoke, logger } from "./platform";

export const getArenaAgents = async (): Promise<ArenaAgent[]> => {
  try {
    return await invoke<ArenaAgent[]>("get_arena_agents");
  } catch (error) {
    logger.error("Error fetching arena agents.");
    throw error;
  }
};

export const createArenaAgent = async (agent: CreateArenaAgentRequest): Promise<ArenaAgent> => {
  try {
    return await invoke<ArenaAgent>("create_arena_agent", { agent });
  } catch (error) {
    logger.error("Error creating arena agent.");
    throw error;
  }
};

export const getArenaChallenges = async (): Promise<ArenaChallenge[]> => {
  try {
    return await invoke<ArenaChallenge[]>("get_arena_challenges");
  } catch (error) {
    logger.error("Error fetching arena challenges.");
    throw error;
  }
};

export const createArenaChallenge = async (
  challenge: CreateArenaChallengeRequest,
): Promise<ArenaChallenge> => {
  try {
    return await invoke<ArenaChallenge>("create_arena_challenge", { challenge });
  } catch (error) {
    logger.error("Error creating arena challenge.");
    throw error;
  }
};

export const joinArenaChallenge = async (
  challengeId: string,
  agentId: string,
): Promise<ArenaParticipant> => {
  try {
    return await invoke<ArenaParticipant>("join_arena_challenge", { challengeId, agentId });
  } catch (error) {
    logger.error("Error joining arena challenge.");
    throw error;
  }
};

export const getArenaParticipants = async (challengeId: string): Promise<ArenaParticipant[]> => {
  try {
    return await invoke<ArenaParticipant[]>("get_arena_participants", { challengeId });
  } catch (error) {
    logger.error("Error fetching arena participants.");
    throw error;
  }
};

export const runArenaAgent = async (request: RunArenaAgentRequest): Promise<ArenaRun> => {
  try {
    return await invoke<ArenaRun>("run_arena_agent", { request });
  } catch (error) {
    logger.error("Error running arena agent.");
    throw error;
  }
};

export const runDueArenaAgents = async (): Promise<ArenaRun[]> => {
  try {
    return await invoke<ArenaRun[]>("run_due_arena_agents");
  } catch (error) {
    logger.error("Error running due arena agents.");
    throw error;
  }
};

export const settleArenaChallenge = async (challengeId: string): Promise<ArenaLeaderboard> => {
  try {
    return await invoke<ArenaLeaderboard>("settle_arena_challenge", { challengeId });
  } catch (error) {
    logger.error("Error settling arena challenge.");
    throw error;
  }
};

export const getArenaLeaderboard = async (challengeId: string): Promise<ArenaLeaderboard> => {
  try {
    return await invoke<ArenaLeaderboard>("get_arena_leaderboard", { challengeId });
  } catch (error) {
    logger.error("Error fetching arena leaderboard.");
    throw error;
  }
};

export const getArenaPortfolio = async (participantId: string): Promise<ArenaPortfolio> => {
  try {
    return await invoke<ArenaPortfolio>("get_arena_portfolio", { participantId });
  } catch (error) {
    logger.error("Error fetching arena portfolio.");
    throw error;
  }
};

export const getArenaRuns = async (challengeId: string): Promise<ArenaRun[]> => {
  try {
    return await invoke<ArenaRun[]>("get_arena_runs", { challengeId });
  } catch (error) {
    logger.error("Error fetching arena runs.");
    throw error;
  }
};

export const getArenaTrades = async (challengeId: string): Promise<ArenaTrade[]> => {
  try {
    return await invoke<ArenaTrade[]>("get_arena_trades", { challengeId });
  } catch (error) {
    logger.error("Error fetching arena trades.");
    throw error;
  }
};

export const createCompanyThesis = async (
  thesis: CreateCompanyThesisRequest,
): Promise<CompanyThesis> => {
  try {
    return await invoke<CompanyThesis>("create_company_thesis", { thesis });
  } catch (error) {
    logger.error("Error creating company thesis.");
    throw error;
  }
};

export const getCompanyTheses = async (input?: {
  symbol?: string;
  challengeId?: string;
  limit?: number;
}): Promise<CompanyThesis[]> => {
  try {
    return await invoke<CompanyThesis[]>("get_company_theses", input ?? {});
  } catch (error) {
    logger.error("Error fetching company theses.");
    throw error;
  }
};
