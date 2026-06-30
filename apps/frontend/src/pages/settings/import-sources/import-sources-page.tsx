import { useSourceEmails } from "@/hooks/use-provenance";
import { EmptyPlaceholder, Icons, Separator, Skeleton } from "@wealthfolio/ui";
import { SettingsHeader } from "../settings-header";

export default function ImportSourcesPage() {
  const { data: emails = [], isLoading } = useSourceEmails({ limit: 200 });

  return (
    <div className="space-y-6">
      <SettingsHeader
        heading="Import sources"
        text="Transaction emails the assistant saved as the source for imported activities."
      />
      <Separator />

      {isLoading ? (
        <div className="space-y-3">
          <Skeleton className="h-12" />
          <Skeleton className="h-12" />
        </div>
      ) : emails.length === 0 ? (
        <EmptyPlaceholder>
          <EmptyPlaceholder.Icon name="Mail" />
          <EmptyPlaceholder.Title>No saved source emails yet</EmptyPlaceholder.Title>
          <EmptyPlaceholder.Description>
            When the assistant uses an email to create an activity, it is saved here for
            traceability.
          </EmptyPlaceholder.Description>
        </EmptyPlaceholder>
      ) : (
        <div className="divide-border bg-card divide-y rounded-md border">
          {emails.map((email) => (
            <div key={email.id} className="flex items-start gap-3 p-4">
              <Icons.Mail className="text-muted-foreground mt-0.5 h-5 w-5 shrink-0" />
              <div className="min-w-0 flex-1 space-y-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="truncate font-medium">{email.subject ?? "(no subject)"}</span>
                  {email.sentAt && (
                    <span className="bg-muted text-muted-foreground rounded-md px-2 py-0.5 text-xs">
                      {email.sentAt.slice(0, 10)}
                    </span>
                  )}
                </div>
                <div className="text-muted-foreground flex flex-wrap items-center gap-1.5 text-sm">
                  {email.sender && <span>{email.sender}</span>}
                  {email.linkedActivityId && (
                    <>
                      <span>·</span>
                      <span>linked activity {email.linkedActivityId.slice(0, 8)}</span>
                    </>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
