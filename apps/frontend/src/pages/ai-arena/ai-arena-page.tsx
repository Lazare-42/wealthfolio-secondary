import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";

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
import { Badge } from "@wealthfolio/ui/components/ui/badge";
import { Button } from "@wealthfolio/ui/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@wealthfolio/ui/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@wealthfolio/ui/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@wealthfolio/ui/components/ui/tabs";
import { Icons, Skeleton } from "@wealthfolio/ui";

import { AgentForm } from "./components/agent-form";
import { EmptyRow } from "./components/empty-row";
import { OnboardingChecklist } from "./components/onboarding-checklist";
import { decimal, formatMoney, formatPct, statusVariant } from "./components/formatters";
import { LeaderboardTable } from "./components/leaderboard-table";
import { Metric } from "./components/metric";
import { ParticipantButton } from "./components/participant-button";
import { ThesisForm } from "./components/thesis-form";

export default function AiArenaPage() {
  const navigate = useNavigate();
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

  const isLoading = agentsLoading || challengesLoading;

  if (isLoading) {
    return (
      <div className="space-y-4 p-4 lg:p-6">
        <Skeleton className="h-12" />
        <Skeleton className="h-96" />
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col gap-4 p-4 lg:p-6">
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <Icons.Brain className="text-primary h-5 w-5" />
            <h1 className="text-xl font-semibold tracking-normal">AI Arena</h1>
          </div>
          <div className="text-muted-foreground mt-1 flex flex-wrap gap-x-3 gap-y-1 text-sm">
            <span>{agents.length} agents</span>
            <span>{challenges.length} challenges</span>
            <span>{trades.length} paper trades</span>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button size="sm" onClick={() => navigate("/ai-arena/challenges/new")}>
            <Icons.PlusCircle className="mr-2 h-4 w-4" />
            New challenge
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => mutations.runDueMutation.mutate()}
            disabled={mutations.runDueMutation.isPending}
          >
            {mutations.runDueMutation.isPending ? (
              <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Icons.Clock className="mr-2 h-4 w-4" />
            )}
            Run due
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() =>
              selectedChallengeId && mutations.settleChallengeMutation.mutate(selectedChallengeId)
            }
            disabled={!selectedChallengeId || mutations.settleChallengeMutation.isPending}
          >
            <Icons.CheckCircle className="mr-2 h-4 w-4" />
            Settle
          </Button>
        </div>
      </header>

      <OnboardingChecklist
        hasProvider={enabledProviders.some(
          (provider) => provider.type === "local" || provider.hasApiKey,
        )}
        hasAgent={agents.length > 0}
        hasChallenge={challenges.length > 0}
        hasParticipant={hasJoinedOnce || participants.length > 0}
        hasRun={hasRunOnce || runs.length > 0}
      />

      <div className="grid min-h-0 flex-1 gap-4 xl:grid-cols-[360px_minmax(0,1fr)_420px]">
        <section className="border-border bg-card min-h-0 overflow-auto rounded-md border">
          <div className="border-border border-b p-4">
            <h2 className="text-sm font-semibold">Setup</h2>
          </div>
          <div className="space-y-6 p-4">
            <div id="arena-agent-form">
              <AgentForm
                enabledProviders={enabledProviders}
                createAgentMutation={mutations.createAgentMutation}
              />
            </div>
            <div className="border-border border-t pt-5">
              <h3 className="mb-3 text-sm font-medium">Challenge</h3>
              <Button className="w-full" onClick={() => navigate("/ai-arena/challenges/new")}>
                <Icons.PlusCircle className="mr-2 h-4 w-4" />
                New challenge
              </Button>
              <p className="text-muted-foreground mt-2 text-xs">
                Design a challenge — with AI help — on its own page.
              </p>
            </div>
          </div>
        </section>

        <main className="border-border bg-card min-h-0 overflow-hidden rounded-md border">
          <div className="border-border flex flex-wrap items-center justify-between gap-3 border-b p-4">
            <div className="min-w-0">
              <h2 className="truncate text-sm font-semibold">
                {selectedChallenge?.name ?? "No challenge"}
              </h2>
              {selectedChallenge ? (
                <div className="text-muted-foreground mt-1 flex flex-wrap gap-x-3 gap-y-1 text-xs">
                  <span>{selectedChallenge.market}</span>
                  <span>{formatMoney(selectedChallenge.initialCash)}</span>
                  <span>max {decimal.format(selectedChallenge.maxPositionPct)}%</span>
                  <span>{selectedChallenge.universe.join(", ") || "open universe"}</span>
                </div>
              ) : (
                <p className="text-muted-foreground mt-1 text-xs">
                  Create a challenge in Setup to start a match — agents trade paper money against
                  its universe.
                </p>
              )}
            </div>
            <Select
              value={selectedChallengeId}
              onValueChange={(challengeId) => {
                setSelectedChallengeId(challengeId);
                setSelectedParticipantId("");
              }}
            >
              <SelectTrigger className="min-w-52">
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
          </div>

          <div className="grid min-h-0 gap-0 lg:grid-cols-[260px_minmax(0,1fr)]">
            <aside
              id="arena-participants"
              className="border-border min-h-0 border-b p-4 lg:border-b-0 lg:border-r"
            >
              <div className="mb-3 flex items-center justify-between">
                <h3 className="text-sm font-medium">Participants</h3>
                <Badge variant="outline">{participants.length}</Badge>
              </div>
              <div className="space-y-2">
                {participants.map((participant) => {
                  const agent = agentsById.get(participant.agentId);
                  return (
                    <ParticipantButton
                      key={participant.id}
                      participant={participant}
                      agent={agent}
                      selected={participant.id === selectedParticipantId}
                      onSelect={() => setSelectedParticipantId(participant.id)}
                      onRun={() =>
                        mutations.runAgentMutation.mutate({
                          challengeId: participant.challengeId,
                          agentId: participant.agentId,
                          runType: "manual",
                        })
                      }
                      running={mutations.runAgentMutation.isPending}
                    />
                  );
                })}
                {participants.length === 0 && (
                  <div className="text-muted-foreground rounded-md border border-dashed p-3 text-sm">
                    {!selectedChallengeId
                      ? "Select or create a challenge to add participants."
                      : agents.length === 0
                        ? "No agents yet. Add one in Setup, then join it here."
                        : "Join an agent below to enter it in this match."}
                  </div>
                )}
              </div>

              {selectedChallengeId && availableAgents.length > 0 && (
                <div className="border-border mt-4 border-t pt-4">
                  <h3 className="mb-2 text-sm font-medium">Join</h3>
                  <div className="space-y-2">
                    {availableAgents.map((agent) => (
                      <Button
                        key={agent.id}
                        variant="outline"
                        size="sm"
                        className="w-full justify-start"
                        onClick={() =>
                          mutations.joinChallengeMutation.mutate({
                            challengeId: selectedChallengeId,
                            agentId: agent.id,
                          })
                        }
                      >
                        <Icons.Plus className="mr-2 h-4 w-4" />
                        {agent.name}
                      </Button>
                    ))}
                  </div>
                </div>
              )}
            </aside>

            <Tabs defaultValue="leaderboard" className="min-h-0 p-4">
              <TabsList className="mb-4">
                <TabsTrigger value="leaderboard">Leaderboard</TabsTrigger>
                <TabsTrigger value="trades">Trades</TabsTrigger>
                <TabsTrigger value="runs">Runs</TabsTrigger>
              </TabsList>
              <TabsContent value="leaderboard" className="m-0">
                <LeaderboardTable
                  entries={leaderboard?.entries ?? []}
                  participants={participants}
                  agentsById={agentsById}
                  onSelectParticipant={setSelectedParticipantId}
                />
              </TabsContent>
              <TabsContent value="trades" className="m-0">
                <div className="max-h-[560px] overflow-auto rounded-md border">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Symbol</TableHead>
                        <TableHead>Side</TableHead>
                        <TableHead className="text-right">Notional</TableHead>
                        <TableHead>Status</TableHead>
                        <TableHead>Reason</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {trades.map((trade) => (
                        <TableRow key={trade.id}>
                          <TableCell className="font-medium">{trade.symbol}</TableCell>
                          <TableCell>{trade.side.toUpperCase()}</TableCell>
                          <TableCell className="text-right">
                            {formatMoney(trade.notional)}
                          </TableCell>
                          <TableCell>
                            <Badge variant={statusVariant(trade.status)}>{trade.status}</Badge>
                          </TableCell>
                          <TableCell className="text-muted-foreground max-w-80 truncate">
                            {trade.rejectionReason ?? trade.rationale ?? ""}
                          </TableCell>
                        </TableRow>
                      ))}
                      {trades.length === 0 && (
                        <EmptyRow
                          colSpan={5}
                          label="Paper trades appear after an agent runs. Pick a participant and Run, or use Run due."
                        />
                      )}
                    </TableBody>
                  </Table>
                </div>
              </TabsContent>
              <TabsContent value="runs" className="m-0">
                <div className="max-h-[560px] overflow-auto rounded-md border">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Agent</TableHead>
                        <TableHead>Type</TableHead>
                        <TableHead>Status</TableHead>
                        <TableHead>Started</TableHead>
                        <TableHead>Error</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {runs.map((run) => (
                        <TableRow key={run.id}>
                          <TableCell>{agentsById.get(run.agentId)?.name ?? run.agentId}</TableCell>
                          <TableCell>{run.runType}</TableCell>
                          <TableCell>
                            <Badge variant={statusVariant(run.status)}>{run.status}</Badge>
                          </TableCell>
                          <TableCell className="text-muted-foreground">
                            {new Date(run.startedAt).toLocaleString()}
                          </TableCell>
                          <TableCell className="text-muted-foreground max-w-80 truncate">
                            {run.error ?? ""}
                          </TableCell>
                        </TableRow>
                      ))}
                      {runs.length === 0 && (
                        <EmptyRow
                          colSpan={5}
                          label="Each agent decision is logged here. Click Run on a participant to start one."
                        />
                      )}
                    </TableBody>
                  </Table>
                </div>
              </TabsContent>
            </Tabs>
          </div>
        </main>

        <section className="border-border bg-card min-h-0 overflow-auto rounded-md border">
          <div className="border-border border-b p-4">
            <h2 className="text-sm font-semibold">Portfolio</h2>
          </div>
          <div className="space-y-5 p-4">
            {portfolio ? (
              <>
                <div className="grid grid-cols-2 gap-3">
                  <Metric label="Value" value={formatMoney(portfolio.totalValue)} />
                  <Metric label="Return" value={formatPct(portfolio.returnPct)} />
                  <Metric label="Cash" value={formatMoney(portfolio.cash)} />
                  <Metric label="Drawdown" value={formatPct(-Math.abs(portfolio.maxDrawdownPct))} />
                </div>
                <div>
                  <h3 className="mb-2 text-sm font-medium">Positions</h3>
                  <div className="rounded-md border">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>Symbol</TableHead>
                          <TableHead className="text-right">Qty</TableHead>
                          <TableHead className="text-right">Value</TableHead>
                          <TableHead className="text-right">P/L</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {portfolio.positions.map((position) => (
                          <TableRow key={position.symbol}>
                            <TableCell className="font-medium">{position.symbol}</TableCell>
                            <TableCell className="text-right">
                              {decimal.format(position.quantity)}
                            </TableCell>
                            <TableCell className="text-right">
                              {formatMoney(position.marketValue)}
                            </TableCell>
                            <TableCell className="text-right">
                              {formatPct(position.unrealizedPnlPct)}
                            </TableCell>
                          </TableRow>
                        ))}
                        {portfolio.positions.length === 0 && (
                          <EmptyRow colSpan={4} label="No positions" />
                        )}
                      </TableBody>
                    </Table>
                  </div>
                </div>
              </>
            ) : (
              <div className="text-muted-foreground rounded-md border border-dashed p-4 text-sm">
                {participants.length === 0
                  ? "No participants yet — join an agent to a challenge to build a paper portfolio."
                  : "Select a participant to view its cash, positions, return, and drawdown."}
              </div>
            )}

            <ThesisForm
              createThesisMutation={mutations.createThesisMutation}
              challengeId={selectedChallengeId || undefined}
              agentId={selectedParticipant?.agentId}
            />

            <div className="border-border border-t pt-5">
              <h3 className="mb-3 text-sm font-medium">Recent theses</h3>
              <div className="space-y-2">
                {theses.map((thesis) => (
                  <div key={thesis.id} className="rounded-md border p-3">
                    <div className="mb-1 flex items-center justify-between gap-2">
                      <div className="flex items-center gap-2">
                        <span className="font-medium">{thesis.symbol}</span>
                        {thesis.rating && <Badge variant="outline">{thesis.rating}</Badge>}
                      </div>
                      {thesis.confidence !== null && thesis.confidence !== undefined && (
                        <span className="text-muted-foreground text-xs">
                          {decimal.format(thesis.confidence)}
                        </span>
                      )}
                    </div>
                    <p className="text-muted-foreground line-clamp-3 text-sm">{thesis.thesis}</p>
                  </div>
                ))}
                {theses.length === 0 && (
                  <div className="text-muted-foreground rounded-md border border-dashed p-3 text-sm">
                    Save a thesis above, or run an agent — model decisions are stored here with
                    rating and confidence.
                  </div>
                )}
              </div>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}
