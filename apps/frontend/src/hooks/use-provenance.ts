import { listSourceEmails } from "@/adapters";
import type { ChatSourceEmail } from "@/lib/types";
import { useQuery } from "@tanstack/react-query";

export function useSourceEmails(params?: { threadId?: string; limit?: number }) {
  return useQuery<ChatSourceEmail[]>({
    queryKey: ["source-emails", params ?? {}],
    queryFn: () => listSourceEmails(params),
  });
}
