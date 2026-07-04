import type { ArenaAgent } from "@/lib/types";
import { Icons } from "@wealthfolio/ui";
import { Button } from "@wealthfolio/ui/components/ui/button";

export function JoinAgentPanel({
  availableAgents,
  onJoin,
}: {
  availableAgents: ArenaAgent[];
  onJoin: (agentId: string) => void;
}) {
  return (
    <div className="border-border mt-4 border-t pt-4">
      <h3 className="mb-2 text-sm font-medium">Join</h3>
      <div className="space-y-2">
        {availableAgents.map((agent) => (
          <Button
            key={agent.id}
            variant="outline"
            size="sm"
            className="w-full justify-start"
            onClick={() => onJoin(agent.id)}
          >
            <Icons.Plus className="mr-2 h-4 w-4" />
            {agent.name}
          </Button>
        ))}
      </div>
    </div>
  );
}
