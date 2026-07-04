import { useEffect, useState } from "react";
import { Link } from "react-router-dom";

import type { useAiArenaMutations } from "@/hooks/use-ai-arena";
import type { CreateArenaAgentRequest, MergedProvider } from "@/lib/types";
import { Icons } from "@wealthfolio/ui";
import { Badge } from "@wealthfolio/ui/components/ui/badge";
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
import { SwitchRow } from "./switch-row";

function latestModelId(provider?: MergedProvider) {
  return provider?.selectedModel ?? provider?.defaultModel ?? provider?.models[0]?.id ?? "";
}

interface AgentFormState {
  name: string;
  providerId: string;
  modelId: string;
  persona: string;
  enabled: boolean;
  scheduleEnabled: boolean;
}

const defaultAgentForm: AgentFormState = {
  name: "",
  providerId: "",
  modelId: "",
  persona:
    "Long-only public equity analyst. Prefer liquid stocks and ETFs. Keep position risk explicit.",
  enabled: true,
  scheduleEnabled: false,
};

export function AgentForm({
  enabledProviders,
  createAgentMutation,
}: {
  enabledProviders: MergedProvider[];
  createAgentMutation: ReturnType<typeof useAiArenaMutations>["createAgentMutation"];
}) {
  const [agentForm, setAgentForm] = useState<AgentFormState>(defaultAgentForm);
  const [advancedOpen, setAdvancedOpen] = useState(false);

  const selectedProvider = enabledProviders.find(
    (provider) => provider.id === agentForm.providerId,
  );
  const providerModels = selectedProvider?.models ?? [];

  useEffect(() => {
    if (!agentForm.providerId && enabledProviders.length > 0) {
      const first = enabledProviders[0];
      setAgentForm((current) => ({
        ...current,
        providerId: first.id,
        modelId: latestModelId(first),
      }));
    }
  }, [agentForm.providerId, enabledProviders]);

  const createAgent = () => {
    const payload: CreateArenaAgentRequest = {
      name: agentForm.name.trim(),
      providerId: agentForm.providerId,
      modelId: agentForm.modelId,
      persona: agentForm.persona.trim() || null,
      enabled: agentForm.enabled,
      scheduleEnabled: agentForm.scheduleEnabled,
    };
    createAgentMutation.mutate(payload, {
      onSuccess: () =>
        setAgentForm((current) => ({
          ...defaultAgentForm,
          providerId: current.providerId,
          modelId: current.modelId,
        })),
    });
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium">Create an agent</h3>
        <Badge variant="outline">{enabledProviders.length} providers</Badge>
      </div>
      {enabledProviders.length === 0 && (
        <div className="text-muted-foreground space-y-1 rounded-md border border-dashed p-3 text-sm">
          <p>
            Enable an AI provider before creating an agent — agents need a model to make their
            trading calls.
          </p>
          <Button asChild variant="link" size="sm" className="h-auto p-0 text-xs">
            <Link to="/settings/ai-providers">
              Open AI provider settings
              <Icons.ArrowRight className="ml-1 h-3 w-3" />
            </Link>
          </Button>
        </div>
      )}

      <div className="space-y-3">
        <h4 className="text-muted-foreground text-xs font-medium uppercase tracking-wide">
          Identity
        </h4>
        <Field label="Name">
          <Input
            value={agentForm.name}
            onChange={(event) => setAgentForm({ ...agentForm, name: event.target.value })}
            placeholder="OpenAI momentum"
          />
        </Field>
      </div>

      <div className="space-y-3">
        <h4 className="text-muted-foreground text-xs font-medium uppercase tracking-wide">Model</h4>
        <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
          <Field label="Provider">
            <Select
              value={agentForm.providerId}
              onValueChange={(providerId) => {
                const provider = enabledProviders.find((item) => item.id === providerId);
                setAgentForm({
                  ...agentForm,
                  providerId,
                  modelId: latestModelId(provider),
                });
              }}
            >
              <SelectTrigger>
                <SelectValue placeholder="None" />
              </SelectTrigger>
              <SelectContent>
                {enabledProviders.map((provider) => (
                  <SelectItem key={provider.id} value={provider.id}>
                    {provider.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
          <Field label="Model">
            <Select
              value={agentForm.modelId}
              onValueChange={(modelId) => setAgentForm({ ...agentForm, modelId })}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {providerModels.length === 0 && agentForm.modelId && (
                  <SelectItem value={agentForm.modelId}>{agentForm.modelId}</SelectItem>
                )}
                {providerModels.map((model) => (
                  <SelectItem key={model.id} value={model.id}>
                    {model.name ?? model.id}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
        </div>
        <p className="text-muted-foreground text-xs">
          Each agent thinks with one provider + model.
        </p>
      </div>

      <div className="space-y-3">
        <h4 className="text-muted-foreground text-xs font-medium uppercase tracking-wide">
          Persona
        </h4>
        <Field
          label="Persona"
          help="Instructions that shape how the agent trades — style, risk appetite, preferences."
        >
          <Textarea
            value={agentForm.persona}
            onChange={(event) => setAgentForm({ ...agentForm, persona: event.target.value })}
            className="min-h-24"
          />
        </Field>
      </div>

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
          <SwitchRow
            label="Active"
            description="Agent can be entered into challenges and run"
            checked={agentForm.enabled}
            onCheckedChange={(enabled) => setAgentForm({ ...agentForm, enabled })}
          />
          <SwitchRow
            label="Auto-run on schedule"
            description="Runs automatically at the challenge cadence"
            checked={agentForm.scheduleEnabled}
            onCheckedChange={(scheduleEnabled) => setAgentForm({ ...agentForm, scheduleEnabled })}
          />
        </CollapsibleContent>
      </Collapsible>

      <Button
        className="w-full"
        onClick={createAgent}
        disabled={!agentForm.name.trim() || !agentForm.providerId || !agentForm.modelId}
      >
        <Icons.Plus className="mr-2 h-4 w-4" />
        Add agent
      </Button>
    </div>
  );
}
