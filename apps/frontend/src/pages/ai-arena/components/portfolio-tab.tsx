import { useState } from "react";

import type { useAiArenaMutations } from "@/hooks/use-ai-arena";
import type { ArenaPortfolio, CompanyThesis } from "@/lib/types";
import { Button } from "@wealthfolio/ui/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@wealthfolio/ui/components/ui/table";
import { Icons } from "@wealthfolio/ui";

import { EmptyRow } from "./empty-row";
import { decimal, formatMoney, formatPct } from "./formatters";
import { Metric } from "./metric";
import { ThesesList } from "./theses-list";
import { ThesisForm } from "./thesis-form";

export function PortfolioTab({
  portfolio,
  hasParticipants,
  theses,
  createThesisMutation,
  challengeId,
  agentId,
}: {
  portfolio?: ArenaPortfolio;
  hasParticipants: boolean;
  theses: CompanyThesis[];
  createThesisMutation: ReturnType<typeof useAiArenaMutations>["createThesisMutation"];
  challengeId?: string;
  agentId?: string;
}) {
  const [showThesisForm, setShowThesisForm] = useState(false);

  return (
    <div className="mx-auto max-w-3xl space-y-4">
      {portfolio ? (
        <div className="border-border bg-card space-y-5 rounded-md border p-4">
          <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
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
        </div>
      ) : (
        <div className="text-muted-foreground rounded-md border border-dashed p-4 text-sm">
          {!hasParticipants
            ? "No participants yet — join an agent to a challenge to build a paper portfolio."
            : "Select a participant to view its cash, positions, return, and drawdown."}
        </div>
      )}

      <div className="border-border bg-card rounded-md border p-4">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-medium">Theses</h3>
          {!showThesisForm && (
            <Button size="sm" variant="outline" onClick={() => setShowThesisForm(true)}>
              <Icons.Pencil className="mr-2 h-4 w-4" />
              Write a thesis
            </Button>
          )}
        </div>
        {showThesisForm && (
          <div className="mt-3">
            <ThesisForm
              createThesisMutation={createThesisMutation}
              challengeId={challengeId}
              agentId={agentId}
            />
          </div>
        )}
        <div className="mt-4">
          <ThesesList theses={theses} />
        </div>
      </div>
    </div>
  );
}
