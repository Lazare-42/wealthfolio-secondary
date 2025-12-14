import React, { useState } from "react";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import * as z from "zod";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { Icons } from "@/components/ui/icons";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { toast } from "sonner";
import { getAiProviderConfig, setAiProviderConfig, setAiApiKey, hasAiApiKey, testAiConnection } from "@/commands/ai";

const providerFormSchema = z.object({
  provider: z.enum(['disabled', 'anthropic', 'openai', 'openrouter', 'ollama', 'mistral']),
  model: z.string().optional(),
  baseUrl: z.string().url().optional().or(z.literal('')),
  apiKey: z.string().optional(),
});

type ProviderFormValues = z.infer<typeof providerFormSchema>;

const PROVIDER_MODELS: Record<string, Array<{ value: string; label: string; recommended?: boolean }>> = {
  anthropic: [
    { value: 'claude-3-5-sonnet-20241022', label: 'Claude 3.5 Sonnet', recommended: true },
    { value: 'claude-3-5-haiku-20241022', label: 'Claude 3.5 Haiku (Faster/Cheaper)' },
  ],
  openai: [
    { value: 'gpt-4o', label: 'GPT-4o', recommended: true },
    { value: 'gpt-4o-mini', label: 'GPT-4o Mini (Cheaper)' },
  ],
  openrouter: [
    { value: 'anthropic/claude-3.5-sonnet', label: 'Claude 3.5 Sonnet' },
    { value: 'openai/gpt-4o', label: 'GPT-4o' },
    { value: 'google/gemini-pro', label: 'Gemini Pro' },
  ],
  ollama: [
    { value: 'llama3', label: 'Llama 3' },
    { value: 'mistral', label: 'Mistral' },
    { value: 'codellama', label: 'Code Llama' },
  ],
  mistral: [
    { value: 'mistral-large-latest', label: 'Mistral Large' },
    { value: 'mistral-medium-latest', label: 'Mistral Medium' },
  ],
};

export function ProviderSettings() {
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ success: boolean; message: string } | null>(null);
  const [keyExists, setKeyExists] = useState(false);

  const defaultValues: ProviderFormValues = {
    provider: 'disabled',
    model: undefined,
    baseUrl: '',
    apiKey: '',
  };

  const form = useForm<ProviderFormValues>({
    resolver: zodResolver(providerFormSchema),
    defaultValues,
  });

  const selectedProvider = form.watch('provider');
  const needsApiKey = selectedProvider !== 'disabled' && selectedProvider !== 'ollama';
  const needsBaseUrl = selectedProvider === 'ollama';

  // Load existing config on mount
  React.useEffect(() => {
    async function loadConfig() {
      try {
        const config = await getAiProviderConfig();
        form.reset({
          provider: config.type,
          model: config.model || undefined,
          baseUrl: config.baseUrl || '',
          apiKey: '',
        });

        // Check if API key exists for this provider
        if (config.type !== 'disabled' && config.type !== 'ollama') {
          const exists = await hasAiApiKey(config.type);
          setKeyExists(exists);
        }
      } catch (error) {
        console.error('Failed to load AI config:', error);
      }
    }
    loadConfig();
  }, [form]);

  async function onSubmit(data: ProviderFormValues) {
    try {
      await setAiProviderConfig({
        type: data.provider,
        model: data.model,
        baseUrl: data.baseUrl,
      });

      if (data.apiKey && needsApiKey) {
        await setAiApiKey(data.provider, data.apiKey);
        setKeyExists(true);
      }

      toast.success('AI settings saved successfully');
    } catch (error) {
      toast.error('Failed to save AI settings', {
        description: error instanceof Error ? error.message : 'Unknown error',
      });
    }
  }

  async function handleTestConnection() {
    setIsTesting(true);
    setTestResult(null);

    try {
      const result = await testAiConnection();
      setTestResult(result);

      if (result.success) {
        toast.success('Connection successful');
      } else {
        toast.error('Connection failed');
      }
    } catch (error) {
      setTestResult({
        success: false,
        message: error instanceof Error ? error.message : 'Connection test failed',
      });
    } finally {
      setIsTesting(false);
    }
  }

  const availableModels = selectedProvider !== 'disabled'
    ? PROVIDER_MODELS[selectedProvider] || []
    : [];

  return (
    <Card>
      <CardHeader>
        <CardTitle>AI Provider Configuration</CardTitle>
        <CardDescription>
          Choose and configure an AI provider for intelligent document parsing and CSV mapping
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-6">
            {/* Provider Selection */}
            <FormField
              control={form.control}
              name="provider"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>AI Provider</FormLabel>
                  <Select onValueChange={field.onChange} value={field.value}>
                    <FormControl>
                      <SelectTrigger>
                        <SelectValue placeholder="Select a provider" />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent>
                      <SelectItem value="disabled">Disabled</SelectItem>
                      <SelectItem value="anthropic">
                        <div className="flex items-center gap-2">
                          <span>Anthropic (Claude)</span>
                          <Badge variant="outline" className="text-[10px] px-1 py-0">Recommended</Badge>
                        </div>
                      </SelectItem>
                      <SelectItem value="openai">OpenAI (GPT-4)</SelectItem>
                      <SelectItem value="openrouter">OpenRouter (Multiple Models)</SelectItem>
                      <SelectItem value="ollama">Ollama (Local/Private)</SelectItem>
                      <SelectItem value="mistral">Mistral AI</SelectItem>
                    </SelectContent>
                  </Select>
                  <FormDescription>
                    {selectedProvider === 'disabled' && 'AI features will be unavailable'}
                    {selectedProvider === 'anthropic' && 'Best for PDF parsing with vision support'}
                    {selectedProvider === 'openai' && 'Reliable performance with good speed'}
                    {selectedProvider === 'openrouter' && 'Access multiple models through one API'}
                    {selectedProvider === 'ollama' && 'Run AI models locally for privacy'}
                    {selectedProvider === 'mistral' && 'European alternative with strong performance'}
                  </FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />

            {/* Model Selection */}
            {selectedProvider !== 'disabled' && availableModels.length > 0 && (
              <FormField
                control={form.control}
                name="model"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Model</FormLabel>
                    <Select onValueChange={field.onChange} value={field.value}>
                      <FormControl>
                        <SelectTrigger>
                          <SelectValue placeholder="Select a model" />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        {availableModels.map((model) => (
                          <SelectItem key={model.value} value={model.value}>
                            <div className="flex items-center gap-2">
                              <span>{model.label}</span>
                              {model.recommended && (
                                <Badge variant="outline" className="text-[10px] px-1 py-0">Recommended</Badge>
                              )}
                            </div>
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <FormDescription>
                      Choose the AI model to use for processing
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
            )}

            {/* Base URL for Ollama */}
            {needsBaseUrl && (
              <FormField
                control={form.control}
                name="baseUrl"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Base URL</FormLabel>
                    <FormControl>
                      <Input placeholder="http://localhost:11434" {...field} />
                    </FormControl>
                    <FormDescription>
                      The URL where your Ollama instance is running
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
            )}

            {/* API Key Input */}
            {needsApiKey && (
              <FormField
                control={form.control}
                name="apiKey"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>API Key</FormLabel>
                    <FormControl>
                      <div className="flex gap-2">
                        <Input
                          type="password"
                          placeholder="Enter your API key"
                          {...field}
                          className="flex-1"
                        />
                        {keyExists && (
                          <div className="flex items-center text-sm text-green-600">
                            <Icons.CheckCircle className="h-4 w-4" />
                          </div>
                        )}
                      </div>
                    </FormControl>
                    <FormDescription>
                      Your API key is stored securely in the system keyring
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
            )}

            {/* Test Connection */}
            {selectedProvider !== 'disabled' && (
              <div className="space-y-4">
                <Button
                  type="button"
                  variant="outline"
                  onClick={handleTestConnection}
                  disabled={isTesting}
                >
                  {isTesting ? (
                    <>
                      <Icons.Loader className="mr-2 h-4 w-4 animate-spin" />
                      Testing...
                    </>
                  ) : (
                    <>
                      <Icons.Sparkles className="mr-2 h-4 w-4" />
                      Test Connection
                    </>
                  )}
                </Button>

                {testResult && (
                  <Alert variant={testResult.success ? 'default' : 'destructive'}>
                    <AlertTitle>
                      {testResult.success ? 'Connection Successful' : 'Connection Failed'}
                    </AlertTitle>
                    <AlertDescription>{testResult.message}</AlertDescription>
                  </Alert>
                )}
              </div>
            )}

            <Button type="submit">Save Configuration</Button>
          </form>
        </Form>
      </CardContent>
    </Card>
  );
}
