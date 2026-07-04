import { useState } from "react";

import type {
  ArenaAgent,
  ArenaChallenge,
  ArenaLeaderboardEntry,
  ArenaParticipant,
  ArenaRun,
  ArenaTrade,
} from "@/lib/types";
import { Badge } from "@wealthfolio/ui/components/ui/badge";
import { Button } from "@wealthfolio/ui/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@wealthfolio/ui/components/ui/tabs";
import { Icons } from "@wealthfolio/ui";

import { decimal, formatMoney } from "./formatters";
import { JoinAgentPanel } from "./join-agent-panel";
import { LeaderboardTable } from "./leaderboard-table";
import { ParticipantButton } from "./participant-button";
import { RunsTable } from "./runs-table";
import { TradesTable } from "./trades-table";

export function ArenaTab({
  challenge,
  agents,
  agentsById,
  participants,
  availableAgents,
  selectedParticipantId,
  leaderboardEntries,
  trades,
  runs,
  onSelectParticipant,
  onOpenParticipant,
  onJoin,
  onRunParticipant,
  isRunningParticipant,
  onGoToSetup,
}: {
  challenge?: ArenaChallenge;
  agents: ArenaAgent[];
  agentsById: Map<string, ArenaAgent>;
  participants: ArenaParticipant[];
  availableAgents: ArenaAgent[];
  selectedParticipantId: string;
  leaderboardEntries: ArenaLeaderboardEntry[];
  trades: ArenaTrade[];
  runs: ArenaRun[];
  onSelectParticipant: (participantId: string) => void;
  onOpenParticipant: (participantId: string) => void;
  onJoin: (agentId: string) => void;
  onRunParticipant: (participant: ArenaParticipant) => void;
  isRunningParticipant: (participant: ArenaParticipant) => boolean;
  onGoToSetup: () => void;
}) {
  const [activityView, setActivityView] = useState("trades");

  return (
    <div className="space-y-4">
      <div className="border-border bg-card rounded-md border p-4">
        <h2 className="truncate text-sm font-semibold">{challenge?.name ?? "No challenge"}</h2>
        {challenge ? (
          <div className="text-muted-foreground mt-1 flex flex-wrap gap-x-3 gap-y-1 text-xs">
            <span>{challenge.market}</span>
            <span>{formatMoney(challenge.initialCash)}</span>
            <span>max {decimal.format(challenge.maxPositionPct)}%</span>
            <span
              className="min-w-0 max-w-full truncate"
              title={challenge.universe.length > 0 ? challenge.universe.join(", ") : undefined}
            >
              {challenge.universe.join(", ") || "open universe"}
            </span>
          </div>
        ) : (
          <p className="text-muted-foreground mt-1 text-xs">
            Create a challenge in Setup to start a match — agents trade paper money against its
            universe.
          </p>
        )}
      </div>

      <div className="grid items-start gap-4 lg:grid-cols-[280px_minmax(0,1fr)]">
        <aside id="arena-participants" className="border-border bg-card rounded-md border p-4">
          <div className="mb-3 flex items-center justify-between">
            <h3 className="text-sm font-medium">Participants</h3>
            <Badge variant="outline">{participants.length}</Badge>
          </div>
          <div className="space-y-2">
            {participants.map((participant) => (
              <ParticipantButton
                key={participant.id}
                participant={participant}
                agent={agentsById.get(participant.agentId)}
                selected={participant.id === selectedParticipantId}
                onSelect={() => onSelectParticipant(participant.id)}
                onRun={() => onRunParticipant(participant)}
                running={isRunningParticipant(participant)}
              />
            ))}
            {participants.length === 0 && (
              <div className="text-muted-foreground rounded-md border border-dashed p-3 text-sm">
                {!challenge ? (
                  "Select or create a challenge to add participants."
                ) : agents.length === 0 ? (
                  <div className="space-y-2">
                    <p>No agents yet — an agent is needed to enter this match.</p>
                    <Button size="sm" variant="outline" onClick={onGoToSetup}>
                      <Icons.Plus className="mr-2 h-4 w-4" />
                      Create an agent in Setup
                    </Button>
                  </div>
                ) : (
                  "Join an agent below to enter it in this match."
                )}
              </div>
            )}
          </div>

          {challenge && availableAgents.length > 0 && (
            <JoinAgentPanel availableAgents={availableAgents} onJoin={onJoin} />
          )}
        </aside>

        <div className="space-y-4">
          <section className="border-border bg-card rounded-md border p-4">
            <h3 className="mb-3 text-sm font-medium">Leaderboard</h3>
            <LeaderboardTable
              entries={leaderboardEntries}
              participants={participants}
              agentsById={agentsById}
              onSelectParticipant={onOpenParticipant}
            />
          </section>

          <section className="border-border bg-card rounded-md border p-4">
            <div className="mb-3 flex items-center justify-between">
              <h3 className="text-sm font-medium">Activity</h3>
              <Tabs value={activityView} onValueChange={setActivityView}>
                <TabsList className="h-8">
                  <TabsTrigger value="trades" className="text-xs">
                    Trades
                  </TabsTrigger>
                  <TabsTrigger value="runs" className="text-xs">
                    Runs
                  </TabsTrigger>
                </TabsList>
              </Tabs>
            </div>
            {activityView === "trades" ? (
              <TradesTable trades={trades} />
            ) : (
              <RunsTable runs={runs} agentsById={agentsById} />
            )}
          </section>
        </div>
      </div>
    </div>
  );
}
