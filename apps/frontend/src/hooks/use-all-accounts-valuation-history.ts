import { useQuery, keepPreviousData } from "@tanstack/react-query";
import type { AccountValuation, DateRange } from "@/lib/types";
import { getHistoricalValuations } from "@/adapters";
import { QueryKeys } from "@/lib/query-keys";
import { format } from "date-fns";

export interface AccountValuationHistory {
  accountId: string;
  accountName: string;
  valuations: AccountValuation[];
}

interface AccountRef {
  id: string;
  name: string;
}

/**
 * Fetches the historical valuation series for each account individually, so the
 * dashboard history chart can overlay a per-account line. Mirrors
 * useValuationHistory's date handling but scopes each query to a single account
 * via { type: "account", accountId }.
 */
export function useAllAccountsValuationHistory(
  dateRange: DateRange | undefined,
  accounts: AccountRef[],
  options: { enabled?: boolean } = {},
) {
  const dateRangeMode = dateRange === undefined ? "all" : "range";
  const startDate = dateRange?.from ? format(dateRange.from, "yyyy-MM-dd") : undefined;
  const endDate = dateRange?.to ? format(dateRange.to, "yyyy-MM-dd") : undefined;
  const accountIds = accounts.map((a) => a.id);

  const isEnabled =
    (options.enabled ?? true) &&
    accountIds.length > 0 &&
    (dateRangeMode === "all" || (!!startDate && !!endDate));

  const {
    data: allAccountsHistory,
    isLoading,
    isFetching,
  } = useQuery<AccountValuationHistory[], Error>({
    queryKey: [
      QueryKeys.HISTORY_VALUATION,
      "all-accounts",
      accountIds,
      dateRangeMode,
      startDate ?? null,
      endDate ?? null,
    ],
    queryFn: async () => {
      const results = await Promise.all(
        accounts.map(async (account) => {
          const scope = { type: "account", accountId: account.id } as const;
          const valuations =
            dateRangeMode === "all"
              ? await getHistoricalValuations(scope, undefined, undefined)
              : await getHistoricalValuations(scope, startDate, endDate);
          return { accountId: account.id, accountName: account.name, valuations };
        }),
      );
      return results;
    },
    enabled: isEnabled,
    placeholderData: keepPreviousData,
  });

  return {
    allAccountsHistory,
    isLoading: isLoading || isFetching,
  };
}
