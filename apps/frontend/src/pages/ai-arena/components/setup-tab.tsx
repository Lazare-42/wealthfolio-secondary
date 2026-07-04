import type { useAiArenaMutations } from "@/hooks/use-ai-arena";
import type { ArenaAgent, ArenaChallenge, MergedProvider } from "@/lib/types";
import { Icons } from "@wealthfolio/ui";
import { Badge } from "@wealthfolio/ui/components/ui/badge";
import { Button } from "@wealthfolio/ui/components/ui/button";

import { AgentForm } from "./agent-form";
import { AgentsList } from "./agents-list";
import { decimal, formatMoney, statusVariant } from "./formatters";

export function SetupTab({
  enabledProviders,
  agents,
  challenges,
  createAgentMutation,
  onOpenChallenge,
  onNewChallenge,
}: {
  enabledProviders: MergedProvider[];
  agents: ArenaAgent[];
  challenges: ArenaChallenge[];
  createAgentMutation: ReturnType<typeof useAiArenaMutations>["createAgentMutation"];
  onOpenChallenge: (challengeId: string) => void;
  onNewChallenge: () => void;
}) {
  return (
    <div className="grid items-start gap-4 lg:grid-cols-2">
      <div className="max-w-xl space-y-4">
        <section id="arena-agent-form" className="border-border bg-card rounded-md border p-4">
          <AgentForm
            enabledProviders={enabledProviders}
            createAgentMutation={createAgentMutation}
          />
        </section>
        <section className="border-border bg-card rounded-md border p-4">
          <div className="mb-3 flex items-center justify-between">
            <h3 className="text-sm font-medium">Your agents</h3>
            <Badge variant="outline">{agents.length}</Badge>
          </div>
          <AgentsList agents={agents} />
        </section>
      </div>

      <section className="border-border bg-card max-w-xl rounded-md border p-4">
        <div className="mb-3 flex items-center justify-between">
          <h3 className="text-sm font-medium">Challenges</h3>
          <Button size="sm" variant="outline" onClick={onNewChallenge}>
            <Icons.PlusCircle className="mr-2 h-4 w-4" />
            New challenge
          </Button>
        </div>
        <div className="space-y-2">
          {challenges.map((challenge) => (
            <div key={challenge.id} className="rounded-md border p-3">
              <div className="flex items-center justify-between gap-2">
                <span className="truncate text-sm font-medium">{challenge.name}</span>
                <Badge variant={statusVariant(challenge.status)}>{challenge.status}</Badge>
              </div>
              <div className="text-muted-foreground mt-1 flex flex-wrap gap-x-3 gap-y-1 text-xs">
                <span>{challenge.market}</span>
                <span>{formatMoney(challenge.initialCash)}</span>
                <span>max {decimal.format(challenge.maxPositionPct)}%</span>
                <span>
                  {challenge.universe.length > 0
                    ? `${challenge.universe.length} tickers`
                    : "open universe"}
                </span>
              </div>
              <Button
                size="sm"
                variant="ghost"
                className="mt-2 h-7 px-2 text-xs"
                onClick={() => onOpenChallenge(challenge.id)}
              >
                Open in Arena
                <Icons.ArrowRight className="ml-1 h-3 w-3" />
              </Button>
            </div>
          ))}
          {challenges.length === 0 && (
            <div className="text-muted-foreground space-y-3 rounded-md border border-dashed p-4 text-sm">
              <p>No challenges yet. Design a challenge — with AI help — on its own page.</p>
              <Button size="sm" onClick={onNewChallenge}>
                <Icons.PlusCircle className="mr-2 h-4 w-4" />
                New challenge
              </Button>
            </div>
          )}
        </div>
      </section>
    </div>
  );
}
