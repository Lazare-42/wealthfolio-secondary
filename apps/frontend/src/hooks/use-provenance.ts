import { listSourceEmails } from "@/adapters";
import { QueryKeys } from "@/lib/query-keys";
import type { ChatSourceEmail } from "@/lib/types";
import { useQuery } from "@tanstack/react-query";

export function useSourceEmails(params?: { threadId?: string; limit?: number }) {
  return useQuery<ChatSourceEmail[]>({
    queryKey: [QueryKeys.SOURCE_EMAILS, params ?? {}],
    queryFn: () => listSourceEmails(params),
  });
}
