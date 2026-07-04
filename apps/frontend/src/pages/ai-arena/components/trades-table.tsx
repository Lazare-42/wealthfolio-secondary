import type { ArenaTrade } from "@/lib/types";
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
import { formatMoney, statusVariant } from "./formatters";

export function TradesTable({ trades }: { trades: ArenaTrade[] }) {
  return (
    <div className="max-h-[560px] overflow-auto rounded-md border">
      {/* min-w only when populated so wide rows scroll instead of crushing columns */}
      <Table className={trades.length > 0 ? "min-w-[36rem]" : undefined}>
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
              <TableCell className="text-right">{formatMoney(trade.notional)}</TableCell>
              <TableCell>
                <Badge variant={statusVariant(trade.status)}>{trade.status}</Badge>
              </TableCell>
              <TableCell
                className="text-muted-foreground max-w-80 truncate"
                title={trade.rejectionReason ?? trade.rationale ?? undefined}
              >
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
  );
}
