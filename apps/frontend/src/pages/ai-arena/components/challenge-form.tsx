import { useState } from "react";

import type { useAiArenaMutations } from "@/hooks/use-ai-arena";
import type { CreateArenaChallengeRequest } from "@/lib/types";
import { Icons } from "@wealthfolio/ui";
import { Badge } from "@wealthfolio/ui/components/ui/badge";
import { Button } from "@wealthfolio/ui/components/ui/button";
import { Input } from "@wealthfolio/ui/components/ui/input";
import { Textarea } from "@wealthfolio/ui/components/ui/textarea";

import { Field } from "./field";

function splitSymbols(value: string): string[] {
  return value
    .split(/[\s,]+/)
    .map((symbol) => symbol.trim().toUpperCase())
    .filter(Boolean);
}

function numberOr(value: string, fallback: number): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

interface ChallengeFormState {
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

const defaultChallengeForm: ChallengeFormState = {
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

export function ChallengeForm({
  createChallengeMutation,
  onCreated,
}: {
  createChallengeMutation: ReturnType<typeof useAiArenaMutations>["createChallengeMutation"];
  onCreated: (challengeId: string) => void;
}) {
  const [challengeForm, setChallengeForm] = useState<ChallengeFormState>(defaultChallengeForm);

  const createChallenge = () => {
    const payload: CreateArenaChallengeRequest = {
      name: challengeForm.name.trim(),
      description: challengeForm.description.trim() || null,
      market: challengeForm.market.trim() || "us-stock",
      scoringMethod: "riskAdjusted",
      initialCash: numberOr(challengeForm.initialCash, 100000),
      maxPositionPct: numberOr(challengeForm.maxPositionPct, 50),
      maxDrawdownPct: numberOr(challengeForm.maxDrawdownPct, 25),
      runCadence: challengeForm.runCadence.trim() || "daily",
      scheduledTimeLocal: challengeForm.scheduledTimeLocal.trim() || null,
      universe: splitSymbols(challengeForm.universe),
    };
    createChallengeMutation.mutate(payload, {
      onSuccess: (challenge) => onCreated(challenge.id),
    });
  };

  return (
    <div className="border-border border-t pt-5">
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-sm font-medium">Challenge</h3>
        <Badge variant="outline">Long only</Badge>
      </div>
      <div className="space-y-3">
        <Field label="Name">
          <Input
            value={challengeForm.name}
            onChange={(event) => setChallengeForm({ ...challengeForm, name: event.target.value })}
          />
        </Field>
        <Field label="Description">
          <Textarea
            value={challengeForm.description}
            onChange={(event) =>
              setChallengeForm({ ...challengeForm, description: event.target.value })
            }
            className="min-h-16"
          />
        </Field>
        <div className="grid grid-cols-2 gap-2">
          <Field label="Market">
            <Input
              value={challengeForm.market}
              onChange={(event) =>
                setChallengeForm({ ...challengeForm, market: event.target.value })
              }
            />
          </Field>
          <Field label="Cadence">
            <Input
              value={challengeForm.runCadence}
              onChange={(event) =>
                setChallengeForm({ ...challengeForm, runCadence: event.target.value })
              }
            />
          </Field>
        </div>
        <Field label="Universe">
          <Input
            value={challengeForm.universe}
            onChange={(event) =>
              setChallengeForm({ ...challengeForm, universe: event.target.value })
            }
          />
        </Field>
        <div className="grid grid-cols-3 gap-2">
          <Field label="Cash">
            <Input
              type="number"
              value={challengeForm.initialCash}
              onChange={(event) =>
                setChallengeForm({ ...challengeForm, initialCash: event.target.value })
              }
            />
          </Field>
          <Field label="Max %">
            <Input
              type="number"
              value={challengeForm.maxPositionPct}
              onChange={(event) =>
                setChallengeForm({ ...challengeForm, maxPositionPct: event.target.value })
              }
            />
          </Field>
          <Field label="DD %">
            <Input
              type="number"
              value={challengeForm.maxDrawdownPct}
              onChange={(event) =>
                setChallengeForm({ ...challengeForm, maxDrawdownPct: event.target.value })
              }
            />
          </Field>
        </div>
        <Field label="Time">
          <Input
            value={challengeForm.scheduledTimeLocal}
            onChange={(event) =>
              setChallengeForm({ ...challengeForm, scheduledTimeLocal: event.target.value })
            }
          />
        </Field>
        <Button
          className="w-full"
          variant="secondary"
          onClick={createChallenge}
          disabled={!challengeForm.name.trim()}
        >
          <Icons.PlusCircle className="mr-2 h-4 w-4" />
          Add challenge
        </Button>
      </div>
    </div>
  );
}
