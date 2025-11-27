# AI-Powered Document Import - Implementation Plan

## Status: Backend Architecture Complete ✅

This document outlines the **frontend implementation plan** for AI-powered document import and intelligent CSV mapping.

---

## Overview

**Goal:** Enable users to import financial transactions from ANY document type (PDF, images, Excel, CSV) using AI to automatically parse and map data.

**Backend Complete:**
- ✅ AI provider abstraction (OpenAI, Anthropic, OpenRouter, Ollama, Mistral)
- ✅ Business logic centralized in `src-core` for code sharing
- ✅ Tauri commands using thin wrappers (keyring storage)
- ✅ Architecture ready for web server integration
- ✅ CSV column mapping with schema-driven prompts
- ✅ Document parsing commands (multimodal support)
- ✅ Settings persistence and API key management

**Frontend TODO:**
- ⏳ Settings UI for AI configuration
- ⏳ Import wizard enhancement
- ⏳ Document upload handling
- ⏳ Provider capability detection
- ⏳ Error handling and user feedback

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Frontend                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Settings UI (AI Configuration)                            │
│  ├─ Provider Selection                                     │
│  ├─ Model Selection                                        │
│  ├─ API Key Management                                     │
│  └─ Test Connection                                        │
│                                                             │
│  Import Wizard Enhancement                                 │
│  ├─ CSV Upload (existing) + AI Smart Mapping              │
│  └─ Document Upload (new) → AI Parse → Preview → Import   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                    ↓ Tauri Commands / HTTP API
┌─────────────────────────────────────────────────────────────┐
│                  Platform Layer (src-tauri)                 │
├─────────────────────────────────────────────────────────────┤
│  Tauri Commands (thin wrappers):                           │
│  ├─ get_ai_provider_config()                               │
│  ├─ set_ai_provider_config()                               │
│  ├─ set_ai_api_key()         ← Keyring storage             │
│  ├─ has_ai_api_key()         ← Keyring storage             │
│  ├─ test_ai_connection()                                   │
│  ├─ suggest_csv_column_mapping()                           │
│  └─ parse_financial_document()                             │
│                                                             │
│  Secret Store: KeyringSecretStore (macOS/Windows/Linux)    │
└─────────────────────────────────────────────────────────────┘
                              ↓ calls
┌─────────────────────────────────────────────────────────────┐
│              Core Business Logic (src-core) ✅              │
├─────────────────────────────────────────────────────────────┤
│  src-core/src/ai/                                           │
│  ├─ mod.rs                                                  │
│  │   ├─ AIProviderConfig (enum)                            │
│  │   └─ create_client(&SecretStore) → AiClient             │
│  │                                                          │
│  └─ csv_mapper.rs                                           │
│      ├─ suggest_column_mappings()                          │
│      ├─ build_mapping_prompt() (schema-driven)             │
│      └─ parse_mapping_response()                           │
│                                                             │
│  Providers: Anthropic | OpenAI | OpenRouter | Ollama |     │
│             Mistral                                         │
│                                                             │
│  Uses: ai-lib SDK, schemars (JSON Schema generation)       │
└─────────────────────────────────────────────────────────────┘
                              ↓ can be used by
┌─────────────────────────────────────────────────────────────┐
│           Web Server Layer (src-server) ✅                  │
├─────────────────────────────────────────────────────────────┤
│  HTTP Endpoints (thin wrappers):                           │
│  POST /api/v1/ai/suggest-mapping         ✅                │
│  POST /api/v1/ai/parse-document          (planned)         │
│                                                             │
│  Secret Store: FileSecretStore (encrypted JSON)            │
│  Calls: wealthfolio_core::ai::* (same core logic)          │
└─────────────────────────────────────────────────────────────┘
```

### Architecture Pattern

**Code Organization:**
1. **Business logic** → `src-core` (shared)
2. **Tauri commands** → `src-tauri` (thin wrappers + KeyringSecretStore)
3. **Web API handlers** → `src-server` (thin wrappers + FileSecretStore)

**Benefits:**
- ✅ Zero duplication - AI logic implemented once in core
- ✅ Platform-specific code isolated (keyring vs file storage)
- ✅ Easy testing - core logic testable independently
- ✅ Web/desktop parity - same AI behavior everywhere

---

## Phase 1: Settings UI (AI Configuration)

### Location
`src/pages/settings/ai/ai-settings-page.tsx`

### Features

#### 1. Provider Selection
```tsx
<Select value={provider} onValueChange={handleProviderChange}>
  <SelectTrigger>
    <SelectValue placeholder="Select AI Provider" />
  </SelectTrigger>
  <SelectContent>
    <SelectItem value="disabled">Disabled</SelectItem>
    <SelectItem value="anthropic">
      <div className="flex items-center gap-2">
        <Badge variant="outline">Recommended</Badge>
        Anthropic (Claude)
      </div>
    </SelectItem>
    <SelectItem value="openai">OpenAI (GPT-4)</SelectItem>
    <SelectItem value="openrouter">OpenRouter (Multiple Models)</SelectItem>
    <SelectItem value="ollama">Ollama (Local/Private)</SelectItem>
  </SelectContent>
</Select>
```

**Implementation:**
- Load current config on mount via `getAiProviderConfig()`
- Show/hide API key input based on provider
- Display provider-specific settings (Ollama base URL, etc.)

#### 2. Model Selection (Provider-Specific)
```tsx
// Anthropic Models
const anthropicModels = [
  { value: 'claude-3-5-sonnet-20241022', label: 'Claude 3.5 Sonnet', recommended: true },
  { value: 'claude-3-5-haiku-20241022', label: 'Claude 3.5 Haiku (Faster/Cheaper)' },
];

// OpenAI Models
const openaiModels = [
  { value: 'gpt-4o', label: 'GPT-4o', recommended: true },
  { value: 'gpt-4o-mini', label: 'GPT-4o Mini (Cheaper)' },
];

// OpenRouter - Dynamic list (fetch from OpenRouter API or hardcode popular ones)
```

#### 3. API Key Management
```tsx
<Card>
  <CardHeader>
    <CardTitle>API Key Configuration</CardTitle>
    <CardDescription>
      Your API key is stored securely in the system keyring
    </CardDescription>
  </CardHeader>
  <CardContent>
    <div className="space-y-4">
      <div className="flex gap-2">
        <Input
          type="password"
          placeholder="Enter API Key"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
        />
        <Button onClick={handleSaveKey} disabled={!apiKey}>
          Save
        </Button>
      </div>

      {keyExists && (
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <CheckCircle className="h-4 w-4 text-green-500" />
          API key configured
        </div>
      )}
    </div>
  </CardContent>
</Card>
```

**Backend calls:**
```typescript
await setAiApiKey(provider, apiKey);  // Stores in keyring
const exists = await hasAiApiKey(provider);  // Check if key exists
```

#### 4. Connection Test
```tsx
<Button
  onClick={handleTestConnection}
  disabled={!keyExists || testing}
  variant="outline"
>
  {testing ? (
    <>
      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
      Testing...
    </>
  ) : (
    <>
      <Zap className="mr-2 h-4 w-4" />
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
```

**Backend call:**
```typescript
const result = await testAiConnection();
// Returns: { success: boolean, message: string, model_used?: string }
```

#### 5. Provider Capabilities Display
```tsx
<Card>
  <CardHeader>
    <CardTitle>Provider Capabilities</CardTitle>
  </CardHeader>
  <CardContent>
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Feature</TableHead>
          <TableHead>Status</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        <TableRow>
          <TableCell>CSV Smart Mapping</TableCell>
          <TableCell>
            <Badge variant="success">All Providers</Badge>
          </TableCell>
        </TableRow>
        <TableRow>
          <TableCell>PDF Document Parsing</TableCell>
          <TableCell>
            {provider === 'anthropic' ? (
              <Badge variant="success">Supported</Badge>
            ) : (
              <Badge variant="secondary">Not Supported</Badge>
            )}
          </TableCell>
        </TableRow>
        <TableRow>
          <TableCell>Image/Screenshot Parsing</TableCell>
          <TableCell>
            {['anthropic', 'openai', 'openrouter'].includes(provider) ? (
              <Badge variant="success">Supported</Badge>
            ) : (
              <Badge variant="secondary">Not Supported</Badge>
            )}
          </TableCell>
        </TableRow>
        <TableRow>
          <TableCell>Excel File Parsing</TableCell>
          <TableCell>
            <Badge variant="warning">Convert to PDF First</Badge>
          </TableCell>
        </TableRow>
      </TableBody>
    </Table>
  </CardContent>
</Card>
```

#### 6. Cost Estimator (Optional but Nice)
```tsx
<Card>
  <CardHeader>
    <CardTitle>Estimated Costs</CardTitle>
    <CardDescription>
      Approximate costs per document parsed
    </CardDescription>
  </CardHeader>
  <CardContent>
    <div className="space-y-2">
      {provider === 'anthropic' && (
        <div className="flex justify-between">
          <span>5-page PDF:</span>
          <span className="font-mono">~$0.10 - $0.20</span>
        </div>
      )}
      {provider === 'openai' && (
        <div className="flex justify-between">
          <span>Screenshot:</span>
          <span className="font-mono">~$0.05 - $0.10</span>
        </div>
      )}
      {provider === 'ollama' && (
        <Badge variant="success">Free (Local)</Badge>
      )}
    </div>
  </CardContent>
</Card>
```

### Files to Create

```
src/pages/settings/ai/
├── ai-settings-page.tsx           # Main settings page
├── components/
│   ├── provider-selector.tsx      # Provider dropdown
│   ├── model-selector.tsx         # Model dropdown
│   ├── api-key-input.tsx          # Secure key input
│   ├── connection-tester.tsx      # Test connection button
│   └── capability-matrix.tsx      # Feature support table
└── hooks/
    ├── use-ai-config.ts            # Load/save config
    └── use-ai-test.ts              # Test connection logic
```

### Navigation Integration

Add to settings navigation:
```tsx
// src/routes.tsx or settings navigation
{
  path: '/settings/ai',
  label: 'AI & Automation',
  icon: <Sparkles />,
  component: AISettingsPage,
}
```

---

## Phase 2: Import Wizard Enhancement

### Location
`src/pages/activity/import/`

### Features

#### 1. Detect AI Configuration Status

```tsx
// src/pages/activity/import/hooks/use-ai-status.ts

export function useAIStatus() {
  const [aiEnabled, setAiEnabled] = useState(false);
  const [provider, setProvider] = useState<string | null>(null);
  const [capabilities, setCapabilities] = useState({
    csvMapping: false,
    pdfParsing: false,
    imageParsing: false,
  });

  useEffect(() => {
    async function checkAI() {
      const config = await getAiProviderConfig();

      if (config.type === 'disabled') {
        setAiEnabled(false);
        return;
      }

      setAiEnabled(true);
      setProvider(config.type);

      // Set capabilities based on provider
      setCapabilities({
        csvMapping: true,  // All providers support this
        pdfParsing: config.type === 'anthropic',
        imageParsing: ['anthropic', 'openai', 'openrouter'].includes(config.type),
      });
    }

    checkAI();
  }, []);

  return { aiEnabled, provider, capabilities };
}
```

#### 2. Enhanced Account Selection Step

**File:** `src/pages/activity/import/steps/account-selection-step.tsx`

```tsx
export function AccountSelectionStep({ ... }) {
  const { aiEnabled, capabilities } = useAIStatus();
  const [uploadMode, setUploadMode] = useState<'csv' | 'document' | null>(null);

  return (
    <div className="space-y-6">
      {/* Account Selection (existing) */}
      <Card>
        <CardHeader>
          <CardTitle>Select Account</CardTitle>
        </CardHeader>
        <CardContent>
          <Select value={selectedAccount?.id} onValueChange={handleAccountSelect}>
            {/* Account options */}
          </Select>
        </CardContent>
      </Card>

      {/* Upload Mode Selection */}
      <div className="grid gap-4 md:grid-cols-2">
        {/* CSV Upload (existing + AI enhancement) */}
        <Card
          className={cn(
            "cursor-pointer transition-all hover:border-primary",
            uploadMode === 'csv' && "border-primary bg-accent"
          )}
          onClick={() => setUploadMode('csv')}
        >
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="text-base">CSV Import</CardTitle>
              {aiEnabled && (
                <Badge variant="secondary" className="gap-1">
                  <Sparkles className="h-3 w-3" />
                  AI Enhanced
                </Badge>
              )}
            </div>
            <CardDescription>
              Upload CSV files with automatic column mapping
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <FileText className="h-4 w-4" />
              .csv files
            </div>
          </CardContent>
        </Card>

        {/* Document Upload (new) */}
        <Card
          className={cn(
            "cursor-pointer transition-all",
            aiEnabled && capabilities.pdfParsing
              ? "hover:border-primary"
              : "opacity-50 cursor-not-allowed",
            uploadMode === 'document' && "border-primary bg-accent"
          )}
          onClick={() => aiEnabled && setUploadMode('document')}
        >
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="text-base">Document Parsing</CardTitle>
              {aiEnabled ? (
                <Badge variant="default" className="gap-1">
                  <Sparkles className="h-3 w-3" />
                  AI Powered
                </Badge>
              ) : (
                <Badge variant="outline">Requires AI</Badge>
              )}
            </div>
            <CardDescription>
              {aiEnabled
                ? "Parse any financial document with AI"
                : "Configure AI in settings to enable"
              }
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-1">
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <FileText className="h-4 w-4" />
                PDFs, Images, Screenshots
              </div>
              {!aiEnabled && (
                <Button
                  variant="link"
                  size="sm"
                  className="h-auto p-0"
                  onClick={(e) => {
                    e.stopPropagation();
                    navigate('/settings/ai');
                  }}
                >
                  Configure AI Provider →
                </Button>
              )}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* File Upload Zone */}
      {uploadMode === 'csv' && (
        <Card>
          <CardContent className="pt-6">
            <FileDropzone
              accept=".csv"
              onFileSelect={handleCsvUpload}
              isParsing={isParsing}
            >
              <div className="text-center">
                <FileText className="mx-auto h-12 w-12 text-muted-foreground" />
                <p className="mt-2 text-sm font-medium">Drop CSV file here</p>
                <p className="text-xs text-muted-foreground">
                  {aiEnabled ? 'AI will suggest column mappings' : 'or click to browse'}
                </p>
              </div>
            </FileDropzone>
          </CardContent>
        </Card>
      )}

      {uploadMode === 'document' && aiEnabled && (
        <Card>
          <CardContent className="pt-6">
            <FileDropzone
              accept=".pdf,.png,.jpg,.jpeg,.webp"
              onFileSelect={handleDocumentUpload}
              isParsing={isAiParsing}
            >
              <div className="text-center">
                <Sparkles className="mx-auto h-12 w-12 text-primary" />
                <p className="mt-2 text-sm font-medium">Drop document here</p>
                <p className="text-xs text-muted-foreground">
                  Bank statements, brokerage reports, loan schedules
                </p>
                <div className="mt-4 flex justify-center gap-2">
                  <Badge variant="outline">PDF</Badge>
                  <Badge variant="outline">PNG</Badge>
                  <Badge variant="outline">JPG</Badge>
                  <Badge variant="outline">WEBP</Badge>
                </div>
              </div>
            </FileDropzone>
          </CardContent>
        </Card>
      )}

      {/* Action Buttons */}
      <div className="flex justify-between">
        <Button variant="outline" onClick={onBack}>
          Cancel
        </Button>
        <Button
          onClick={onNext}
          disabled={!selectedAccount || (!selectedFile && uploadMode === 'csv')}
        >
          Next
        </Button>
      </div>
    </div>
  );
}
```

#### 3. Document Upload Handler

```tsx
// src/pages/activity/import/hooks/use-document-parser.ts

export function useDocumentParser() {
  const [isAiParsing, setIsAiParsing] = useState(false);
  const [parseError, setParseError] = useState<string | null>(null);

  const parseDocument = async (file: File): Promise<ActivityImport[]> => {
    setIsAiParsing(true);
    setParseError(null);

    try {
      // 1. Validate file
      const maxSize = 32 * 1024 * 1024; // 32MB
      if (file.size > maxSize) {
        throw new Error('File too large. Maximum size is 32MB.');
      }

      // 2. Read file as bytes
      const arrayBuffer = await file.arrayBuffer();
      const fileBytes = Array.from(new Uint8Array(arrayBuffer));

      // 3. Determine document type hint
      const documentType = inferDocumentType(file.name);

      // 4. Call backend
      const result = await parseFinancialDocument({
        file_bytes: fileBytes,
        file_type: file.type || inferMimeType(file.name),
        document_type: documentType,
      });

      return result;

    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      setParseError(message);
      throw error;
    } finally {
      setIsAiParsing(false);
    }
  };

  return { parseDocument, isAiParsing, parseError };
}

function inferDocumentType(filename: string): string {
  const lower = filename.toLowerCase();
  if (lower.includes('loan') || lower.includes('mortgage') || lower.includes('amortization')) {
    return 'loan_schedule';
  }
  if (lower.includes('statement') || lower.includes('bank')) {
    return 'bank_statement';
  }
  if (lower.includes('broker') || lower.includes('trading')) {
    return 'brokerage_report';
  }
  return 'auto';
}

function inferMimeType(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase();
  const mimeTypes: Record<string, string> = {
    pdf: 'application/pdf',
    png: 'image/png',
    jpg: 'image/jpeg',
    jpeg: 'image/jpeg',
    webp: 'image/webp',
  };
  return mimeTypes[ext || ''] || 'application/octet-stream';
}
```

#### 4. AI Parsing Progress Indicator

```tsx
// src/pages/activity/import/components/ai-parsing-overlay.tsx

export function AiParsingOverlay({ visible }: { visible: boolean }) {
  if (!visible) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm">
      <Card className="w-[400px]">
        <CardHeader>
          <div className="flex items-center gap-2">
            <Sparkles className="h-5 w-5 animate-pulse text-primary" />
            <CardTitle>AI Processing Document</CardTitle>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <Loader2 className="h-4 w-4 animate-spin" />
              <span className="text-sm">Analyzing document structure...</span>
            </div>
            <Progress value={33} />
          </div>

          <p className="text-xs text-muted-foreground">
            This may take 10-30 seconds depending on document complexity
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
```

#### 5. Smart CSV Mapping Integration

**File:** `src/pages/activity/import/steps/mapping-step.tsx`

Add AI suggestion button:

```tsx
export function MappingStep({ headers, data, onNext, onBack }: Props) {
  const [mapping, setMapping] = useState<ImportMappingData>({ ... });
  const [isLoadingAI, setIsLoadingAI] = useState(false);
  const { aiEnabled } = useAIStatus();

  const handleAISuggest = async () => {
    setIsLoadingAI(true);
    try {
      const sampleRows = data.slice(0, 3).map(row => {
        const obj: Record<string, string> = {};
        headers.forEach((header, i) => {
          obj[header] = row[i] || '';
        });
        return obj;
      });

      const result = await suggestCsvColumnMapping({
        headers,
        sample_rows: sampleRows,
      });

      if (result.success && result.suggestions) {
        // Apply suggestions to mapping
        const newMapping = { ...mapping };
        result.suggestions.suggestions.forEach(suggestion => {
          if (suggestion.confidence > 0.7) {
            newMapping.fieldMappings[suggestion.field] = suggestion.suggested_column;
          }
        });
        setMapping(newMapping);

        toast.success('AI suggestions applied', {
          description: `Mapped ${result.suggestions.suggestions.length} fields`
        });
      }
    } catch (error) {
      toast.error('AI suggestion failed', {
        description: error instanceof Error ? error.message : 'Unknown error'
      });
    } finally {
      setIsLoadingAI(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">Map CSV Columns</h2>
          <p className="text-sm text-muted-foreground">
            Match your CSV columns to Wealthfolio fields
          </p>
        </div>

        {aiEnabled && (
          <Button
            variant="outline"
            size="sm"
            onClick={handleAISuggest}
            disabled={isLoadingAI}
            className="gap-2"
          >
            {isLoadingAI ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin" />
                Analyzing...
              </>
            ) : (
              <>
                <Sparkles className="h-4 w-4" />
                AI Suggest Mappings
              </>
            )}
          </Button>
        )}
      </div>

      {/* Rest of mapping UI */}
      <MappingTable
        headers={headers}
        mapping={mapping}
        onChange={setMapping}
      />

      {/* Action buttons */}
    </div>
  );
}
```

---

## Phase 3: Error Handling & User Feedback

### Common Error Scenarios

#### 1. AI Not Configured
```tsx
<Alert variant="warning">
  <AlertTriangle className="h-4 w-4" />
  <AlertTitle>AI Provider Not Configured</AlertTitle>
  <AlertDescription>
    To use document parsing, configure an AI provider in settings.
    <Button variant="link" onClick={() => navigate('/settings/ai')}>
      Configure Now →
    </Button>
  </AlertDescription>
</Alert>
```

#### 2. Unsupported File Type
```tsx
if (!capabilities.pdfParsing && file.type === 'application/pdf') {
  toast.error('PDF not supported', {
    description: `${provider} doesn't support PDF parsing. Please use Anthropic or convert to images.`
  });
  return;
}
```

#### 3. API Key Invalid/Missing
```tsx
<Alert variant="destructive">
  <XCircle className="h-4 w-4" />
  <AlertTitle>API Key Error</AlertTitle>
  <AlertDescription>
    Your API key appears to be invalid or missing. Please check your configuration.
    <Button variant="link" onClick={() => navigate('/settings/ai')}>
      Update API Key →
    </Button>
  </AlertDescription>
</Alert>
```

#### 4. Rate Limit Exceeded
```tsx
if (error.message.includes('rate limit')) {
  toast.error('Rate Limit Exceeded', {
    description: 'Please wait a few minutes before trying again.',
    duration: 5000,
  });
}
```

#### 5. Parsing Failed (AI couldn't extract data)
```tsx
<Alert variant="warning">
  <AlertTriangle className="h-4 w-4" />
  <AlertTitle>No Transactions Found</AlertTitle>
  <AlertDescription>
    The AI couldn't detect any financial transactions in this document.
    <div className="mt-2 space-y-1">
      <p className="text-sm font-medium">Possible reasons:</p>
      <ul className="text-sm list-disc list-inside">
        <li>Document quality is too low</li>
        <li>Document is not a financial statement</li>
        <li>Format not recognized</li>
      </ul>
    </div>
    <Button variant="outline" size="sm" className="mt-2" onClick={tryAgain}>
      Try Another Document
    </Button>
  </AlertDescription>
</Alert>
```

### Success Feedback

```tsx
// After successful document parse
toast.success('Document Parsed Successfully', {
  description: `Found ${transactions.length} transactions. Review and import below.`,
  duration: 3000,
});

// After successful CSV mapping suggestion
toast.success('AI Mapping Applied', {
  description: `${suggestedCount} fields mapped with high confidence. Please review.`,
  action: {
    label: 'Review',
    onClick: () => scrollToMapping(),
  },
});
```

---

## Phase 4: Testing & Validation

### Test Documents Needed

1. **Bank Statements**
   - PDF: Chase, Bank of America, Wells Fargo formats
   - Images: Screenshots of mobile banking apps

2. **Brokerage Reports**
   - PDF: Interactive Brokers, TD Ameritrade, Robinhood
   - CSV: Degiro, Schwab exports

3. **Loan Schedules**
   - PDF: Mortgage amortization schedules
   - Excel: Car loan payment schedules (converted to PDF)

4. **Edge Cases**
   - Multi-page PDFs
   - Low-quality scans
   - Mixed languages
   - Complex formats (merged statements)

### Test Scenarios

#### Scenario 1: Happy Path - PDF Bank Statement
```
1. User configures Anthropic in settings
2. User navigates to import wizard
3. User selects "Document Parsing"
4. User drops PDF bank statement
5. AI parses → Extracts 15 transactions
6. Preview shows all transactions correctly
7. User clicks Import → Success
```

#### Scenario 2: CSV Smart Mapping
```
1. User has AI configured (any provider)
2. User uploads CSV with custom headers
3. Mapping step shows → User clicks "AI Suggest"
4. AI suggests 8/10 mappings with >90% confidence
5. User reviews, adjusts 2 mappings manually
6. User proceeds to import → Success
```

#### Scenario 3: Loan Schedule Import
```
1. User uploads loan amortization PDF
2. AI detects loan pattern
3. Extracts: Initial principal, monthly payments, interest
4. Sets negative quantities automatically
5. User reviews, confirms loan details
6. Import creates loan activities correctly
```

#### Scenario 4: Error Recovery
```
1. User attempts PDF parse without AI configured
2. Shows error → "Configure AI Provider"
3. User clicks link → Navigates to settings
4. User configures Anthropic + API key
5. User returns to import → Tries again → Success
```

---

## Phase 5: Documentation

### User Documentation Needed

1. **Settings Page Help**
   - How to get API keys (link to provider docs)
   - Cost estimates and billing info
   - Privacy considerations (what data is sent)

2. **Import Wizard Help**
   - When to use CSV vs Document parsing
   - Supported file formats
   - How to prepare documents for best results

3. **Troubleshooting Guide**
   - Common errors and solutions
   - Document quality tips
   - Provider selection guidance

### In-App Help Components

```tsx
// Help popover for document upload
<HelpPopover>
  <HelpPopoverTrigger>
    <Button variant="ghost" size="icon">
      <HelpCircle className="h-4 w-4" />
    </Button>
  </HelpPopoverTrigger>
  <HelpPopoverContent>
    <div className="space-y-2">
      <p className="font-medium">Document Parsing Tips</p>
      <ul className="text-sm list-disc list-inside space-y-1">
        <li>Use high-quality scans or original PDFs</li>
        <li>Ensure text is readable (not handwritten)</li>
        <li>Multi-page documents are supported</li>
        <li>Processing takes 10-30 seconds</li>
      </ul>
    </div>
  </HelpPopoverContent>
</HelpPopover>
```

---

## Commands Reference (Backend → Frontend)

### TypeScript Bindings

```typescript
// src/commands/ai-import.ts

export interface AIProviderConfig {
  type: 'anthropic' | 'openai' | 'openrouter' | 'ollama' | 'disabled';
  model?: string;
}

export interface MappingSuggestion {
  field: string;
  suggested_column: string;
  confidence: number;
}

export interface ParseDocumentRequest {
  file_bytes: number[];
  file_type: string;
  document_type: 'bank_statement' | 'brokerage_report' | 'loan_schedule' | 'auto';
}

// Get current AI configuration
export async function getAiProviderConfig(): Promise<AIProviderConfig> {
  return invokeTauri('get_ai_provider_config');
}

// Update AI configuration
export async function setAiProviderConfig(config: AIProviderConfig): Promise<void> {
  return invokeTauri('set_ai_provider_config', { newConfig: config });
}

// Store API key securely
export async function setAiApiKey(provider: string, key: string): Promise<void> {
  return invokeTauri('set_ai_api_key', { provider, key });
}

// Check if API key exists
export async function hasAiApiKey(provider: string): Promise<boolean> {
  return invokeTauri('has_ai_api_key', { provider });
}

// Test connection
export async function testAiConnection(): Promise<{
  success: boolean;
  message: string;
  model_used?: string;
}> {
  return invokeTauri('test_ai_connection');
}

// Suggest CSV column mappings
export async function suggestCsvColumnMapping(request: {
  headers: string[];
  sample_rows: Record<string, string>[];
}): Promise<{
  success: boolean;
  suggestions?: { suggestions: MappingSuggestion[] };
  error?: string;
}> {
  return invokeTauri('suggest_csv_column_mapping', { request });
}

// Parse financial document
export async function parseFinancialDocument(
  request: ParseDocumentRequest
): Promise<ActivityImport[]> {
  return invokeTauri('parse_financial_document', { request });
}
```

---

## File Structure Summary

```
src/
├── commands/
│   └── ai-import.ts                      # ✅ Backend command bindings
│
├── pages/
│   ├── settings/
│   │   └── ai/
│   │       ├── ai-settings-page.tsx       # ⏳ Main settings page
│   │       ├── components/
│   │       │   ├── provider-selector.tsx  # ⏳ Provider dropdown
│   │       │   ├── model-selector.tsx     # ⏳ Model dropdown
│   │       │   ├── api-key-input.tsx      # ⏳ Secure input
│   │       │   ├── connection-tester.tsx  # ⏳ Test button
│   │       │   └── capability-matrix.tsx  # ⏳ Feature table
│   │       └── hooks/
│   │           ├── use-ai-config.ts       # ⏳ Config management
│   │           └── use-ai-test.ts         # ⏳ Test logic
│   │
│   └── activity/
│       └── import/
│           ├── steps/
│           │   └── account-selection-step.tsx  # 🔄 Enhanced with AI
│           │   └── mapping-step.tsx            # 🔄 Add AI suggest
│           ├── components/
│           │   └── ai-parsing-overlay.tsx      # ⏳ Loading state
│           └── hooks/
│               ├── use-ai-status.ts            # ⏳ Check AI config
│               └── use-document-parser.ts      # ⏳ Document upload
│
└── routes.tsx                             # 🔄 Add AI settings route
```

**Legend:**
- ✅ Done (backend command bindings)
- ⏳ TODO (frontend implementation)
- 🔄 Modify (enhance existing)

---

## Implementation Checklist

### Settings UI
- [ ] Create `ai-settings-page.tsx`
- [ ] Build provider selector component
- [ ] Build model selector component
- [ ] Build API key input component
- [ ] Build connection tester component
- [ ] Build capability matrix display
- [ ] Create `use-ai-config` hook
- [ ] Create `use-ai-test` hook
- [ ] Add route to settings navigation
- [ ] Add help documentation

### Import Wizard
- [ ] Create `use-ai-status` hook
- [ ] Enhance account-selection-step with document upload
- [ ] Create `use-document-parser` hook
- [ ] Create AI parsing overlay component
- [ ] Add AI suggest button to mapping step
- [ ] Update file dropzone to accept multiple types
- [ ] Add document type inference logic
- [ ] Wire up preview step for AI-parsed data

### Error Handling
- [ ] AI not configured warning
- [ ] Unsupported file type error
- [ ] API key invalid error
- [ ] Rate limit handling
- [ ] Parse failure feedback
- [ ] Network error handling

### Testing
- [ ] Test with real bank statements
- [ ] Test with brokerage reports
- [ ] Test with loan schedules
- [ ] Test CSV smart mapping
- [ ] Test error scenarios
- [ ] Test with different providers
- [ ] Performance testing (large files)

### Documentation
- [ ] Settings page help text
- [ ] Import wizard help text
- [ ] Provider comparison guide
- [ ] Cost estimation guide
- [ ] Troubleshooting guide
- [ ] Privacy/security explanation

### Polish
- [ ] Loading states and animations
- [ ] Success/error toast messages
- [ ] Keyboard shortcuts
- [ ] Mobile responsive design
- [ ] Dark mode compatibility
- [ ] Accessibility (a11y) review

---

## Success Metrics

**User Experience:**
- Import time reduced from 10min (manual) → 30sec (AI)
- Mapping accuracy >95% with AI suggestions
- Support 90% of common financial document formats

**Technical:**
- AI request success rate >95%
- Average parse time <30 seconds
- Error recovery rate >90%
- Zero API key leaks

**Adoption:**
- 40% of users configure AI within first month
- 60% of CSV imports use AI suggestions
- 30% of imports use document parsing

---

## Future Enhancements (Post-Launch)

1. **Batch Import** - Upload multiple documents at once
2. **Smart Document Classification** - Auto-detect document type
3. **Historical Data** - Track AI accuracy over time
4. **Custom Prompts** - Let power users tweak AI prompts
5. **Webhook Integration** - Auto-import from email attachments
6. **Mobile App Support** - Photo capture → AI parse
7. **Provider Auto-Selection** - Suggest best provider per document type
8. **Cost Tracking** - Show actual AI API costs per user

---

## Questions/Decisions Needed

1. **Should we support Excel directly?**
   - Option A: Require PDF conversion first (simpler)
   - Option B: Add xlsx parser + convert to CSV (more complex)
   - **Recommendation:** Start with A, add B in v2

2. **Rate limiting for AI calls?**
   - Should we limit calls per user per day?
   - **Recommendation:** No limits initially, monitor usage

3. **Retry logic for failed AI requests?**
   - Auto-retry on transient errors?
   - **Recommendation:** Yes, 2 retries with exponential backoff

4. **Cache AI responses?**
   - Cache parsed results for same file hash?
   - **Recommendation:** Yes, cache for 24 hours

5. **Telemetry?**
   - Track AI usage, success rate, errors (anonymized)?
   - **Recommendation:** Yes, with opt-out in settings

---

## Getting Started

1. **Phase 1 (Week 1):** Settings UI
   ```bash
   # Create settings page structure
   mkdir -p src/pages/settings/ai/components
   mkdir -p src/pages/settings/ai/hooks

   # Start with provider selector
   touch src/pages/settings/ai/ai-settings-page.tsx
   ```

2. **Phase 2 (Week 2):** Import wizard enhancement
   ```bash
   # Create AI integration hooks
   touch src/pages/activity/import/hooks/use-ai-status.ts
   touch src/pages/activity/import/hooks/use-document-parser.ts

   # Enhance existing steps
   # Edit: account-selection-step.tsx
   # Edit: mapping-step.tsx
   ```

3. **Phase 3 (Week 3):** Polish, testing, documentation

---

## Support & Rollout

**Beta Testing:**
- Enable for 10% of users initially
- Collect feedback via in-app survey
- Monitor error rates and success metrics

**Rollout Plan:**
- Week 1: Internal testing
- Week 2: Beta (10% users)
- Week 3: Gradual rollout (50% users)
- Week 4: Full release (100% users)

**Fallback Plan:**
- Feature flag to disable AI features
- Classic CSV import always available
- No breaking changes to existing workflows

---

## Notes

- Backend commands assume synchronous responses (adjust if streaming needed)
- File size limits enforced on frontend (32MB) and backend
- API keys stored in OS keyring (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- All AI requests include timeout (60 seconds)
- Frontend gracefully degrades if AI unavailable

---

## Backend Refactoring (Completed 2025-11-28)

### What Changed

**Before:**
- AI logic duplicated between `src-tauri/src/ai/` (full implementation)
- `src-server` would need to duplicate everything again

**After:**
- ✅ AI logic centralized in `src-core/src/ai/`
  - `mod.rs` - `AIProviderConfig` + `create_client()`
  - `csv_mapper.rs` - CSV mapping business logic
- ✅ `src-tauri/src/ai/` - Thin re-export wrappers
  ```rust
  pub use wealthfolio_core::ai::AIProviderConfig;
  pub use wealthfolio_core::ai::csv_mapper::*;
  ```
- ✅ Tauri commands updated to pass `KeyringSecretStore` to core
- ✅ Ready for web server to add HTTP endpoints using same core logic

### Files Modified

```
src-core/
  ├─ Cargo.toml (+ai-lib dependency)
  ├─ src/lib.rs (+ai module export)
  └─ src/ai/
      ├─ mod.rs (NEW - 200 lines)
      └─ csv_mapper.rs (NEW - 159 lines)

src-tauri/
  ├─ src/ai/
  │   ├─ mod.rs (REFACTORED - now 6 lines re-export)
  │   └─ csv_mapper.rs (REFACTORED - now 2 lines re-export)
  └─ src/commands/ai_import.rs (UPDATED - pass SecretStore)
```

### Commit 1: Core Refactoring

```
refactor: centralize AI implementation in src-core

- Move AIProviderConfig and CSV mapper logic to src-core
- Update src-tauri to use thin wrappers re-exporting from core
- Pass SecretStore to core's create_client for flexibility
- Enable code sharing across desktop/web platforms
```

**Stats:** 371 insertions, 361 deletions (net refactoring, no feature changes)

### Commit 2: Web Server Implementation

```
feat: add HTTP AI endpoints to src-server

- Create /api/v1/ai/suggest-mapping POST endpoint
- Add ai_config to AppState with RwLock
- Register AI router in main API
- Use FileSecretStore for web mode secret management
- Complete backend architecture across both platforms
```

**Files Modified:**
```
src-server/
  ├─ src/api.rs (register AI router)
  ├─ src/main_lib.rs (add ai_config to AppState)
  ├─ src/api/ai.rs (NEW - 79 lines)
  └─ Cargo.lock (dependency updates)
```

**API Endpoint:**
- `POST /api/v1/ai/suggest-mapping`
  - Request: `{ headers: string[], sample_rows: Record<string, string>[] }`
  - Response: `{ success: boolean, suggestions?: MappingSuggestions, error?: string }`
  - Uses FileSecretStore from AppState
  - Handles RwLock properly (drops guard before async operations)

---

**Last Updated:** 2025-11-28
**Status:** Backend Complete (Desktop + Web), Ready for Frontend Implementation
**Estimated Effort:** 3-4 weeks frontend (1 developer)
