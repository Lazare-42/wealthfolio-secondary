import { useState } from "react";
import { useNavigate } from "react-router-dom";

import { useAiArenaMutations } from "@/hooks/use-ai-arena";
import type { GeneratedArenaChallengeSpec } from "@/lib/types";
import { Icons, Page, PageContent, PageHeader } from "@wealthfolio/ui";
import { Badge } from "@wealthfolio/ui/components/ui/badge";
import { Button } from "@wealthfolio/ui/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@wealthfolio/ui/components/ui/card";
import { Textarea } from "@wealthfolio/ui/components/ui/textarea";

import {
  ChallengeFields,
  defaultChallengeForm,
  splitSymbols,
  toCreateChallengeRequest,
  type ChallengeFormState,
} from "./components/challenge-fields";

export default function ChallengeCreatePage() {
  const navigate = useNavigate();
  const { createChallengeMutation, generateChallengeSpecMutation } = useAiArenaMutations();

  const [form, setForm] = useState<ChallengeFormState>(defaultChallengeForm);
  const [theme, setTheme] = useState("");
  const [generatedSpec, setGeneratedSpec] = useState<GeneratedArenaChallengeSpec | null>(null);

  const applySpec = (spec: GeneratedArenaChallengeSpec) => {
    // The backend guarantees a non-empty universe (it errors otherwise);
    // never overwrite name/description while silently keeping a stale one.
    if (spec.universe.length === 0) return;
    setGeneratedSpec(spec);
    setForm((current) => ({
      ...current,
      name: spec.name || current.name,
      description: spec.description || current.description,
      universe: spec.universe.join(", "),
    }));
  };

  const generateWithAi = () => {
    generateChallengeSpecMutation.mutate({ theme: theme.trim() }, { onSuccess: applySpec });
  };

  // Only fields the user actually edited count as draft content — prefilled
  // defaults (name, universe) are not user intent, so Enhance omits them and
  // a theme-only enhance behaves like Generate.
  const draft = {
    name: form.name.trim() === defaultChallengeForm.name ? "" : form.name.trim(),
    description: form.description.trim(),
    universe:
      splitSymbols(form.universe).join(",") ===
      splitSymbols(defaultChallengeForm.universe).join(",")
        ? []
        : splitSymbols(form.universe),
  };
  const draftHasContent =
    Boolean(draft.name) || Boolean(draft.description) || draft.universe.length > 0;

  const enhanceWithAi = () => {
    generateChallengeSpecMutation.mutate(
      { theme: theme.trim() || undefined, draft },
      { onSuccess: applySpec },
    );
  };

  // Shown next to both AI actions so the note sits adjacent to whichever
  // action (Generate or Enhance) produced it.
  const droppedNote =
    generatedSpec && generatedSpec.dropped.length > 0 ? (
      <p className="text-muted-foreground text-xs">
        Dropped unresolvable symbols: {generatedSpec.dropped.join(", ")}
      </p>
    ) : null;

  const createChallenge = () => {
    createChallengeMutation.mutate(toCreateChallengeRequest(form), {
      onSuccess: (challenge) => navigate(`/ai-arena?tab=arena&challenge=${challenge.id}`),
    });
  };

  return (
    <Page>
      <PageHeader
        heading="New Challenge"
        text="Set up a paper-trading match for your AI agents"
        onBack={() => navigate("/ai-arena")}
      />
      <PageContent>
        <div className="mx-auto max-w-2xl space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Start with AI</CardTitle>
              <CardDescription className="text-xs">
                Describe your challenge idea and let your AI Assistant draft the name, thesis, and
                ticker universe.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <Textarea
                value={theme}
                onChange={(event) => setTheme(event.target.value)}
                placeholder='e.g. "European defense primes" or "profitable small-cap uranium"'
                className="min-h-16"
              />
              <Button
                className="w-full"
                variant="outline"
                onClick={generateWithAi}
                disabled={!theme.trim() || generateChallengeSpecMutation.isPending}
              >
                {generateChallengeSpecMutation.isPending ? (
                  <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />
                ) : (
                  <Icons.Sparkles className="mr-2 h-4 w-4" />
                )}
                Generate with AI
              </Button>
              <p className="text-muted-foreground text-xs">
                {generatedSpec
                  ? `Generated with ${generatedSpec.providerId} · ${generatedSpec.modelId}`
                  : "Uses your AI Assistant provider"}
              </p>
              {droppedNote}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <div className="flex items-center justify-between gap-2">
                <div>
                  <CardTitle className="text-base">Challenge</CardTitle>
                  <CardDescription className="text-xs">
                    Everything stays editable — tweak the AI output or fill it in yourself.
                  </CardDescription>
                </div>
                <Badge variant="outline" className="shrink-0">
                  Long only
                </Badge>
              </div>
            </CardHeader>
            <CardContent className="space-y-3">
              <ChallengeFields form={form} onChange={setForm} />
              <Button
                className="w-full"
                variant="outline"
                onClick={enhanceWithAi}
                disabled={!draftHasContent || generateChallengeSpecMutation.isPending}
              >
                {generateChallengeSpecMutation.isPending ? (
                  <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />
                ) : (
                  <Icons.Sparkles className="mr-2 h-4 w-4" />
                )}
                Enhance with AI
              </Button>
              <p className="text-muted-foreground text-xs">
                Enhance keeps your name, intent, and tickers, sharpens the thesis, and adds
                complementary symbols.
              </p>
              {droppedNote}
            </CardContent>
          </Card>

          <div className="flex gap-3">
            <Button variant="outline" className="flex-1" onClick={() => navigate("/ai-arena")}>
              Cancel
            </Button>
            <Button
              className="flex-1"
              onClick={createChallenge}
              disabled={!form.name.trim() || createChallengeMutation.isPending}
            >
              {createChallengeMutation.isPending ? (
                <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Icons.PlusCircle className="mr-2 h-4 w-4" />
              )}
              Create challenge
            </Button>
          </div>
        </div>
      </PageContent>
    </Page>
  );
}
