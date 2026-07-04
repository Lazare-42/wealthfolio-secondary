import { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";

import { SwipablePage, type SwipablePageView } from "@/components/page";
import { useAiProviders } from "@/features/ai-assistant/hooks/use-ai-providers";
import { usePersistentState } from "@/hooks/use-persistent-state";
import {
  useAiArenaMutations,
  useArenaAgents,
  useArenaChallenges,
  useArenaLeaderboard,
  useArenaParticipants,
  useArenaPortfolio,
  useArenaRuns,
  useArenaTrades,
  useCompanyTheses,
} from "@/hooks/use-ai-arena";
import { Button } from "@wealthfolio/ui/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@wealthfolio/ui/components/ui/select";
import { Icons, Skeleton } from "@wealthfolio/ui";

import { ArenaActions } from "./components/arena-actions";
import { ArenaTab } from "./components/arena-tab";
import { OnboardingChecklist } from "./components/onboarding-checklist";
import { PortfolioTab } from "./components/portfolio-tab";
import { SetupTab } from "./components/setup-tab";

const TAB_PERSIST_KEY = "ai-arena-page-tab";

export default function AiArenaPage() {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const { data: providersResponse } = useAiProviders();
  const { data: agents = [], isLoading: agentsLoading } = useArenaAgents();
  const { data: challenges = [], isLoading: challengesLoading } = useArenaChallenges();
  const mutations = useAiArenaMutations();

  const [selectedChallengeId, setSelectedChallengeId] = useState<string>("");
  const [selectedParticipantId, setSelectedParticipantId] = useState<string>("");

  const enabledProviders = useMemo(
    () => (providersResponse?.providers ?? []).filter((provider) => provider.enabled),
    [providersResponse?.providers],
  );
  const selectedChallenge = challenges.find((challenge) => challenge.id === selectedChallengeId);

  const { data: participants = [] } = useArenaParticipants(selectedChallengeId);
  const { data: leaderboard } = useArenaLeaderboard(selectedChallengeId);
  const { data: portfolio } = useArenaPortfolio(selectedParticipantId);
  const { data: runs = [] } = useArenaRuns(selectedChallengeId);
  const { data: trades = [] } = useArenaTrades(selectedChallengeId);
  const { data: theses = [] } = useCompanyTheses({
    challengeId: selectedChallengeId || undefined,
    limit: 20,
  });

  const agentsById = useMemo(
    () => new Map(agents.map((agent) => [agent.id, agent] as const)),
    [agents],
  );
  const participantsByAgentId = useMemo(
    () => new Map(participants.map((participant) => [participant.agentId, participant] as const)),
    [participants],
  );
  const availableAgents = agents.filter((agent) => !participantsByAgentId.has(agent.id));
  const selectedParticipant = participants.find(
    (participant) => participant.id === selectedParticipantId,
  );

  useEffect(() => {
    if (!selectedChallengeId && challenges.length > 0) {
      setSelectedChallengeId(challenges[0].id);
      return;
    }
    if (
      selectedChallengeId &&
      !challenges.some((challenge) => challenge.id === selectedChallengeId)
    ) {
      setSelectedChallengeId(challenges[0]?.id ?? "");
    }
  }, [challenges, selectedChallengeId]);

  // Deep-link (?challenge=<id>, e.g. from the create page): select it once
  // loaded, then drop the param so later selection changes aren't pinned.
  // Runs after the auto-select effect so the deep-link wins on mount.
  const challengeParam = searchParams.get("challenge");
  useEffect(() => {
    if (!challengeParam) return;
    if (!challenges.some((challenge) => challenge.id === challengeParam)) return;
    setSelectedChallengeId(challengeParam);
    setSelectedParticipantId("");
    setSearchParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        next.delete("challenge");
        return next;
      },
      { replace: true },
    );
  }, [challengeParam, challenges, setSearchParams]);

  // Onboarding steps 4 (join) and 5 (run) derive from per-selected-challenge
  // queries and would revert when switching to an empty challenge — latch
  // them once observed done.
  const [hasJoinedOnce, setHasJoinedOnce] = usePersistentState("ai-arena-onboarding-joined", false);
  const [hasRunOnce, setHasRunOnce] = usePersistentState("ai-arena-onboarding-run", false);
  useEffect(() => {
    if (participants.length > 0) setHasJoinedOnce(true);
  }, [participants.length, setHasJoinedOnce]);
  useEffect(() => {
    if (runs.length > 0) setHasRunOnce(true);
  }, [runs.length, setHasRunOnce]);

  useEffect(() => {
    if (!selectedParticipantId && participants.length > 0) {
      setSelectedParticipantId(participants[0].id);
      return;
    }
    if (
      selectedParticipantId &&
      !participants.some((participant) => participant.id === selectedParticipantId)
    ) {
      setSelectedParticipantId(participants[0]?.id ?? "");
    }
  }, [participants, selectedParticipantId]);

  // Switch tab (the URL is SwipablePage's source of truth) and optionally
  // scroll to an anchor once the newly selected view has rendered.
  const goToTab = useCallback(
    (tab: string, anchorId?: string) => {
      setSearchParams(
        (prev) => {
          const next = new URLSearchParams(prev);
          next.set("tab", tab);
          return next;
        },
        { replace: true },
      );
      try {
        window.localStorage.setItem(TAB_PERSIST_KEY, JSON.stringify(tab));
      } catch {
        // Persistence is best-effort.
      }
      if (anchorId) {
        requestAnimationFrame(() => {
          requestAnimationFrame(() => {
            document
              .getElementById(anchorId)
              ?.scrollIntoView({ behavior: "smooth", block: "start" });
          });
        });
      }
    },
    [setSearchParams],
  );

  const hasProvider = enabledProviders.some(
    (provider) => provider.type === "local" || provider.hasApiKey,
  );

  const isLoading = agentsLoading || challengesLoading;

  if (isLoading) {
    return (
      <div className="space-y-4 p-4 lg:p-6">
        <Skeleton className="h-12" />
        <Skeleton className="h-96" />
      </div>
    );
  }

  // Computed fallback tab — the URL `?tab=` param and the persisted tab
  // (handled inside SwipablePage) both take precedence over this.
  const defaultTab =
    !hasProvider || agents.length === 0 || challenges.length === 0 ? "setup" : "arena";

  const challengeSelect = (
    <Select
      value={selectedChallengeId}
      onValueChange={(challengeId) => {
        setSelectedChallengeId(challengeId);
        setSelectedParticipantId("");
      }}
    >
      <SelectTrigger className="h-8 w-40 text-sm sm:w-48">
        <SelectValue placeholder="No challenges" />
      </SelectTrigger>
      <SelectContent>
        {challenges.map((challenge) => (
          <SelectItem key={challenge.id} value={challenge.id}>
            {challenge.name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );

  const participantSelect = (
    <Select value={selectedParticipantId} onValueChange={setSelectedParticipantId}>
      <SelectTrigger className="h-8 w-40 text-sm sm:w-48">
        <SelectValue placeholder="No participants" />
      </SelectTrigger>
      <SelectContent>
        {participants.map((participant) => (
          <SelectItem key={participant.id} value={participant.id}>
            {agentsById.get(participant.agentId)?.name ?? participant.agentId}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );

  const views: SwipablePageView[] = [
    {
      value: "setup",
      label: "Setup",
      icon: Icons.Settings2,
      content: (
        <SetupTab
          enabledProviders={enabledProviders}
          agents={agents}
          challenges={challenges}
          createAgentMutation={mutations.createAgentMutation}
          onOpenChallenge={(challengeId) => {
            setSelectedChallengeId(challengeId);
            setSelectedParticipantId("");
            goToTab("arena");
          }}
          onNewChallenge={() => navigate("/ai-arena/challenges/new")}
        />
      ),
      actions: (
        <Button size="sm" onClick={() => navigate("/ai-arena/challenges/new")}>
          <Icons.PlusCircle className="mr-2 h-4 w-4" />
          New challenge
        </Button>
      ),
    },
    {
      value: "arena",
      label: "Arena",
      icon: Icons.Brain,
      content: (
        <ArenaTab
          challenge={selectedChallenge}
          agents={agents}
          agentsById={agentsById}
          participants={participants}
          availableAgents={availableAgents}
          selectedParticipantId={selectedParticipantId}
          leaderboardEntries={leaderboard?.entries ?? []}
          trades={trades}
          runs={runs}
          onSelectParticipant={setSelectedParticipantId}
          onOpenParticipant={(participantId) => {
            setSelectedParticipantId(participantId);
            goToTab("portfolio");
          }}
          onJoin={(agentId) =>
            selectedChallengeId &&
            mutations.joinChallengeMutation.mutate({
              challengeId: selectedChallengeId,
              agentId,
            })
          }
          onRunParticipant={(participant) =>
            mutations.runAgentMutation.mutate({
              challengeId: participant.challengeId,
              agentId: participant.agentId,
              runType: "manual",
            })
          }
          isRunningParticipant={(participant) =>
            mutations.runAgentMutation.isPending &&
            mutations.runAgentMutation.variables?.challengeId === participant.challengeId &&
            mutations.runAgentMutation.variables?.agentId === participant.agentId
          }
          onGoToSetup={() => goToTab("setup", "arena-agent-form")}
        />
      ),
      actions: (
        <div className="flex min-w-0 flex-wrap items-center justify-end gap-2">
          {challengeSelect}
          <ArenaActions
            selectedChallengeId={selectedChallengeId}
            runDueMutation={mutations.runDueMutation}
            settleChallengeMutation={mutations.settleChallengeMutation}
          />
        </div>
      ),
    },
    {
      value: "portfolio",
      label: "Portfolio",
      icon: Icons.Wallet,
      content: (
        <PortfolioTab
          portfolio={portfolio}
          hasParticipants={participants.length > 0}
          theses={theses}
          createThesisMutation={mutations.createThesisMutation}
          challengeId={selectedChallengeId || undefined}
          agentId={selectedParticipant?.agentId}
        />
      ),
      actions: (
        <div className="flex min-w-0 flex-wrap items-center justify-end gap-2">
          {challengeSelect}
          {participantSelect}
        </div>
      ),
    },
  ];

  return (
    <SwipablePage
      title="AI Arena"
      views={views}
      defaultView={defaultTab}
      persistKey={TAB_PERSIST_KEY}
      banner={
        <OnboardingChecklist
          hasProvider={hasProvider}
          hasAgent={agents.length > 0}
          hasChallenge={challenges.length > 0}
          hasParticipant={hasJoinedOnce || participants.length > 0}
          hasRun={hasRunOnce || runs.length > 0}
          goTo={goToTab}
        />
      }
    />
  );
}
