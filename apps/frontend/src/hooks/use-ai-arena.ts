import {
  createArenaAgent,
  createArenaChallenge,
  createCompanyThesis,
  generateArenaChallengeSpec,
  getArenaAgents,
  getArenaChallenges,
  getArenaLeaderboard,
  getArenaParticipants,
  getArenaPortfolio,
  getArenaRuns,
  getArenaTrades,
  getCompanyTheses,
  joinArenaChallenge,
  runArenaAgent,
  runDueArenaAgents,
  settleArenaChallenge,
} from "@/adapters";
import { QueryKeys } from "@/lib/query-keys";
import type {
  CreateArenaAgentRequest,
  CreateArenaChallengeRequest,
  CreateCompanyThesisRequest,
  RunArenaAgentRequest,
} from "@/lib/types";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

export function useArenaAgents() {
  return useQuery({
    queryKey: [QueryKeys.AI_ARENA_AGENTS],
    queryFn: getArenaAgents,
  });
}

export function useArenaChallenges() {
  return useQuery({
    queryKey: [QueryKeys.AI_ARENA_CHALLENGES],
    queryFn: getArenaChallenges,
  });
}

export function useArenaParticipants(challengeId?: string) {
  return useQuery({
    queryKey: QueryKeys.aiArenaParticipants(challengeId ?? "none"),
    queryFn: () => getArenaParticipants(challengeId ?? ""),
    enabled: Boolean(challengeId),
  });
}

export function useArenaLeaderboard(challengeId?: string) {
  return useQuery({
    queryKey: QueryKeys.aiArenaLeaderboard(challengeId ?? "none"),
    queryFn: () => getArenaLeaderboard(challengeId ?? ""),
    enabled: Boolean(challengeId),
  });
}

export function useArenaPortfolio(participantId?: string) {
  return useQuery({
    queryKey: QueryKeys.aiArenaPortfolio(participantId ?? "none"),
    queryFn: () => getArenaPortfolio(participantId ?? ""),
    enabled: Boolean(participantId),
  });
}

export function useArenaRuns(challengeId?: string) {
  return useQuery({
    queryKey: QueryKeys.aiArenaRuns(challengeId ?? "none"),
    queryFn: () => getArenaRuns(challengeId ?? ""),
    enabled: Boolean(challengeId),
  });
}

export function useArenaTrades(challengeId?: string) {
  return useQuery({
    queryKey: QueryKeys.aiArenaTrades(challengeId ?? "none"),
    queryFn: () => getArenaTrades(challengeId ?? ""),
    enabled: Boolean(challengeId),
  });
}

export function useCompanyTheses(input?: {
  symbol?: string;
  challengeId?: string;
  limit?: number;
}) {
  return useQuery({
    queryKey: QueryKeys.aiArenaTheses(input?.symbol, input?.challengeId),
    queryFn: () => getCompanyTheses(input),
  });
}

export function useAiArenaMutations() {
  const queryClient = useQueryClient();

  const invalidateChallenge = (challengeId?: string) => {
    queryClient.invalidateQueries({ queryKey: [QueryKeys.AI_ARENA_CHALLENGES] });
    if (!challengeId) return;
    queryClient.invalidateQueries({ queryKey: QueryKeys.aiArenaParticipants(challengeId) });
    queryClient.invalidateQueries({ queryKey: QueryKeys.aiArenaLeaderboard(challengeId) });
    queryClient.invalidateQueries({ queryKey: QueryKeys.aiArenaRuns(challengeId) });
    queryClient.invalidateQueries({ queryKey: QueryKeys.aiArenaTrades(challengeId) });
    queryClient.invalidateQueries({ queryKey: [QueryKeys.AI_ARENA_PORTFOLIO] });
    queryClient.invalidateQueries({ queryKey: [QueryKeys.AI_ARENA_THESES] });
  };

  const createAgentMutation = useMutation({
    mutationFn: (agent: CreateArenaAgentRequest) => createArenaAgent(agent),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.AI_ARENA_AGENTS] });
      toast.success("Agent created.");
    },
    onError: () => toast.error("Failed to create agent."),
  });

  const createChallengeMutation = useMutation({
    mutationFn: (challenge: CreateArenaChallengeRequest) => createArenaChallenge(challenge),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.AI_ARENA_CHALLENGES] });
      toast.success("Challenge created.");
    },
    onError: () => toast.error("Failed to create challenge."),
  });

  // Pure generation call - no query-key invalidation needed.
  const generateChallengeSpecMutation = useMutation({
    mutationFn: (theme: string) => generateArenaChallengeSpec(theme),
    onError: (error) =>
      toast.error(
        error instanceof Error && error.message
          ? error.message
          : "Failed to generate challenge with AI.",
      ),
  });

  const joinChallengeMutation = useMutation({
    mutationFn: (input: { challengeId: string; agentId: string }) =>
      joinArenaChallenge(input.challengeId, input.agentId),
    onSuccess: (participant) => {
      invalidateChallenge(participant.challengeId);
      toast.success("Agent joined challenge.");
    },
    onError: () => toast.error("Failed to join challenge."),
  });

  const runAgentMutation = useMutation({
    mutationFn: (request: RunArenaAgentRequest) => runArenaAgent(request),
    onSuccess: (run) => {
      invalidateChallenge(run.challengeId);
      toast.success("Agent run completed.");
    },
    onError: () => toast.error("Agent run failed."),
  });

  const runDueMutation = useMutation({
    mutationFn: () => runDueArenaAgents(),
    onSuccess: (runs) => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.AI_ARENA_CHALLENGES] });
      runs.forEach((run) => invalidateChallenge(run.challengeId));
      toast.success("Scheduled agents checked.");
    },
    onError: () => toast.error("Failed to run scheduled agents."),
  });

  const settleChallengeMutation = useMutation({
    mutationFn: (challengeId: string) => settleArenaChallenge(challengeId),
    onSuccess: (leaderboard) => {
      invalidateChallenge(leaderboard.challenge.id);
      toast.success("Challenge settled.");
    },
    onError: () => toast.error("Failed to settle challenge."),
  });

  const createThesisMutation = useMutation({
    mutationFn: (thesis: CreateCompanyThesisRequest) => createCompanyThesis(thesis),
    onSuccess: (thesis) => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.AI_ARENA_THESES] });
      if (thesis.challengeId) invalidateChallenge(thesis.challengeId);
      toast.success("Thesis saved.");
    },
    onError: () => toast.error("Failed to save thesis."),
  });

  return {
    createAgentMutation,
    createChallengeMutation,
    generateChallengeSpecMutation,
    joinChallengeMutation,
    runAgentMutation,
    runDueMutation,
    settleChallengeMutation,
    createThesisMutation,
  };
}
