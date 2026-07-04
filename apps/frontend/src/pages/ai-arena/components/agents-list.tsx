import type { ArenaAgent } from "@/lib/types";
import { Badge } from "@wealthfolio/ui/components/ui/badge";

export function AgentsList({ agents }: { agents: ArenaAgent[] }) {
  if (agents.length === 0) {
    return (
      <div className="text-muted-foreground rounded-md border border-dashed p-3 text-sm">
        No agents yet — create one above. Each agent trades with its own provider, model, and
        persona.
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {agents.map((agent) => (
        <div key={agent.id} className="rounded-md border p-3">
          <div className="flex items-center justify-between gap-2">
            <span className="truncate text-sm font-medium">{agent.name}</span>
            <div className="flex shrink-0 items-center gap-1">
              {agent.enabled && <Badge variant="success">Active</Badge>}
              {agent.scheduleEnabled && <Badge variant="outline">Scheduled</Badge>}
            </div>
          </div>
          <div className="text-muted-foreground mt-1 truncate text-xs">
            {agent.providerId} · {agent.modelId}
          </div>
        </div>
      ))}
    </div>
  );
}
