import { searchTicker } from "@/adapters";
import { Button } from "@wealthfolio/ui/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@wealthfolio/ui/components/ui/command";
import { Icons } from "@wealthfolio/ui/components/ui/icons";
import { Popover, PopoverContent, PopoverTrigger } from "@wealthfolio/ui/components/ui/popover";
import { Skeleton } from "@wealthfolio/ui/components/ui/skeleton";
import { SymbolSearchResult } from "@/lib/types";
import { getExchangeDisplayName } from "@/lib/constants";
import { useScenarios } from "@/hooks/use-scenarios";

import { cn } from "@/lib/utils";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { useNavigate } from "react-router-dom";

const SCENARIO_ADDON_ROUTE = "/addon/scenario-addon";

// Predefined benchmarks with canonical asset IDs
// exchangeMic is undefined for indices (will use "INDEX" as pseudo-MIC)
// exchangeMic is set for ETFs that trade on real exchanges
const BENCHMARKS = [
  {
    group: "US Market Indices",
    items: [
      { symbol: "^GSPC", name: "S&P 500", description: "Large-cap US stocks" },
      { symbol: "^NDX", name: "Nasdaq 100", description: "Large-cap tech-focused US stocks" },
      { symbol: "^RUT", name: "Russell 2000", description: "Small-cap US stocks" },
      { symbol: "^DJI", name: "Dow Jones", description: "Blue-chip US stocks" },
    ],
  },
  {
    group: "European Indices",
    items: [
      { symbol: "^FTSE", name: "FTSE 100", description: "Large-cap UK stocks" },
      { symbol: "^STOXX50E", name: "EURO STOXX 50", description: "European blue-chip stocks" },
      { symbol: "^GDAXI", name: "DAX", description: "German blue-chip stocks" },
      { symbol: "^FCHI", name: "CAC 40", description: "French large-cap stocks" },
      { symbol: "^IBEX", name: "IBEX 35", description: "Spanish large-cap stocks" },
      { symbol: "^AEX", name: "AEX", description: "Dutch blue-chip stocks" },
      { symbol: "^OMX", name: "OMX Stockholm 30", description: "Swedish large-cap stocks" },
    ],
  },
  {
    group: "Asian Indices",
    items: [
      { symbol: "^N225", name: "Nikkei 225", description: "Japanese large-cap stocks" },
      { symbol: "^HSI", name: "Hang Seng", description: "Hong Kong large-cap stocks" },
      { symbol: "000001.SS", name: "Shanghai Composite", description: "Chinese A-shares" },
      { symbol: "^KS11", name: "KOSPI", description: "South Korean stocks" },
      { symbol: "^TWII", name: "Taiwan Weighted", description: "Taiwanese stocks" },
      { symbol: "^AXJO", name: "ASX 200", description: "Australian large-cap stocks" },
      { symbol: "^BSESN", name: "BSE Sensex", description: "Indian large-cap stocks" },
      { symbol: "^NSEI", name: "NIFTY 50", description: "Indian blue-chip stocks" },
    ],
  },
  {
    group: "Global & Emerging Markets",
    items: [
      {
        symbol: "EEM",
        name: "MSCI Emerging Markets",
        description: "Emerging market stocks",
        exchangeMic: "ARCX",
      },
      {
        symbol: "ACWI",
        name: "MSCI All Country World",
        description: "Global equity markets",
        exchangeMic: "XNAS",
      },
      {
        symbol: "IEFA",
        name: "Core MSCI EAFE",
        description: "Europe, Australasia, Far East",
        exchangeMic: "ARCX",
      },
    ],
  },
  {
    group: "ETFs",
    items: [
      {
        symbol: "VOO",
        name: "Vanguard S&P 500",
        description: "S&P 500 index fund",
        exchangeMic: "ARCX",
      },
      {
        symbol: "VTI",
        name: "Vanguard Total Stock",
        description: "Total US market",
        exchangeMic: "ARCX",
      },
      {
        symbol: "VEA",
        name: "Vanguard FTSE Developed",
        description: "Developed markets ex-US",
        exchangeMic: "ARCX",
      },
      {
        symbol: "VWO",
        name: "Vanguard FTSE Emerging",
        description: "Emerging markets",
        exchangeMic: "ARCX",
      },
    ],
  },
];

interface BenchmarkSymbolSelectorProps {
  onSelect: (symbol: { id: string; name: string; type?: "symbol" | "scenario" }) => void;
  className?: string;
  iconOnly?: boolean;
}

export function BenchmarkSymbolSelector({
  onSelect,
  className,
  iconOnly = false,
}: BenchmarkSymbolSelectorProps) {
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const [value, setValue] = useState("");
  const [searchQuery, setSearchQuery] = useState("");

  // Saved basket scenarios surfaced as benchmark options.
  const { data: scenarios } = useScenarios();
  const basketScenarios = (scenarios ?? []).filter((scenario) => scenario.kind === "basket");
  const filteredScenarios = basketScenarios.filter(
    (scenario) =>
      searchQuery.length === 0 || scenario.name.toLowerCase().includes(searchQuery.toLowerCase()),
  );

  const handleScenarioSelect = (scenario: { id: string; name: string }) => {
    setValue(scenario.name);
    onSelect({ id: scenario.id, name: scenario.name, type: "scenario" });
    setOpen(false);
    setSearchQuery("");
  };

  // Query for dynamic ticker search
  const {
    data: searchResults,
    isLoading,
    isError,
  } = useQuery<SymbolSearchResult[], Error>({
    queryKey: ["benchmark-ticker-search", searchQuery],
    queryFn: () => searchTicker(searchQuery),
    enabled: searchQuery?.length > 2, // Only search when query is longer than 2 characters
  });

  // Sort search results by score if available
  const sortedSearchResults = searchResults?.sort((a, b) => b.score - a.score) ?? [];

  // Filter out search results that are already in predefined benchmarks
  const existingSymbols = BENCHMARKS.flatMap((group) => group.items.map((item) => item.symbol));
  const filteredSearchResults = sortedSearchResults.filter(
    (result) => !existingSymbols.includes(result.symbol),
  );

  const handleBenchmarkSelect = (benchmark: {
    symbol: string;
    name: string;
    exchangeMic?: string;
  }) => {
    setValue(benchmark.name);
    onSelect({ id: benchmark.symbol, name: benchmark.name });
    setOpen(false);
    setSearchQuery(""); // Clear search when selecting
  };

  const handleSearchResultSelect = (ticker: SymbolSearchResult) => {
    setValue(ticker.longName || ticker.symbol);
    onSelect({
      id: ticker.existingAssetId || ticker.symbol,
      name: ticker.longName || ticker.symbol,
    });
    setOpen(false);
    setSearchQuery(""); // Clear search when selecting
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          role="combobox"
          aria-expanded={open}
          aria-label={iconOnly ? "Add benchmark" : undefined}
          className={cn(
            "bg-secondary/30 hover:bg-muted/80 flex items-center gap-1.5 rounded-md border-dashed text-sm font-medium",
            iconOnly ? "h-9 w-9 p-0" : "h-8 px-3 py-1",
            className,
          )}
          size={iconOnly ? "icon" : "sm"}
        >
          <Icons.TrendingUp className="h-4 w-4" />
          {!iconOnly && "Add Benchmark"}
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-[350px] p-0">
        <Command shouldFilter={false}>
          <CommandInput
            placeholder="Search benchmarks or any symbol..."
            value={searchQuery}
            onValueChange={setSearchQuery}
          />
          <CommandList className="max-h-[300px] overflow-y-auto">
            <CommandEmpty>
              {isLoading ? "Searching..." : "No benchmarks or symbols found."}
            </CommandEmpty>

            {/* Saved basket scenarios */}
            {filteredScenarios.length > 0 && (
              <CommandGroup
                heading="Scenarios"
                className="[&_[cmdk-group-heading]]:bg-popover [&_[cmdk-group-heading]]:border-border/10 [&_[cmdk-group-heading]]:sticky [&_[cmdk-group-heading]]:top-0 [&_[cmdk-group-heading]]:z-10 [&_[cmdk-group-heading]]:border-b"
              >
                {filteredScenarios.map((scenario) => (
                  <CommandItem
                    key={scenario.id}
                    value={`scenario ${scenario.name} ${scenario.id}`}
                    onSelect={() => handleScenarioSelect(scenario)}
                  >
                    <div className="flex flex-col">
                      <div className="flex items-center">
                        <Icons.TrendingUp className="text-muted-foreground mr-2 h-3.5 w-3.5" />
                        <span className="font-medium">{scenario.name}</span>
                      </div>
                      <span className="text-muted-foreground text-xs">
                        {scenario.basket.length} holdings · basket replay
                      </span>
                    </div>
                    <Icons.Check
                      className={cn(
                        "ml-auto h-4 w-4",
                        value === scenario.name ? "opacity-100" : "opacity-0",
                      )}
                    />
                  </CommandItem>
                ))}
              </CommandGroup>
            )}

            {/* Predefined benchmark groups */}
            {BENCHMARKS.map((group) => (
              <CommandGroup
                key={group.group}
                heading={group.group}
                className="[&_[cmdk-group-heading]]:bg-popover [&_[cmdk-group-heading]]:border-border/10 [&_[cmdk-group-heading]]:sticky [&_[cmdk-group-heading]]:top-0 [&_[cmdk-group-heading]]:z-10 [&_[cmdk-group-heading]]:border-b"
              >
                {group.items
                  .filter(
                    (benchmark) =>
                      searchQuery.length === 0 ||
                      benchmark.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                      benchmark.symbol.toLowerCase().includes(searchQuery.toLowerCase()) ||
                      benchmark.description.toLowerCase().includes(searchQuery.toLowerCase()),
                  )
                  .map((benchmark) => (
                    <CommandItem
                      key={benchmark.symbol}
                      value={`${benchmark.name} ${benchmark.symbol}`}
                      onSelect={() => handleBenchmarkSelect(benchmark)}
                    >
                      <div className="flex flex-col">
                        <div className="flex items-center">
                          <span className="font-medium">{benchmark.name}</span>
                          <span className="text-muted-foreground ml-2 text-xs">
                            {benchmark.symbol}
                          </span>
                        </div>
                        <span className="text-muted-foreground text-xs">
                          {benchmark.description}
                        </span>
                      </div>
                      <Icons.Check
                        className={cn(
                          "ml-auto h-4 w-4",
                          value === benchmark.name ? "opacity-100" : "opacity-0",
                        )}
                      />
                    </CommandItem>
                  ))}
              </CommandGroup>
            ))}

            {/* Loading state for search results */}
            {isLoading && searchQuery.length > 2 && (
              <CommandGroup
                heading="Search Results"
                className="[&_[cmdk-group-heading]]:bg-popover [&_[cmdk-group-heading]]:border-border/10 [&_[cmdk-group-heading]]:sticky [&_[cmdk-group-heading]]:top-0 [&_[cmdk-group-heading]]:z-10 [&_[cmdk-group-heading]]:border-b"
              >
                <div className="space-y-2 p-2">
                  <Skeleton className="h-12 w-full" />
                  <Skeleton className="h-12 w-full" />
                  <Skeleton className="h-12 w-full" />
                </div>
              </CommandGroup>
            )}

            {/* Error state for search results */}
            {isError && searchQuery.length > 2 && (
              <CommandGroup
                heading="Search Results"
                className="[&_[cmdk-group-heading]]:bg-popover [&_[cmdk-group-heading]]:border-border/10 [&_[cmdk-group-heading]]:sticky [&_[cmdk-group-heading]]:top-0 [&_[cmdk-group-heading]]:z-10 [&_[cmdk-group-heading]]:border-b"
              >
                <div className="text-muted-foreground p-4 text-sm">
                  Error searching for symbols. Please try again.
                </div>
              </CommandGroup>
            )}

            {/* Dynamic search results */}
            {!isLoading &&
              !isError &&
              filteredSearchResults.length > 0 &&
              searchQuery.length > 2 && (
                <CommandGroup
                  heading="Search Results"
                  className="[&_[cmdk-group-heading]]:bg-popover [&_[cmdk-group-heading]]:border-border/10 [&_[cmdk-group-heading]]:sticky [&_[cmdk-group-heading]]:top-0 [&_[cmdk-group-heading]]:z-10 [&_[cmdk-group-heading]]:border-b"
                >
                  {filteredSearchResults.slice(0, 8).map((ticker) => (
                    <CommandItem
                      key={ticker.symbol}
                      value={ticker.symbol}
                      onSelect={() => handleSearchResultSelect(ticker)}
                    >
                      <div className="flex flex-col">
                        <div className="flex items-center">
                          <span className="font-medium">{ticker.longName || ticker.symbol}</span>
                          <span className="text-muted-foreground ml-2 text-xs">
                            {ticker.symbol}
                          </span>
                        </div>
                        {ticker.exchange && (
                          <span className="text-muted-foreground text-xs">
                            {ticker.exchangeName || getExchangeDisplayName(ticker.exchange)}
                          </span>
                        )}
                      </div>
                      <Icons.Check
                        className={cn(
                          "ml-auto h-4 w-4",
                          value === (ticker.longName || ticker.symbol)
                            ? "opacity-100"
                            : "opacity-0",
                        )}
                      />
                    </CommandItem>
                  ))}
                </CommandGroup>
              )}
          </CommandList>
          <button
            type="button"
            onClick={() => {
              setOpen(false);
              navigate(SCENARIO_ADDON_ROUTE);
            }}
            className="text-muted-foreground hover:text-foreground border-border/40 flex w-full items-center gap-1.5 border-t px-3 py-2 text-left text-xs"
          >
            <Icons.Settings className="h-3.5 w-3.5" />
            Manage scenarios
          </button>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
