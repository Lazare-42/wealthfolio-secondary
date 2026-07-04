import type { useAiArenaMutations } from "@/hooks/use-ai-arena";
import { Icons } from "@wealthfolio/ui";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@wealthfolio/ui/components/ui/alert-dialog";
import { Button } from "@wealthfolio/ui/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@wealthfolio/ui/components/ui/tooltip";

/** Arena-tab header actions: Run due + Settle (with confirm). */
export function ArenaActions({
  selectedChallengeId,
  runDueMutation,
  settleChallengeMutation,
}: {
  selectedChallengeId: string;
  runDueMutation: ReturnType<typeof useAiArenaMutations>["runDueMutation"];
  settleChallengeMutation: ReturnType<typeof useAiArenaMutations>["settleChallengeMutation"];
}) {
  return (
    <>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="outline"
            size="sm"
            onClick={() => runDueMutation.mutate()}
            disabled={runDueMutation.isPending}
          >
            {runDueMutation.isPending ? (
              <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Icons.Clock className="mr-2 h-4 w-4" />
            )}
            Run due
          </Button>
        </TooltipTrigger>
        <TooltipContent>Run every scheduled agent whose challenge run time is due.</TooltipContent>
      </Tooltip>
      <AlertDialog>
        <Tooltip>
          <TooltipTrigger asChild>
            <AlertDialogTrigger asChild>
              <Button
                size="sm"
                variant="outline"
                disabled={!selectedChallengeId || settleChallengeMutation.isPending}
              >
                <Icons.CheckCircle className="mr-2 h-4 w-4" />
                Settle
              </Button>
            </AlertDialogTrigger>
          </TooltipTrigger>
          <TooltipContent>Finalize scores and close this challenge.</TooltipContent>
        </Tooltip>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Settle this challenge?</AlertDialogTitle>
            <AlertDialogDescription>
              Finalize scores and close this challenge. Agents can no longer run in it.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() =>
                selectedChallengeId && settleChallengeMutation.mutate(selectedChallengeId)
              }
            >
              Settle
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
