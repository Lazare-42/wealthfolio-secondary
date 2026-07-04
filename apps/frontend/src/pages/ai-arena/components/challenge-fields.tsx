import { useState } from "react";

import type { CreateArenaChallengeRequest } from "@/lib/types";
import { Icons } from "@wealthfolio/ui";
import { Button } from "@wealthfolio/ui/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@wealthfolio/ui/components/ui/collapsible";
import { Input } from "@wealthfolio/ui/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@wealthfolio/ui/components/ui/select";
import { Textarea } from "@wealthfolio/ui/components/ui/textarea";

import { Field } from "./field";

export function splitSymbols(value: string): string[] {
  return value
    .split(/[\s,]+/)
    .map((symbol) => symbol.trim().toUpperCase())
    .filter(Boolean);
}

function numberOr(value: string, fallback: number): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

export interface ChallengeFormState {
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

export const defaultChallengeForm: ChallengeFormState = {
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

export function toCreateChallengeRequest(form: ChallengeFormState): CreateArenaChallengeRequest {
  return {
    name: form.name.trim(),
    description: form.description.trim() || null,
    market: form.market.trim() || "us-stock",
    scoringMethod: "riskAdjusted",
    initialCash: numberOr(form.initialCash, 100000),
    maxPositionPct: numberOr(form.maxPositionPct, 50),
    maxDrawdownPct: numberOr(form.maxDrawdownPct, 25),
    runCadence: form.runCadence.trim() || "daily",
    scheduledTimeLocal: form.scheduledTimeLocal.trim() || null,
    universe: splitSymbols(form.universe),
  };
}

/** The full challenge field set, shared by the create page and any inline use. */
export function ChallengeFields({
  form,
  onChange,
}: {
  form: ChallengeFormState;
  onChange: (form: ChallengeFormState) => void;
}) {
  const [advancedOpen, setAdvancedOpen] = useState(false);

  return (
    <div className="space-y-3">
      <Field label="Name">
        <Input
          value={form.name}
          onChange={(event) => onChange({ ...form, name: event.target.value })}
        />
      </Field>
      <Field label="Description">
        <Textarea
          value={form.description}
          onChange={(event) => onChange({ ...form, description: event.target.value })}
          className="min-h-16"
        />
      </Field>
      <Field label="Market" help="AI Arena currently supports US stocks only — more markets later.">
        <Select value={form.market} onValueChange={(market) => onChange({ ...form, market })}>
          <SelectTrigger disabled>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="us-stock">US stocks</SelectItem>
          </SelectContent>
        </Select>
      </Field>
      <Field
        label="Universe"
        help="Tickers, comma or space separated — leave empty for an open universe."
      >
        <Input
          value={form.universe}
          onChange={(event) => onChange({ ...form, universe: event.target.value })}
        />
      </Field>

      <Collapsible open={advancedOpen} onOpenChange={setAdvancedOpen}>
        <CollapsibleTrigger asChild>
          <Button
            variant="ghost"
            size="sm"
            type="button"
            className="text-muted-foreground hover:text-foreground flex w-full items-center justify-between px-0 py-1 hover:bg-transparent"
          >
            <span className="text-sm font-medium">Advanced</span>
            <Icons.ChevronDown
              className={`h-4 w-4 transition-transform ${advancedOpen ? "rotate-180" : ""}`}
            />
          </Button>
        </CollapsibleTrigger>
        <CollapsibleContent className="space-y-3 pt-2">
          <h4 className="text-muted-foreground text-xs font-medium uppercase tracking-wide">
            Risk &amp; rules
          </h4>
          <Field label="Initial cash">
            <Input
              type="number"
              value={form.initialCash}
              onChange={(event) => onChange({ ...form, initialCash: event.target.value })}
            />
          </Field>
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
            <Field label="Max position size (%)" help="Largest single position as % of portfolio.">
              <Input
                type="number"
                value={form.maxPositionPct}
                onChange={(event) => onChange({ ...form, maxPositionPct: event.target.value })}
              />
            </Field>
            <Field label="Disqualify at drawdown (%)">
              <Input
                type="number"
                value={form.maxDrawdownPct}
                onChange={(event) => onChange({ ...form, maxDrawdownPct: event.target.value })}
              />
            </Field>
          </div>
          <h4 className="text-muted-foreground text-xs font-medium uppercase tracking-wide">
            Schedule
          </h4>
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
            <Field label="Cadence" help="Scheduled auto-runs currently fire on daily cadence only.">
              <Select
                value={form.runCadence}
                onValueChange={(runCadence) => onChange({ ...form, runCadence })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="daily">Daily</SelectItem>
                  <SelectItem value="weekly">Weekly</SelectItem>
                </SelectContent>
              </Select>
            </Field>
            <Field label="Run time">
              <Input
                type="time"
                value={form.scheduledTimeLocal}
                onChange={(event) => onChange({ ...form, scheduledTimeLocal: event.target.value })}
              />
            </Field>
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
}
