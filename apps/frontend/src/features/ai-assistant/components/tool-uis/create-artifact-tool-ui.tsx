import type { ToolCallMessagePartProps } from "@assistant-ui/react";
import { makeAssistantToolUI } from "@assistant-ui/react";
import { Button, Card, CardContent } from "@wealthfolio/ui";
import { Icons } from "@wealthfolio/ui/components/ui/icons";
import { memo, useEffect, useRef } from "react";

import { useArtifactStore } from "../../hooks/use-artifacts";
import { useRuntimeContext } from "../../hooks/use-runtime-context";
import type { CreateArtifactArgs, CreateArtifactOutput } from "../../types";
import { ARTIFACT_KIND_META, buildArtifact } from "../artifacts/registry";

type CreateArtifactToolUIContentProps = ToolCallMessagePartProps<
  CreateArtifactArgs,
  CreateArtifactOutput
>;

function CreateArtifactLoadingState() {
  return (
    <Card className="bg-muted/40 border-primary/10 w-full overflow-hidden">
      <CardContent className="flex items-center gap-3 py-4">
        <div className="bg-primary/10 flex h-9 w-9 shrink-0 items-center justify-center rounded-full">
          <Icons.FileText className="text-primary h-4 w-4 animate-pulse" />
        </div>
        <p className="text-sm font-medium">Writing document...</p>
        <Icons.Spinner className="text-muted-foreground ml-auto h-4 w-4 shrink-0 animate-spin" />
      </CardContent>
    </Card>
  );
}

export function CreateArtifactToolUIContentImpl({
  args,
  result,
  status,
  toolCallId,
}: CreateArtifactToolUIContentProps) {
  const runtime = useRuntimeContext();
  const threadId = runtime.currentThreadId;
  const isLiveRun = runtime.isRunning;
  const { upsertArtifact, openArtifact } = useArtifactStore();

  const wasRunning = useRef(false);
  const autoOpened = useRef(false);
  const hasOpenedForContent = useRef(false);
  if (status?.type === "running") wasRunning.current = true;

  const artifact = threadId ? buildArtifact(args, result, toolCallId, threadId) : null;
  const artifactKey = artifact
    ? `${artifact.id}:${args?.markdown ?? ""}:${JSON.stringify(args?.table ?? null)}`
    : null;
  const hasArtifactContent = Boolean(args?.markdown?.trim() || args?.table);

  // Register the artifact in the panel store whenever its content changes.
  useEffect(() => {
    if (artifact) upsertArtifact(artifact);
    // artifactKey captures id + content; rebuilding on every keystroke of a
    // streamed arg is intended (live updates into the panel).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [artifactKey]);

  // Auto-open for live documents. Some providers/tool adapters surface complete
  // tool parts without a visible "running" transition, so content arrival is
  // also treated as a live signal.
  useEffect(() => {
    if (!artifact || !threadId) return;
    const completedLive = status?.type === "complete" && wasRunning.current;
    const contentArrivedLive = isLiveRun && hasArtifactContent;
    if (
      !autoOpened.current &&
      (completedLive || (contentArrivedLive && !hasOpenedForContent.current))
    ) {
      autoOpened.current = true;
      hasOpenedForContent.current = true;
      openArtifact(threadId, artifact.id);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [status?.type, artifactKey, threadId, hasArtifactContent, isLiveRun]);

  if (status?.type === "running" && !artifact) return <CreateArtifactLoadingState />;
  if (!artifact) return null;

  const meta = ARTIFACT_KIND_META[artifact.kind];
  const KindIcon = meta.icon;

  return (
    <Card className="bg-card w-full overflow-hidden">
      <CardContent className="flex items-center gap-3 py-3">
        <div className="bg-primary/10 flex h-9 w-9 shrink-0 items-center justify-center rounded-full">
          <KindIcon className="text-primary h-4 w-4" />
        </div>
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium">{artifact.title}</p>
          <p className="text-muted-foreground truncate text-xs">
            {artifact.summary ?? `${meta.label} · opens in the document panel`}
          </p>
        </div>
        <Button
          size="sm"
          variant="outline"
          className="shrink-0"
          onClick={() => threadId && openArtifact(threadId, artifact.id)}
        >
          Open
        </Button>
      </CardContent>
    </Card>
  );
}

const CreateArtifactToolUIContent = memo(CreateArtifactToolUIContentImpl);

export const CreateArtifactToolUI = makeAssistantToolUI<CreateArtifactArgs, CreateArtifactOutput>({
  toolName: "create_artifact",
  render: (props) => <CreateArtifactToolUIContent {...props} />,
});
