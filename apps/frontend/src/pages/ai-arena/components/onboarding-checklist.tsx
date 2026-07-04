import { useNavigate } from "react-router-dom";

import { usePersistentState } from "@/hooks/use-persistent-state";
import { Icons } from "@wealthfolio/ui";
import { Button } from "@wealthfolio/ui/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@wealthfolio/ui/components/ui/card";

/**
 * Onboarding plan for new arena users. Steps derive from data the arena page
 * already loads; shown while incomplete, dismissible via localStorage.
 */
export function OnboardingChecklist({
  hasProvider,
  hasAgent,
  hasChallenge,
  hasParticipant,
  hasRun,
  goTo,
}: {
  hasProvider: boolean;
  hasAgent: boolean;
  hasChallenge: boolean;
  hasParticipant: boolean;
  hasRun: boolean;
  /** Switch the page tab, then scroll to an anchor once the view renders. */
  goTo: (tab: string, anchorId?: string) => void;
}) {
  const navigate = useNavigate();
  const [dismissed, setDismissed] = usePersistentState("ai-arena-onboarding-dismissed", false);

  const steps = [
    {
      label: "Connect an AI provider",
      description: "Add an API key so agents can think.",
      done: hasProvider,
      actionLabel: "Open settings",
      onAction: () => navigate("/settings/ai-providers"),
    },
    {
      label: "Create your first agent",
      description: "Pick a provider, model, and persona in Setup.",
      done: hasAgent,
      actionLabel: "Go to agent form",
      onAction: () => goTo("setup", "arena-agent-form"),
    },
    {
      label: "Create a challenge",
      description: "Define the market, universe, and rules — AI can draft it.",
      done: hasChallenge,
      actionLabel: "New challenge",
      onAction: () => navigate("/ai-arena/challenges/new"),
    },
    {
      label: "Join agents to a challenge",
      description: "Enter agents into the match from the Participants panel.",
      done: hasParticipant,
      actionLabel: "Go to participants",
      onAction: () => goTo("arena", "arena-participants"),
    },
    {
      label: "Run a round",
      description: "Click Run on a participant (or Run due in the header).",
      done: hasRun,
      actionLabel: "Go to participants",
      onAction: () => goTo("arena", "arena-participants"),
    },
  ];

  const allDone = steps.every((step) => step.done);
  if (dismissed || allDone) return null;

  const dismiss = () => setDismissed(true);

  const doneCount = steps.filter((step) => step.done).length;

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <div className="flex items-center gap-2">
          <Icons.ListChecks className="text-primary h-4 w-4" />
          <CardTitle className="text-sm">Get set up</CardTitle>
          <span className="text-muted-foreground text-xs">
            {doneCount}/{steps.length}
          </span>
        </div>
        <Button variant="ghost" size="sm" className="h-7 px-2" onClick={dismiss}>
          <Icons.Close className="mr-1 h-3.5 w-3.5" />
          Dismiss
        </Button>
      </CardHeader>
      <CardContent>
        <ol className="grid gap-2 md:grid-cols-2 xl:grid-cols-5">
          {steps.map((step, index) => (
            <li
              key={step.label}
              className={`rounded-md border p-3 ${step.done ? "bg-muted/40" : ""}`}
            >
              <div className="flex items-start gap-2">
                {step.done ? (
                  <Icons.CheckCircle className="text-primary mt-0.5 h-4 w-4 shrink-0" />
                ) : (
                  <Icons.Circle className="text-muted-foreground mt-0.5 h-4 w-4 shrink-0" />
                )}
                <div className="min-w-0">
                  <p
                    className={`text-sm font-medium ${
                      step.done ? "text-muted-foreground line-through" : ""
                    }`}
                  >
                    {index + 1}. {step.label}
                  </p>
                  <p className="text-muted-foreground mt-0.5 text-xs">{step.description}</p>
                  {!step.done && (
                    <Button
                      variant="link"
                      size="sm"
                      className="mt-1 h-auto p-0 text-xs"
                      onClick={step.onAction}
                    >
                      {step.actionLabel}
                      <Icons.ArrowRight className="ml-1 h-3 w-3" />
                    </Button>
                  )}
                </div>
              </div>
            </li>
          ))}
        </ol>
      </CardContent>
    </Card>
  );
}
