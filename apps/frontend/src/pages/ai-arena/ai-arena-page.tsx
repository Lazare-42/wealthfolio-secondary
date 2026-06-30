import { useEffect, useMemo, useState } from "react";

import { useAiProviders } from "@/features/ai-assistant/hooks/use-ai-providers";
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
import type {
  ArenaAgent,
  ArenaLeaderboardEntry,
  ArenaParticipant,
  CreateArenaAgentRequest,
  CreateArenaChallengeRequest,
  CreateCompanyThesisRequest,
} from "@/lib/types";
import { Badge } from "@wealthfolio/ui/components/ui/badge";
import { Button } from "@wealthfolio/ui/components/ui/button";
import { Input } from "@wealthfolio/ui/components/ui/input";
import { Label } from "@wealthfolio/ui/components/ui/label";
import { Switch } from "@wealthfolio/ui/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@wealthfolio/ui/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@wealthfolio/ui/components/ui/tabs";
import { Textarea } from "@wealthfolio/ui/components/ui/textarea";
import { Icons, Skeleton } from "@wealthfolio/ui";

const money = new Intl.NumberFormat(undefined, {
  style: "currency",
  currency: "USD",
  maximumFractionDigits: 0,
});

const decimal = new Intl.NumberFormat(undefined, {
  maximumFractionDigits: 2,
});

function formatMoney(value: number) {
  return money.format(Number.isFinite(value) ? value : 0);
}

function formatPct(value: number) {
  const safe = Number.isFinite(value) ? value : 0;
  return `${safe >= 0 ? "+" : ""}${decimal.format(safe)}%`;
}

function splitSymbols(value: string): string[] {
  return value
    .split(/[\s,]+/)
    .map((symbol) => symbol.trim().toUpperCase())
    .filter(Boolean);
}

function splitTextList(value: string): string[] {
  return value
    .split(/\n|,/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function numberOr(value: string, fallback: number): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function statusVariant(
  status: string,
): "default" | "secondary" | "success" | "warning" | "outline" {
  if (status === "active" || status === "completed" || status === "executed") return "success";
  if (status === "running") return "warning";
  if (status === "settled") return "secondary";
  return "outline";
}

function latestModelId(provider?: {
  selectedModel?: string;
  defaultModel: string;
  models: { id: string }[];
}) {
  return provider?.selectedModel ?? provider?.defaultModel ?? provider?.models[0]?.id ?? "";
}

interface AgentFormState {
  name: string;
  providerId: string;
  modelId: string;
  persona: string;
  enabled: boolean;
  scheduleEnabled: boolean;
}

interface ChallengeFormState {
  name: string;
  description: string;
  market: string;
  universe: string;
  initialCash: string;
  maxPositionPct: string;
  maxDrawdownPct: string;
  runCadence: string;
  scheduledTimeLocal: string;
}

interface ThesisFormState {
  symbol: string;
  rating: string;
  confidence: string;
  horizon: string;
  thesis: string;
  risks: string;
  catalysts: string;
}

const defaultAgentForm: AgentFormState = {
  name: "",
  providerId: "",
  modelId: "",
  persona:
    "Long-only public equity analyst. Prefer liquid stocks and ETFs. Keep position risk explicit.",
  enabled: true,
  scheduleEnabled: false,
};

const defaultChallengeForm: ChallengeFormState = {
  name: "US Stock Arena",
  description: "",
  market: "us-stock",
  universe: "AAPL, MSFT, NVDA, SPY, QQQ",
  initialCash: "100000",
  maxPositionPct: "50",
  maxDrawdownPct: "25",
  runCadence: "daily",
  scheduledTimeLocal: "09:30",
};

const defaultThesisForm: ThesisFormState = {
  symbol: "",
  rating: "",
  confidence: "",
  horizon: "3-6 months",
  thesis: "",
  risks: "",
  catalysts: "",
};

export default function AiArenaPage() {
  const { data: providersResponse } = useAiProviders();
  const { data: agents = [], isLoading: agentsLoading } = useArenaAgents();
  const { data: challenges = [], isLoading: challengesLoading } = useArenaChallenges();
  const mutations = useAiArenaMutations();

  const [selectedChallengeId, setSelectedChallengeId] = useState<string>("");
  const [selectedParticipantId, setSelectedParticipantId] = useState<string>("");
  const [agentForm, setAgentForm] = useState<AgentFormState>(defaultAgentForm);
  const [challengeForm, setChallengeForm] = useState<ChallengeFormState>(defaultChallengeForm);
  const [thesisForm, setThesisForm] = useState<ThesisFormState>(defaultThesisForm);

  const enabledProviders = useMemo(
    () => (providersResponse?.providers ?? []).filter((provider) => provider.enabled),
    [providersResponse?.providers],
  );
  const selectedProvider = enabledProviders.find(
    (provider) => provider.id === agentForm.providerId,
  );
  const providerModels = selectedProvider?.models ?? [];
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
    if (!agentForm.providerId && enabledProviders.length > 0) {
      const first = enabledProviders[0];
      setAgentForm((current) => ({
        ...current,
        providerId: first.id,
        modelId: latestModelId(first),
      }));
    }
  }, [agentForm.providerId, enabledProviders]);

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

  const createAgent = () => {
    const payload: CreateArenaAgentRequest = {
      name: agentForm.name.trim(),
      providerId: agentForm.providerId,
      modelId: agentForm.modelId,
      persona: agentForm.persona.trim() || null,
      enabled: agentForm.enabled,
      scheduleEnabled: agentForm.scheduleEnabled,
    };
    mutations.createAgentMutation.mutate(payload, {
      onSuccess: () =>
        setAgentForm((current) => ({
          ...defaultAgentForm,
          providerId: current.providerId,
          modelId: current.modelId,
        })),
    });
  };

  const createChallenge = () => {
    const payload: CreateArenaChallengeRequest = {
      name: challengeForm.name.trim(),
      description: challengeForm.description.trim() || null,
      market: challengeForm.market.trim() || "us-stock",
      scoringMethod: "riskAdjusted",
      initialCash: numberOr(challengeForm.initialCash, 100000),
      maxPositionPct: numberOr(challengeForm.maxPositionPct, 50),
      maxDrawdownPct: numberOr(challengeForm.maxDrawdownPct, 25),
      runCadence: challengeForm.runCadence.trim() || "daily",
      scheduledTimeLocal: challengeForm.scheduledTimeLocal.trim() || null,
      universe: splitSymbols(challengeForm.universe),
    };
    mutations.createChallengeMutation.mutate(payload, {
      onSuccess: (challenge) => setSelectedChallengeId(challenge.id),
    });
  };

  const createThesis = () => {
    const payload: CreateCompanyThesisRequest = {
      symbol: thesisForm.symbol.trim().toUpperCase(),
      challengeId: selectedChallengeId || null,
      agentId: selectedParticipant?.agentId ?? null,
      rating: thesisForm.rating.trim() || null,
      confidence: thesisForm.confidence.trim() ? numberOr(thesisForm.confidence, 0) : null,
      horizon: thesisForm.horizon.trim() || null,
      thesis: thesisForm.thesis.trim(),
      risks: splitTextList(thesisForm.risks),
      catalysts: splitTextList(thesisForm.catalysts),
    };
    mutations.createThesisMutation.mutate(payload, {
      onSuccess: () => setThesisForm(defaultThesisForm),
    });
  };

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

      <div className="grid min-h-0 flex-1 gap-4 xl:grid-cols-[360px_minmax(0,1fr)_420px]">
        <section className="border-border bg-card min-h-0 overflow-auto rounded-md border">
          <div className="border-border border-b p-4">
            <h2 className="text-sm font-semibold">Setup</h2>
          </div>
          <div className="space-y-6 p-4">
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <h3 className="text-sm font-medium">Agent</h3>
                <Badge variant="outline">{enabledProviders.length} providers</Badge>
              </div>
              <Field label="Name">
                <Input
                  value={agentForm.name}
                  onChange={(event) => setAgentForm({ ...agentForm, name: event.target.value })}
                  placeholder="OpenAI momentum"
                />
              </Field>
              <div className="grid grid-cols-2 gap-2">
                <Field label="Provider">
                  <select
                    className="border-input bg-background h-9 w-full rounded-md border px-3 text-sm"
                    value={agentForm.providerId}
                    onChange={(event) => {
                      const provider = enabledProviders.find(
                        (item) => item.id === event.target.value,
                      );
                      setAgentForm({
                        ...agentForm,
                        providerId: event.target.value,
                        modelId: latestModelId(provider),
                      });
                    }}
                  >
                    {enabledProviders.length === 0 && <option value="">None</option>}
                    {enabledProviders.map((provider) => (
                      <option key={provider.id} value={provider.id}>
                        {provider.name}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="Model">
                  <select
                    className="border-input bg-background h-9 w-full rounded-md border px-3 text-sm"
                    value={agentForm.modelId}
                    onChange={(event) =>
                      setAgentForm({ ...agentForm, modelId: event.target.value })
                    }
                  >
                    {providerModels.length === 0 && agentForm.modelId && (
                      <option value={agentForm.modelId}>{agentForm.modelId}</option>
                    )}
                    {providerModels.map((model) => (
                      <option key={model.id} value={model.id}>
                        {model.name ?? model.id}
                      </option>
                    ))}
                  </select>
                </Field>
              </div>
              <Field label="Persona">
                <Textarea
                  value={agentForm.persona}
                  onChange={(event) => setAgentForm({ ...agentForm, persona: event.target.value })}
                  className="min-h-24"
                />
              </Field>
              <div className="flex items-center justify-between gap-3">
                <SwitchRow
                  label="Enabled"
                  checked={agentForm.enabled}
                  onCheckedChange={(enabled) => setAgentForm({ ...agentForm, enabled })}
                />
                <SwitchRow
                  label="Scheduled"
                  checked={agentForm.scheduleEnabled}
                  onCheckedChange={(scheduleEnabled) =>
                    setAgentForm({ ...agentForm, scheduleEnabled })
                  }
                />
              </div>
              <Button
                className="w-full"
                onClick={createAgent}
                disabled={!agentForm.name.trim() || !agentForm.providerId || !agentForm.modelId}
              >
                <Icons.Plus className="mr-2 h-4 w-4" />
                Add agent
              </Button>
            </div>

            <div className="border-border border-t pt-5">
              <div className="mb-3 flex items-center justify-between">
                <h3 className="text-sm font-medium">Challenge</h3>
                <Badge variant="outline">Long only</Badge>
              </div>
              <div className="space-y-3">
                <Field label="Name">
                  <Input
                    value={challengeForm.name}
                    onChange={(event) =>
                      setChallengeForm({ ...challengeForm, name: event.target.value })
                    }
                  />
                </Field>
                <Field label="Description">
                  <Textarea
                    value={challengeForm.description}
                    onChange={(event) =>
                      setChallengeForm({ ...challengeForm, description: event.target.value })
                    }
                    className="min-h-16"
                  />
                </Field>
                <div className="grid grid-cols-2 gap-2">
                  <Field label="Market">
                    <Input
                      value={challengeForm.market}
                      onChange={(event) =>
                        setChallengeForm({ ...challengeForm, market: event.target.value })
                      }
                    />
                  </Field>
                  <Field label="Cadence">
                    <Input
                      value={challengeForm.runCadence}
                      onChange={(event) =>
                        setChallengeForm({ ...challengeForm, runCadence: event.target.value })
                      }
                    />
                  </Field>
                </div>
                <Field label="Universe">
                  <Input
                    value={challengeForm.universe}
                    onChange={(event) =>
                      setChallengeForm({ ...challengeForm, universe: event.target.value })
                    }
                  />
                </Field>
                <div className="grid grid-cols-3 gap-2">
                  <Field label="Cash">
                    <Input
                      type="number"
                      value={challengeForm.initialCash}
                      onChange={(event) =>
                        setChallengeForm({ ...challengeForm, initialCash: event.target.value })
                      }
                    />
                  </Field>
                  <Field label="Max %">
                    <Input
                      type="number"
                      value={challengeForm.maxPositionPct}
                      onChange={(event) =>
                        setChallengeForm({ ...challengeForm, maxPositionPct: event.target.value })
                      }
                    />
                  </Field>
                  <Field label="DD %">
                    <Input
                      type="number"
                      value={challengeForm.maxDrawdownPct}
                      onChange={(event) =>
                        setChallengeForm({ ...challengeForm, maxDrawdownPct: event.target.value })
                      }
                    />
                  </Field>
                </div>
                <Field label="Time">
                  <Input
                    value={challengeForm.scheduledTimeLocal}
                    onChange={(event) =>
                      setChallengeForm({ ...challengeForm, scheduledTimeLocal: event.target.value })
                    }
                  />
                </Field>
                <Button
                  className="w-full"
                  variant="secondary"
                  onClick={createChallenge}
                  disabled={!challengeForm.name.trim()}
                >
                  <Icons.PlusCircle className="mr-2 h-4 w-4" />
                  Add challenge
                </Button>
              </div>
            </div>
          </div>
        </section>

        <main className="border-border bg-card min-h-0 overflow-hidden rounded-md border">
          <div className="border-border flex flex-wrap items-center justify-between gap-3 border-b p-4">
            <div className="min-w-0">
              <h2 className="truncate text-sm font-semibold">
                {selectedChallenge?.name ?? "No challenge"}
              </h2>
              {selectedChallenge && (
                <div className="text-muted-foreground mt-1 flex flex-wrap gap-x-3 gap-y-1 text-xs">
                  <span>{selectedChallenge.market}</span>
                  <span>{formatMoney(selectedChallenge.initialCash)}</span>
                  <span>max {decimal.format(selectedChallenge.maxPositionPct)}%</span>
                  <span>{selectedChallenge.universe.join(", ") || "open universe"}</span>
                </div>
              )}
            </div>
            <select
              className="border-input bg-background h-9 min-w-52 rounded-md border px-3 text-sm"
              value={selectedChallengeId}
              onChange={(event) => {
                setSelectedChallengeId(event.target.value);
                setSelectedParticipantId("");
              }}
            >
              {challenges.length === 0 && <option value="">No challenges</option>}
              {challenges.map((challenge) => (
                <option key={challenge.id} value={challenge.id}>
                  {challenge.name}
                </option>
              ))}
            </select>
          </div>

          <div className="grid min-h-0 gap-0 lg:grid-cols-[260px_minmax(0,1fr)]">
            <aside className="border-border min-h-0 border-b p-4 lg:border-b-0 lg:border-r">
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
                    No participants
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
                      {trades.length === 0 && <EmptyRow colSpan={5} label="No trades" />}
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
                      {runs.length === 0 && <EmptyRow colSpan={5} label="No runs" />}
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
                Select participant
              </div>
            )}

            <div className="border-border border-t pt-5">
              <h3 className="mb-3 text-sm font-medium">Thesis</h3>
              <div className="space-y-3">
                <div className="grid grid-cols-3 gap-2">
                  <Field label="Symbol">
                    <Input
                      value={thesisForm.symbol}
                      onChange={(event) =>
                        setThesisForm({ ...thesisForm, symbol: event.target.value })
                      }
                    />
                  </Field>
                  <Field label="Rating">
                    <Input
                      value={thesisForm.rating}
                      onChange={(event) =>
                        setThesisForm({ ...thesisForm, rating: event.target.value })
                      }
                    />
                  </Field>
                  <Field label="Conf.">
                    <Input
                      type="number"
                      value={thesisForm.confidence}
                      onChange={(event) =>
                        setThesisForm({ ...thesisForm, confidence: event.target.value })
                      }
                    />
                  </Field>
                </div>
                <Field label="Horizon">
                  <Input
                    value={thesisForm.horizon}
                    onChange={(event) =>
                      setThesisForm({ ...thesisForm, horizon: event.target.value })
                    }
                  />
                </Field>
                <Field label="Thesis">
                  <Textarea
                    value={thesisForm.thesis}
                    onChange={(event) =>
                      setThesisForm({ ...thesisForm, thesis: event.target.value })
                    }
                    className="min-h-24"
                  />
                </Field>
                <div className="grid grid-cols-2 gap-2">
                  <Field label="Risks">
                    <Textarea
                      value={thesisForm.risks}
                      onChange={(event) =>
                        setThesisForm({ ...thesisForm, risks: event.target.value })
                      }
                      className="min-h-20"
                    />
                  </Field>
                  <Field label="Catalysts">
                    <Textarea
                      value={thesisForm.catalysts}
                      onChange={(event) =>
                        setThesisForm({ ...thesisForm, catalysts: event.target.value })
                      }
                      className="min-h-20"
                    />
                  </Field>
                </div>
                <Button
                  className="w-full"
                  variant="secondary"
                  onClick={createThesis}
                  disabled={!thesisForm.symbol.trim() || !thesisForm.thesis.trim()}
                >
                  <Icons.Save className="mr-2 h-4 w-4" />
                  Save thesis
                </Button>
              </div>
            </div>

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
                    No theses
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

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1.5">
      <Label className="text-xs">{label}</Label>
      {children}
    </div>
  );
}

function SwitchRow({
  label,
  checked,
  onCheckedChange,
}: {
  label: string;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
}) {
  return (
    <label className="flex items-center gap-2 text-sm">
      <Switch size="sm" checked={checked} onCheckedChange={onCheckedChange} />
      {label}
    </label>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border p-3">
      <div className="text-muted-foreground text-xs">{label}</div>
      <div className="mt-1 text-lg font-semibold">{value}</div>
    </div>
  );
}

function ParticipantButton({
  participant,
  agent,
  selected,
  running,
  onSelect,
  onRun,
}: {
  participant: ArenaParticipant;
  agent?: ArenaAgent;
  selected: boolean;
  running: boolean;
  onSelect: () => void;
  onRun: () => void;
}) {
  return (
    <div
      className={`rounded-md border p-2 ${selected ? "border-primary bg-primary/5" : "border-border"}`}
    >
      <button type="button" className="w-full text-left" onClick={onSelect}>
        <div className="flex items-center justify-between gap-2">
          <span className="truncate text-sm font-medium">{agent?.name ?? participant.agentId}</span>
          <Badge variant={statusVariant(participant.status)}>{participant.status}</Badge>
        </div>
        <div className="text-muted-foreground mt-1 truncate text-xs">
          {agent?.providerId ?? ""} {agent?.modelId ?? ""}
        </div>
      </button>
      <Button
        size="sm"
        variant="ghost"
        className="mt-2 w-full justify-start"
        onClick={onRun}
        disabled={running}
      >
        {running ? (
          <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />
        ) : (
          <Icons.PlayCircle className="mr-2 h-4 w-4" />
        )}
        Run
      </Button>
    </div>
  );
}

function LeaderboardTable({
  entries,
  participants,
  agentsById,
  onSelectParticipant,
}: {
  entries: ArenaLeaderboardEntry[];
  participants: ArenaParticipant[];
  agentsById: Map<string, ArenaAgent>;
  onSelectParticipant: (participantId: string) => void;
}) {
  const participantRows =
    entries.length > 0
      ? entries
      : participants.map((participant) => ({
          rank: null,
          participantId: participant.id,
          agentId: participant.agentId,
          agentName: agentsById.get(participant.agentId)?.name ?? participant.agentId,
          totalValue: participant.startingCash,
          cash: participant.startingCash,
          returnPct: 0,
          maxDrawdownPct: 0,
          riskAdjustedScore: 0,
          finalScore: null,
          tradeCount: 0,
          disqualifiedReason: null,
        }));

  return (
    <div className="max-h-[560px] overflow-auto rounded-md border">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-14">Rank</TableHead>
            <TableHead>Agent</TableHead>
            <TableHead className="text-right">Value</TableHead>
            <TableHead className="text-right">Return</TableHead>
            <TableHead className="text-right">DD</TableHead>
            <TableHead className="text-right">Score</TableHead>
            <TableHead className="text-right">Trades</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {participantRows.map((entry) => (
            <TableRow
              key={entry.participantId}
              className="cursor-pointer"
              onClick={() => onSelectParticipant(entry.participantId)}
            >
              <TableCell>{entry.rank ?? "-"}</TableCell>
              <TableCell>
                <div className="font-medium">{entry.agentName}</div>
                {entry.disqualifiedReason && (
                  <div className="text-destructive text-xs">{entry.disqualifiedReason}</div>
                )}
              </TableCell>
              <TableCell className="text-right">{formatMoney(entry.totalValue)}</TableCell>
              <TableCell className="text-right">{formatPct(entry.returnPct)}</TableCell>
              <TableCell className="text-right">
                {formatPct(-Math.abs(entry.maxDrawdownPct))}
              </TableCell>
              <TableCell className="text-right">
                {entry.finalScore == null ? "-" : decimal.format(entry.finalScore)}
              </TableCell>
              <TableCell className="text-right">{entry.tradeCount}</TableCell>
            </TableRow>
          ))}
          {participantRows.length === 0 && <EmptyRow colSpan={7} label="No leaderboard" />}
        </TableBody>
      </Table>
    </div>
  );
}

function EmptyRow({ colSpan, label }: { colSpan: number; label: string }) {
  return (
    <TableRow>
      <TableCell colSpan={colSpan} className="text-muted-foreground h-24 text-center text-sm">
        {label}
      </TableCell>
    </TableRow>
  );
}
