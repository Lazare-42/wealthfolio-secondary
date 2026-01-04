import { useMutation, useQueryClient } from "@tanstack/react-query";
import { logger } from "@/adapters";
import { importActivitiesWithSession } from "@/commands/activity-import";
import { toast } from "@/components/ui/use-toast";
import { QueryKeys } from "@/lib/query-keys";
import type { ActivityImport, ImportWithSessionResponse } from "@/lib/types";

export function useActivityImportMutations({
  onSuccess,
  onError,
}: {
  onSuccess?: (activities: ActivityImport[], sessionId?: string) => void;
  onError?: (error: string) => void;
} = {}) {
  const queryClient = useQueryClient();

  const confirmImportMutation = useMutation({
    mutationFn: importActivitiesWithSession,
    onSuccess: async (result: ImportWithSessionResponse) => {
      // Invalidate import sessions cache
      queryClient.invalidateQueries({ queryKey: [QueryKeys.IMPORT_SESSIONS] });
      queryClient.invalidateQueries({ queryKey: [QueryKeys.ACTIVITIES] });

      // Call the provided onSuccess callback if it exists
      if (onSuccess) {
        onSuccess(result.activities, result.session.id);
        toast({
          title: "Import successful",
          description: `${result.session.successCount} activities have been imported successfully.`,
        });
      }
    },
    onError: (error: unknown) => {
      logger.error(`Error confirming import: ${String(error)}`);

      // Call the provided onError callback if it exists
      if (onError) {
        const errMsg =
          error && typeof error === "object" && "message" in error
            ? String((error as { message?: unknown }).message)
            : "An error occurred during import";
        onError(errMsg);
      } else {
        toast({
          title: "Uh oh! Something went wrong.",
          description: "Please try again or report an issue if the problem persists.",
          variant: "destructive",
        });
      }
    },
  });

  return {
    confirmImportMutation,
  };
}
