import type { ArenaAgent, ArenaLeaderboardEntry, ArenaParticipant } from "@/lib/types";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@wealthfolio/ui/components/ui/table";

import { EmptyRow } from "./empty-row";
import { decimal, formatMoney, formatPct } from "./formatters";

export function LeaderboardTable({
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
          {participantRows.length === 0 && (
            <EmptyRow
              colSpan={7}
              label="Join agents to a challenge and run them — rankings by return and risk show here."
            />
          )}
        </TableBody>
      </Table>
    </div>
  );
}
