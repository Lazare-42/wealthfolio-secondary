import type { ArenaAgent, ArenaParticipant } from "@/lib/types";
import { Icons } from "@wealthfolio/ui";
import { Badge } from "@wealthfolio/ui/components/ui/badge";
import { Button } from "@wealthfolio/ui/components/ui/button";

import { statusVariant } from "./formatters";

export function ParticipantButton({
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
