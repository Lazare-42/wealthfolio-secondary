import type { CreateArenaChallengeRequest } from "@/lib/types";
import { Input } from "@wealthfolio/ui/components/ui/input";
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
      <div className="grid grid-cols-2 gap-2">
        <Field label="Market">
          <Input
            value={form.market}
            onChange={(event) => onChange({ ...form, market: event.target.value })}
          />
        </Field>
        <Field label="Cadence">
          <Input
            value={form.runCadence}
            onChange={(event) => onChange({ ...form, runCadence: event.target.value })}
          />
        </Field>
      </div>
      <Field label="Universe">
        <Input
          value={form.universe}
          onChange={(event) => onChange({ ...form, universe: event.target.value })}
        />
      </Field>
      <div className="grid grid-cols-3 gap-2">
        <Field label="Cash">
          <Input
            type="number"
            value={form.initialCash}
            onChange={(event) => onChange({ ...form, initialCash: event.target.value })}
          />
        </Field>
        <Field label="Max %">
          <Input
            type="number"
            value={form.maxPositionPct}
            onChange={(event) => onChange({ ...form, maxPositionPct: event.target.value })}
          />
        </Field>
        <Field label="DD %">
          <Input
            type="number"
            value={form.maxDrawdownPct}
            onChange={(event) => onChange({ ...form, maxDrawdownPct: event.target.value })}
          />
        </Field>
      </div>
      <Field label="Time">
        <Input
          value={form.scheduledTimeLocal}
          onChange={(event) => onChange({ ...form, scheduledTimeLocal: event.target.value })}
        />
      </Field>
    </div>
  );
}
