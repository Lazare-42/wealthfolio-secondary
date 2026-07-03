import { useEffect, useState } from "react";

import type { useAiArenaMutations } from "@/hooks/use-ai-arena";
import type { CreateArenaAgentRequest, MergedProvider } from "@/lib/types";
import { Icons } from "@wealthfolio/ui";
import { Badge } from "@wealthfolio/ui/components/ui/badge";
import { Button } from "@wealthfolio/ui/components/ui/button";
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
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium">Agent</h3>
        <Badge variant="outline">{enabledProviders.length} providers</Badge>
      </div>
      {enabledProviders.length === 0 && (
        <div className="text-muted-foreground rounded-md border border-dashed p-3 text-sm">
          Enable an AI provider in Settings → AI before creating an agent. Each agent uses one
          provider + model to make its trading calls.
        </div>
      )}
      <Field label="Name">
        <Input
          value={agentForm.name}
          onChange={(event) => setAgentForm({ ...agentForm, name: event.target.value })}
          placeholder="OpenAI momentum"
        />
      </Field>
      <div className="grid grid-cols-2 gap-2">
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
      <Field label="Persona">
        <Textarea
          value={agentForm.persona}
          onChange={(event) => setAgentForm({ ...agentForm, persona: event.target.value })}
          className="min-h-24"
        />
      </Field>
      <div className="flex items-center justify-between gap-3">
        <SwitchRow
          label="Enabled"
          checked={agentForm.enabled}
          onCheckedChange={(enabled) => setAgentForm({ ...agentForm, enabled })}
        />
        <SwitchRow
          label="Scheduled"
          checked={agentForm.scheduleEnabled}
          onCheckedChange={(scheduleEnabled) => setAgentForm({ ...agentForm, scheduleEnabled })}
        />
      </div>
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
