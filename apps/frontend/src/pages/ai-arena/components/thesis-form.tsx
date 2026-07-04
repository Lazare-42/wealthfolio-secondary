import { useState } from "react";

import type { useAiArenaMutations } from "@/hooks/use-ai-arena";
import type { CreateCompanyThesisRequest } from "@/lib/types";
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
  const [detailsOpen, setDetailsOpen] = useState(false);

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
    <div className="space-y-3">
      <Field label="Symbol">
        <Input
          value={thesisForm.symbol}
          onChange={(event) => setThesisForm({ ...thesisForm, symbol: event.target.value })}
          placeholder="AAPL"
        />
      </Field>
      <Field label="Thesis">
        <Textarea
          value={thesisForm.thesis}
          onChange={(event) => setThesisForm({ ...thesisForm, thesis: event.target.value })}
          className="min-h-24"
        />
      </Field>

      <Collapsible open={detailsOpen} onOpenChange={setDetailsOpen}>
        <CollapsibleTrigger asChild>
          <Button
            variant="ghost"
            size="sm"
            type="button"
            className="text-muted-foreground hover:text-foreground flex w-full items-center justify-between px-0 py-1 hover:bg-transparent"
          >
            <span className="text-sm font-medium">Details</span>
            <Icons.ChevronDown
              className={`h-4 w-4 transition-transform ${detailsOpen ? "rotate-180" : ""}`}
            />
          </Button>
        </CollapsibleTrigger>
        <CollapsibleContent className="space-y-3 pt-2">
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
            <Field label="Rating">
              <Select
                value={thesisForm.rating}
                onValueChange={(rating) => setThesisForm({ ...thesisForm, rating })}
              >
                <SelectTrigger>
                  <SelectValue placeholder="None" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="buy">Buy</SelectItem>
                  <SelectItem value="hold">Hold</SelectItem>
                  <SelectItem value="sell">Sell</SelectItem>
                </SelectContent>
              </Select>
            </Field>
            <Field label="Confidence (0–1)" help="How sure you are, from 0 to 1.">
              <Input
                type="number"
                min={0}
                max={1}
                step={0.1}
                value={thesisForm.confidence}
                onChange={(event) =>
                  setThesisForm({ ...thesisForm, confidence: event.target.value })
                }
              />
            </Field>
          </div>
          <Field label="Horizon">
            <Input
              value={thesisForm.horizon}
              onChange={(event) => setThesisForm({ ...thesisForm, horizon: event.target.value })}
            />
          </Field>
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
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
                onChange={(event) =>
                  setThesisForm({ ...thesisForm, catalysts: event.target.value })
                }
                className="min-h-20"
              />
            </Field>
          </div>
        </CollapsibleContent>
      </Collapsible>

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
  );
}
