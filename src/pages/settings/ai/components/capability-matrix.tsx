import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Icons } from "@/components/ui/icons";

export function CapabilityMatrix() {
  const capabilities = [
    {
      feature: 'CSV Smart Mapping',
      description: 'Automatically suggest column mappings for CSV imports',
      anthropic: true,
      openai: true,
      openrouter: true,
      ollama: true,
      mistral: true,
    },
    {
      feature: 'PDF Document Parsing',
      description: 'Extract transactions from PDF bank statements',
      anthropic: true,
      openai: false,
      openrouter: 'varies',
      ollama: false,
      mistral: false,
    },
    {
      feature: 'Image/Screenshot Parsing',
      description: 'Extract data from images and screenshots',
      anthropic: true,
      openai: true,
      openrouter: 'varies',
      ollama: false,
      mistral: false,
    },
    {
      feature: 'Excel File Parsing',
      description: 'Process Excel files (convert to PDF first)',
      anthropic: 'via-pdf',
      openai: false,
      openrouter: 'varies',
      ollama: false,
      mistral: false,
    },
  ];

  const renderCapabilityBadge = (value: boolean | string) => {
    if (value === true) {
      return (
        <Badge variant="default" className="gap-1">
          <Icons.Check className="h-3 w-3" />
          Supported
        </Badge>
      );
    }
    if (value === 'varies') {
      return (
        <Badge variant="secondary" className="gap-1">
          <Icons.Info className="h-3 w-3" />
          Model Dependent
        </Badge>
      );
    }
    if (value === 'via-pdf') {
      return (
        <Badge variant="secondary" className="gap-1">
          <Icons.FileText className="h-3 w-3" />
          Via PDF
        </Badge>
      );
    }
    return (
      <Badge variant="outline" className="gap-1">
        <Icons.MinusCircle className="h-3 w-3" />
        Not Supported
      </Badge>
    );
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Provider Capabilities</CardTitle>
        <CardDescription>
          Compare features across different AI providers
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="overflow-x-auto">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-[200px]">Feature</TableHead>
                <TableHead>Anthropic</TableHead>
                <TableHead>OpenAI</TableHead>
                <TableHead>OpenRouter</TableHead>
                <TableHead>Ollama</TableHead>
                <TableHead>Mistral</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {capabilities.map((capability) => (
                <TableRow key={capability.feature}>
                  <TableCell>
                    <div>
                      <div className="font-medium">{capability.feature}</div>
                      <div className="text-xs text-muted-foreground">
                        {capability.description}
                      </div>
                    </div>
                  </TableCell>
                  <TableCell>{renderCapabilityBadge(capability.anthropic)}</TableCell>
                  <TableCell>{renderCapabilityBadge(capability.openai)}</TableCell>
                  <TableCell>{renderCapabilityBadge(capability.openrouter)}</TableCell>
                  <TableCell>{renderCapabilityBadge(capability.ollama)}</TableCell>
                  <TableCell>{renderCapabilityBadge(capability.mistral)}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>

        <div className="mt-6 space-y-2 text-sm text-muted-foreground">
          <p className="font-medium">Notes:</p>
          <ul className="list-disc list-inside space-y-1">
            <li>
              <strong>Anthropic</strong>: Best for PDF parsing with native vision support
            </li>
            <li>
              <strong>OpenAI</strong>: Reliable performance, no PDF support (images only)
            </li>
            <li>
              <strong>OpenRouter</strong>: Access multiple models, capabilities vary by model
            </li>
            <li>
              <strong>Ollama</strong>: Local/private deployment, limited to text processing
            </li>
            <li>
              <strong>Mistral</strong>: European alternative, text processing only
            </li>
          </ul>
        </div>
      </CardContent>
    </Card>
  );
}
