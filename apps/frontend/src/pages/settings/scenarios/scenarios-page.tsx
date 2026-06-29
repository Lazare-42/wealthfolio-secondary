import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";

import { useAccounts } from "@/hooks/use-accounts";
import { usePortfolios } from "@/hooks/use-portfolios";
import { useScenarioMutations, useScenarios } from "@/hooks/use-scenarios";
import { AccountPurpose } from "@/lib/constants";
import type {
  Account,
  AccountScope,
  BasketPosition,
  NewPortfolioScenario,
  PortfolioScenario,
  PortfolioWithAccounts,
} from "@/lib/types";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@wealthfolio/ui/components/ui/alert-dialog";
import { Avatar, AvatarFallback } from "@wealthfolio/ui/components/ui/avatar";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@wealthfolio/ui/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@wealthfolio/ui/components/ui/dropdown-menu";
import { Input } from "@wealthfolio/ui/components/ui/input";
import { Label } from "@wealthfolio/ui/components/ui/label";
import { Textarea } from "@wealthfolio/ui/components/ui/textarea";
import { Button, Checkbox, EmptyPlaceholder, Icons, Separator, Skeleton } from "@wealthfolio/ui";
import { SettingsHeader } from "../settings-header";

type ScenarioScopeMode = AccountScope["type"];

const ADDON_SOURCE_PREFIX = "scenario-addon";

/// A scenario the Scenario Planner add-on owns. The built-in page must not edit
/// it: editing would rebuild `assumptions` and wipe the add-on's projection data.
function isAddonOwned(scenario: PortfolioScenario): boolean {
  if (scenario.kind === "projection") return true;
  const source = (scenario.assumptions as { source?: unknown } | undefined)?.source;
  return typeof source === "string" && source.startsWith(ADDON_SOURCE_PREFIX);
}

function scenarioBadge(scenario: PortfolioScenario): string | null {
  if (isAddonOwned(scenario)) return "Planner add-on";
  if (scenario.kind === "basket") return "Basket";
  return null;
}

export default function ScenariosPage() {
  const navigate = useNavigate();
  const { data: scenarios = [], isLoading: isScenariosLoading } = useScenarios();
  const { accounts, isLoading: isAccountsLoading } = useAccounts({
    filterActive: false,
    includeArchived: true,
    accountPurpose: AccountPurpose.PERFORMANCE,
  });
  const { data: portfolios = [], isLoading: isPortfoliosLoading } = usePortfolios();
  const { createMutation, updateMutation, deleteMutation } = useScenarioMutations();

  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<PortfolioScenario | null>(null);
  const [deleting, setDeleting] = useState<PortfolioScenario | null>(null);

  const isLoading = isScenariosLoading || isAccountsLoading || isPortfoliosLoading;
  const accountNameById = useMemo(
    () => new Map(accounts.map((account) => [account.id, account.name])),
    [accounts],
  );
  const portfolioNameById = useMemo(
    () => new Map(portfolios.map((portfolio) => [portfolio.id, portfolio.name])),
    [portfolios],
  );

  const openCreate = () => {
    setEditing(null);
    setOpen(true);
  };

  const openEdit = (scenario: PortfolioScenario) => {
    setEditing(scenario);
    setOpen(true);
  };

  const handleSave = (scenario: NewPortfolioScenario | PortfolioScenario) => {
    if ("id" in scenario) {
      updateMutation.mutate(scenario, { onSuccess: () => setOpen(false) });
      return;
    }
    createMutation.mutate(scenario, { onSuccess: () => setOpen(false) });
  };

  const handleDelete = () => {
    if (!deleting) return;
    deleteMutation.mutate(deleting.id, { onSuccess: () => setDeleting(null) });
  };

  const compareScenario = (scenario: PortfolioScenario) => {
    const benchmarkText =
      scenario.benchmarkSymbols.length > 0
        ? ` against ${scenario.benchmarkSymbols.join(", ")}`
        : "";
    navigate("/assistant", {
      state: {
        aiPrompt: `Compare saved scenario "${scenario.name}" (${scenario.id})${benchmarkText}. Use the saved scenario tool and summarize the portfolio and benchmark performance.`,
      },
    });
  };

  if (isLoading) {
    return (
      <div className="space-y-3">
        <Skeleton className="h-12" />
        <Skeleton className="h-24" />
      </div>
    );
  }

  return (
    <>
      <div className="space-y-6">
        <SettingsHeader
          heading="Scenarios"
          text="Save portfolio scopes for assistant comparisons and benchmark reviews."
          actionsInline
        >
          <>
            <Button
              size="icon"
              className="sm:hidden"
              onClick={openCreate}
              aria-label="Add scenario"
            >
              <Icons.Plus className="h-4 w-4" />
            </Button>
            <Button size="sm" className="hidden sm:inline-flex" onClick={openCreate}>
              <Icons.Plus className="mr-2 h-4 w-4" />
              Add scenario
            </Button>
          </>
        </SettingsHeader>
        <Separator />

        {scenarios.length === 0 ? (
          <EmptyPlaceholder>
            <EmptyPlaceholder.Icon name="Presentation" />
            <EmptyPlaceholder.Title>No scenarios yet</EmptyPlaceholder.Title>
            <EmptyPlaceholder.Description>
              Save account scopes, as-of dates, and benchmark tickers for repeat comparisons.
            </EmptyPlaceholder.Description>
            <Button onClick={openCreate}>
              <Icons.Plus className="mr-2 h-4 w-4" />
              Add a scenario
            </Button>
          </EmptyPlaceholder>
        ) : (
          <div className="divide-border bg-card divide-y rounded-md border">
            {scenarios.map((scenario) => (
              <div
                key={scenario.id}
                className="grid gap-4 p-4 md:grid-cols-[1fr_auto] md:items-center"
              >
                <div className="flex min-w-0 items-start gap-3">
                  <Avatar className="h-10 w-10 rounded-lg">
                    <AvatarFallback className="rounded-lg bg-emerald-500/10">
                      <Icons.Presentation className="h-5 w-5 text-emerald-500" />
                    </AvatarFallback>
                  </Avatar>
                  <div className="min-w-0 space-y-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-semibold">{scenario.name}</span>
                      {scenarioBadge(scenario) && (
                        <span className="rounded-md bg-emerald-500/10 px-2 py-0.5 text-xs text-emerald-600 dark:text-emerald-400">
                          {scenarioBadge(scenario)}
                        </span>
                      )}
                      {scenario.asOfDate && (
                        <span className="bg-muted text-muted-foreground rounded-md px-2 py-0.5 text-xs">
                          {scenario.asOfDate}
                        </span>
                      )}
                    </div>
                    <div className="text-muted-foreground flex flex-wrap items-center gap-1.5 text-sm">
                      <span>
                        {scopeLabel(scenario.accountScope, accountNameById, portfolioNameById)}
                      </span>
                      <span>·</span>
                      <span>
                        {scenario.resolvedAccountIds.length} account
                        {scenario.resolvedAccountIds.length !== 1 ? "s" : ""}
                      </span>
                      {scenario.benchmarkSymbols.length > 0 && (
                        <>
                          <span>·</span>
                          <span>{scenario.benchmarkSymbols.join(", ")}</span>
                        </>
                      )}
                    </div>
                    {scenario.description && (
                      <p className="text-muted-foreground text-sm">{scenario.description}</p>
                    )}
                  </div>
                </div>

                <div className="flex items-center gap-2 justify-self-start md:justify-self-end">
                  <Button variant="outline" size="sm" onClick={() => compareScenario(scenario)}>
                    <Icons.ArrowRightLeft className="mr-2 h-4 w-4" />
                    Compare
                  </Button>
                  <DropdownMenu>
                    <DropdownMenuTrigger className="hover:bg-muted flex h-8 w-8 items-center justify-center rounded-md border transition-colors">
                      <Icons.MoreVertical className="h-4 w-4" />
                      <span className="sr-only">Open</span>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem
                        disabled={isAddonOwned(scenario)}
                        onClick={() => !isAddonOwned(scenario) && openEdit(scenario)}
                      >
                        {isAddonOwned(scenario) ? "Managed by add-on" : "Edit"}
                      </DropdownMenuItem>
                      <DropdownMenuSeparator />
                      <DropdownMenuItem
                        className="text-destructive focus:text-destructive flex cursor-pointer items-center"
                        onSelect={() => setDeleting(scenario)}
                      >
                        Delete
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <ScenarioDialog
        key={editing?.id ?? "new"}
        open={open}
        scenario={editing}
        accounts={accounts}
        portfolios={portfolios}
        onClose={() => setOpen(false)}
        onSave={handleSave}
        isSaving={createMutation.isPending || updateMutation.isPending}
      />

      <AlertDialog open={deleting !== null} onOpenChange={(value) => !value && setDeleting(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete scenario?</AlertDialogTitle>
            <AlertDialogDescription>
              This removes {deleting ? `"${deleting.name}"` : "this scenario"}. This action cannot
              be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={deleteMutation.isPending}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              disabled={deleteMutation.isPending}
              onClick={handleDelete}
            >
              <Icons.Trash className="mr-2 h-4 w-4" />
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

interface ScenarioDialogProps {
  open: boolean;
  scenario: PortfolioScenario | null;
  accounts: Account[];
  portfolios: PortfolioWithAccounts[];
  onClose: () => void;
  onSave: (scenario: NewPortfolioScenario | PortfolioScenario) => void;
  isSaving: boolean;
}

function ScenarioDialog({
  open,
  scenario,
  accounts,
  portfolios,
  onClose,
  onSave,
  isSaving,
}: ScenarioDialogProps) {
  const initialScope = scenario?.accountScope ?? { type: "all" as const };
  const [name, setName] = useState(scenario?.name ?? "");
  const [description, setDescription] = useState(scenario?.description ?? "");
  const [scopeMode, setScopeMode] = useState<ScenarioScopeMode>(initialScope.type);
  const [accountId, setAccountId] = useState(
    initialScope.type === "account" ? initialScope.accountId : (accounts[0]?.id ?? ""),
  );
  const [portfolioId, setPortfolioId] = useState(
    initialScope.type === "portfolio" ? initialScope.portfolioId : (portfolios[0]?.id ?? ""),
  );
  const [accountIds, setAccountIds] = useState<string[]>(
    initialScope.type === "accounts" ? initialScope.accountIds : [],
  );
  const [asOfDate, setAsOfDate] = useState(scenario?.asOfDate ?? "");
  const [benchmarks, setBenchmarks] = useState(scenario?.benchmarkSymbols.join(", ") ?? "SPY");
  const [basket, setBasket] = useState<BasketPosition[]>(scenario?.basket ?? []);
  const [assumptions, setAssumptions] = useState(() => {
    const value = scenario?.assumptions ?? {};
    return Object.keys(value).length > 0 ? JSON.stringify(value, null, 2) : "";
  });
  const [formError, setFormError] = useState<string | null>(null);

  const canSave =
    name.trim().length > 0 &&
    (scopeMode === "all" ||
      (scopeMode === "account" && accountId.length > 0) ||
      (scopeMode === "portfolio" && portfolioId.length > 0) ||
      (scopeMode === "accounts" && accountIds.length > 0));

  const toggleAccount = (id: string) =>
    setAccountIds((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]));

  const addBasketLeg = () => setBasket((prev) => [...prev, { symbol: "", weight: 0 }]);
  const updateBasketLeg = (index: number, patch: Partial<BasketPosition>) =>
    setBasket((prev) => prev.map((leg, idx) => (idx === index ? { ...leg, ...patch } : leg)));
  const removeBasketLeg = (index: number) =>
    setBasket((prev) => prev.filter((_, idx) => idx !== index));

  const handleSave = () => {
    setFormError(null);
    if (!canSave) return;

    let parsedAssumptions: Record<string, unknown>;
    try {
      parsedAssumptions = parseAssumptions(assumptions);
    } catch (error) {
      setFormError(error instanceof Error ? error.message : "Invalid assumptions JSON.");
      return;
    }

    const cleanBasket = basket
      .map((leg) => ({ symbol: leg.symbol.trim().toUpperCase(), weight: Number(leg.weight) }))
      .filter((leg) => leg.symbol.length > 0 && Number.isFinite(leg.weight) && leg.weight > 0);

    const payload: NewPortfolioScenario = {
      name: name.trim(),
      description: description.trim() || undefined,
      kind: cleanBasket.length > 0 ? "basket" : "comparison",
      accountScope: buildScope(scopeMode, accountId, portfolioId, accountIds),
      asOfDate: asOfDate || undefined,
      benchmarkSymbols: parseBenchmarkSymbols(benchmarks),
      basket: cleanBasket,
      assumptions: parsedAssumptions,
    };

    if (scenario) {
      onSave({ ...scenario, ...payload });
      return;
    }
    onSave(payload);
  };

  return (
    <Dialog open={open} onOpenChange={(value) => !value && onClose()}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{scenario ? "Edit scenario" : "New scenario"}</DialogTitle>
        </DialogHeader>

        <div className="space-y-5 py-2">
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-1">
              <Label htmlFor="scenario-name">Name</Label>
              <Input
                id="scenario-name"
                value={name}
                onChange={(event) => setName(event.target.value)}
                placeholder="e.g. Core portfolio vs S&P 500"
              />
            </div>
            <div className="space-y-1">
              <Label htmlFor="scenario-as-of">As-of date</Label>
              <Input
                id="scenario-as-of"
                type="date"
                value={asOfDate}
                onChange={(event) => setAsOfDate(event.target.value)}
              />
            </div>
          </div>

          <div className="space-y-1">
            <Label htmlFor="scenario-description">Description (optional)</Label>
            <Textarea
              id="scenario-description"
              value={description ?? ""}
              onChange={(event) => setDescription(event.target.value)}
              rows={2}
              placeholder="e.g. Long-term taxable allocation with SPY and QQQ as benchmarks"
            />
          </div>

          <div className="space-y-3">
            <Label>Scope</Label>
            <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
              {(["all", "account", "portfolio", "accounts"] as ScenarioScopeMode[]).map((mode) => (
                <Button
                  key={mode}
                  type="button"
                  variant={scopeMode === mode ? "default" : "outline"}
                  size="sm"
                  onClick={() => setScopeMode(mode)}
                >
                  {scopeModeLabel(mode)}
                </Button>
              ))}
            </div>

            {scopeMode === "account" && (
              <select
                value={accountId}
                onChange={(event) => setAccountId(event.target.value)}
                className="border-input bg-background h-9 w-full rounded-md border px-3 text-sm"
              >
                {accounts.map((account) => (
                  <option key={account.id} value={account.id}>
                    {account.name} ({account.currency})
                  </option>
                ))}
              </select>
            )}

            {scopeMode === "portfolio" && (
              <select
                value={portfolioId}
                onChange={(event) => setPortfolioId(event.target.value)}
                className="border-input bg-background h-9 w-full rounded-md border px-3 text-sm"
              >
                {portfolios.map((portfolio) => (
                  <option key={portfolio.id} value={portfolio.id}>
                    {portfolio.name}
                  </option>
                ))}
              </select>
            )}

            {scopeMode === "accounts" && (
              <div className="divide-border max-h-52 overflow-y-auto rounded-md border">
                {accounts.length === 0 ? (
                  <p className="text-muted-foreground p-3 text-sm">
                    No performance accounts found.
                  </p>
                ) : (
                  accounts.map((account) => (
                    <label
                      key={account.id}
                      className="hover:bg-muted/40 flex cursor-pointer items-center gap-3 px-3 py-2"
                    >
                      <Checkbox
                        checked={accountIds.includes(account.id)}
                        onCheckedChange={() => toggleAccount(account.id)}
                      />
                      <span className="text-sm">
                        {account.name}{" "}
                        <span className="text-muted-foreground text-xs">({account.currency})</span>
                      </span>
                    </label>
                  ))
                )}
              </div>
            )}
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-1">
              <Label htmlFor="scenario-benchmarks">Benchmarks</Label>
              <Input
                id="scenario-benchmarks"
                value={benchmarks}
                onChange={(event) => setBenchmarks(event.target.value)}
                placeholder="SPY, QQQ, VTI"
              />
            </div>
            <div className="space-y-1">
              <Label htmlFor="scenario-assumptions">Assumptions JSON (optional)</Label>
              <Textarea
                id="scenario-assumptions"
                value={assumptions}
                onChange={(event) => setAssumptions(event.target.value)}
                rows={3}
                placeholder='{"note":"rebalance quarterly"}'
              />
            </div>
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <div>
                <Label>Basket (optional)</Label>
                <p className="text-muted-foreground text-xs">
                  Replay a hypothetical weighted portfolio of securities over history. Leave empty
                  for a plain benchmark comparison.
                </p>
              </div>
              <Button type="button" variant="outline" size="sm" onClick={addBasketLeg}>
                <Icons.Plus className="mr-2 h-4 w-4" />
                Add leg
              </Button>
            </div>
            {basket.length > 0 && (
              <div className="space-y-2">
                {basket.map((leg, index) => (
                  <div key={index} className="flex items-center gap-2">
                    <Input
                      value={leg.symbol}
                      onChange={(event) => updateBasketLeg(index, { symbol: event.target.value })}
                      placeholder="Symbol e.g. SPY"
                      className="flex-1"
                    />
                    <Input
                      type="number"
                      min={0}
                      step="any"
                      value={Number.isFinite(leg.weight) && leg.weight !== 0 ? leg.weight : ""}
                      onChange={(event) =>
                        updateBasketLeg(index, { weight: Number(event.target.value) })
                      }
                      placeholder="Weight"
                      className="w-28"
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => removeBasketLeg(index)}
                      aria-label="Remove leg"
                    >
                      <Icons.Trash className="h-4 w-4" />
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </div>

          {formError && <p className="text-destructive text-sm">{formError}</p>}
          {!canSave && (
            <p className="text-muted-foreground text-xs">
              Add a name and choose at least one account when using an account scope.
            </p>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={!canSave || isSaving}>
            {isSaving ? "Saving..." : "Save"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function buildScope(
  mode: ScenarioScopeMode,
  accountId: string,
  portfolioId: string,
  accountIds: string[],
): AccountScope {
  switch (mode) {
    case "account":
      return { type: "account", accountId };
    case "portfolio":
      return { type: "portfolio", portfolioId };
    case "accounts":
      return { type: "accounts", accountIds };
    case "all":
    default:
      return { type: "all" };
  }
}

function parseBenchmarkSymbols(value: string): string[] {
  const seen = new Set<string>();
  return value
    .split(/[,\s]+/)
    .map((symbol) => symbol.trim().toUpperCase())
    .filter((symbol) => {
      if (!symbol || seen.has(symbol)) return false;
      seen.add(symbol);
      return true;
    });
}

function parseAssumptions(value: string): Record<string, unknown> {
  if (!value.trim()) return {};
  const parsed = JSON.parse(value) as unknown;
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("Assumptions must be a JSON object.");
  }
  return parsed as Record<string, unknown>;
}

function scopeModeLabel(mode: ScenarioScopeMode): string {
  switch (mode) {
    case "account":
      return "Account";
    case "portfolio":
      return "Portfolio";
    case "accounts":
      return "Accounts";
    case "all":
    default:
      return "All";
  }
}

function scopeLabel(
  scope: AccountScope,
  accountNameById: Map<string, string>,
  portfolioNameById: Map<string, string>,
): string {
  switch (scope.type) {
    case "account":
      return accountNameById.get(scope.accountId) ?? "Account";
    case "portfolio":
      return portfolioNameById.get(scope.portfolioId) ?? "Portfolio";
    case "accounts":
      return `${scope.accountIds.length} selected account${scope.accountIds.length !== 1 ? "s" : ""}`;
    case "all":
    default:
      return "All performance accounts";
  }
}
