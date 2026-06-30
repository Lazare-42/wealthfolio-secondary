import { getActivitySources, listSourceEmails } from "@/adapters";
import type { ActivitySource, ChatSourceEmail } from "@/lib/types";
import { useQuery } from "@tanstack/react-query";

export function useSourceEmails(params?: { threadId?: string; limit?: number }) {
  return useQuery<ChatSourceEmail[]>({
    queryKey: ["source-emails", params ?? {}],
    queryFn: () => listSourceEmails(params),
  });
}

export function useActivitySources(activityId?: string) {
  return useQuery<ActivitySource[]>({
    queryKey: ["activity-sources", activityId],
    queryFn: () => getActivitySources(activityId as string),
    enabled: !!activityId,
  });
}
