import { useState } from "react";

import type { useAiArenaMutations } from "@/hooks/use-ai-arena";
import type { CreateCompanyThesisRequest } from "@/lib/types";
import { Icons } from "@wealthfolio/ui";
import { Button } from "@wealthfolio/ui/components/ui/button";
import { Input } from "@wealthfolio/ui/components/ui/input";
import { Textarea } from "@wealthfolio/ui/components/ui/textarea";

import { Field } from "./field";

function splitTextList(value: string): string[] {
  return value
    .split(/\n|,/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function numberOr(value: string, fallback: number): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

interface ThesisFormState {
  symbol: string;
  rating: string;
  confidence: string;
  horizon: string;
  thesis: string;
  risks: string;
  catalysts: string;
}

const defaultThesisForm: ThesisFormState = {
  symbol: "",
  rating: "",
  confidence: "",
  horizon: "3-6 months",
  thesis: "",
  risks: "",
  catalysts: "",
};

export function ThesisForm({
  createThesisMutation,
  challengeId,
  agentId,
}: {
  createThesisMutation: ReturnType<typeof useAiArenaMutations>["createThesisMutation"];
  challengeId?: string;
  agentId?: string;
}) {
  const [thesisForm, setThesisForm] = useState<ThesisFormState>(defaultThesisForm);

  const createThesis = () => {
    const payload: CreateCompanyThesisRequest = {
      symbol: thesisForm.symbol.trim().toUpperCase(),
      challengeId: challengeId || null,
      agentId: agentId ?? null,
      rating: thesisForm.rating.trim() || null,
      confidence: thesisForm.confidence.trim() ? numberOr(thesisForm.confidence, 0) : null,
      horizon: thesisForm.horizon.trim() || null,
      thesis: thesisForm.thesis.trim(),
      risks: splitTextList(thesisForm.risks),
      catalysts: splitTextList(thesisForm.catalysts),
    };
    createThesisMutation.mutate(payload, {
      onSuccess: () => setThesisForm(defaultThesisForm),
    });
  };

  return (
    <div className="border-border border-t pt-5">
      <h3 className="mb-3 text-sm font-medium">Thesis</h3>
      <div className="space-y-3">
        <div className="grid grid-cols-3 gap-2">
          <Field label="Symbol">
            <Input
              value={thesisForm.symbol}
              onChange={(event) => setThesisForm({ ...thesisForm, symbol: event.target.value })}
            />
          </Field>
          <Field label="Rating">
            <Input
              value={thesisForm.rating}
              onChange={(event) => setThesisForm({ ...thesisForm, rating: event.target.value })}
            />
          </Field>
          <Field label="Conf.">
            <Input
              type="number"
              value={thesisForm.confidence}
              onChange={(event) => setThesisForm({ ...thesisForm, confidence: event.target.value })}
            />
          </Field>
        </div>
        <Field label="Horizon">
          <Input
            value={thesisForm.horizon}
            onChange={(event) => setThesisForm({ ...thesisForm, horizon: event.target.value })}
          />
        </Field>
        <Field label="Thesis">
          <Textarea
            value={thesisForm.thesis}
            onChange={(event) => setThesisForm({ ...thesisForm, thesis: event.target.value })}
            className="min-h-24"
          />
        </Field>
        <div className="grid grid-cols-2 gap-2">
          <Field label="Risks">
            <Textarea
              value={thesisForm.risks}
              onChange={(event) => setThesisForm({ ...thesisForm, risks: event.target.value })}
              className="min-h-20"
            />
          </Field>
          <Field label="Catalysts">
            <Textarea
              value={thesisForm.catalysts}
              onChange={(event) => setThesisForm({ ...thesisForm, catalysts: event.target.value })}
              className="min-h-20"
            />
          </Field>
        </div>
        <Button
          className="w-full"
          variant="secondary"
          onClick={createThesis}
          disabled={!thesisForm.symbol.trim() || !thesisForm.thesis.trim()}
        >
          <Icons.Save className="mr-2 h-4 w-4" />
          Save thesis
        </Button>
      </div>
    </div>
  );
}
