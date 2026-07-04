import type { ArenaAgent, ArenaRun } from "@/lib/types";
import { Badge } from "@wealthfolio/ui/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@wealthfolio/ui/components/ui/table";

import { EmptyRow } from "./empty-row";
import { statusVariant } from "./formatters";

export function RunsTable({
  runs,
  agentsById,
}: {
  runs: ArenaRun[];
  agentsById: Map<string, ArenaAgent>;
}) {
  return (
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
  );
}
