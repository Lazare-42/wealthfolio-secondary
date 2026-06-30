import { useState } from "react";

import { Badge, Button } from "@wealthfolio/ui";
import { Icons } from "@wealthfolio/ui/components/ui/icons";
import { Tooltip, TooltipContent, TooltipTrigger } from "@wealthfolio/ui/components/ui/tooltip";

import { cn } from "@/lib/utils";
import { useArtifactStore } from "../hooks/use-artifacts";
import { useRuntimeContext } from "../hooks/use-runtime-context";
import type { Artifact } from "../types";
import { ArtifactBody, ARTIFACT_KIND_META } from "./artifacts/registry";

/**
 * Header affordance to re-open the document panel after the user closes it,
 * shown only when the current thread has artifacts but the panel is hidden.
 */
export function ArtifactReopenButton() {
  const runtime = useRuntimeContext();
  const threadId = runtime.currentThreadId;
  const { getThreadState, openArtifact } = useArtifactStore();
  const { artifacts, activeId, isOpen } = getThreadState(threadId);

  if (!threadId || isOpen || artifacts.length === 0) return null;
  const targetId = activeId ?? artifacts[0].id;

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="size-9 shrink-0"
          onClick={() => openArtifact(threadId, targetId)}
        >
          <Icons.FileText className="size-4" />
          <span className="sr-only">Show document panel</span>
        </Button>
      </TooltipTrigger>
      <TooltipContent side="bottom">
        {artifacts.length > 1 ? `${artifacts.length} documents` : "Show document"}
      </TooltipContent>
    </Tooltip>
  );
}

/** Serialize an artifact to clipboard text: markdown for reports, TSV for tables. */
function artifactToText(artifact: Artifact): string {
  if (artifact.kind === "report") return artifact.markdown ?? "";
  if (artifact.kind === "table" && artifact.table) {
    const { columns, rows } = artifact.table;
    const header = columns.map((c) => c.label).join("\t");
    const body = rows
      .map((row) => columns.map((c) => String(row[c.key] ?? "")).join("\t"))
      .join("\n");
    return `${header}\n${body}`;
  }
  return "";
}

/**
 * Side panel that renders the assistant's authored documents next to the chat.
 * Renders nothing when the current thread has no open artifact, so the chat
 * fills the width until the assistant creates one (two-pane on demand).
 */
export function ArtifactPane() {
  const runtime = useRuntimeContext();
  const threadId = runtime.currentThreadId;
  const { getThreadState, closeArtifact, setActiveArtifact } = useArtifactStore();
  const { artifacts, activeId, isOpen } = getThreadState(threadId);
  const [isCopied, setIsCopied] = useState(false);

  if (!isOpen || !threadId || artifacts.length === 0) return null;

  const active = artifacts.find((a) => a.id === activeId) ?? artifacts[0];
  const meta = ARTIFACT_KIND_META[active.kind];
  const KindIcon = meta.icon;

  const handleCopy = () => {
    const text = artifactToText(active);
    if (!text) return;
    navigator.clipboard.writeText(text).then(() => {
      setIsCopied(true);
      setTimeout(() => setIsCopied(false), 2000);
    });
  };

  return (
    <aside
      className={cn(
        // Full-screen overlay on mobile, fixed-width side column on desktop.
        "bg-background fixed inset-0 z-50 flex flex-col border-l",
        "md:static md:inset-auto md:z-auto md:w-[44%] md:min-w-[380px] md:max-w-[640px]",
      )}
    >
      <header className="flex shrink-0 items-center gap-2 border-b px-4 py-2.5">
        <KindIcon className="text-muted-foreground size-4 shrink-0" />
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-semibold">{active.title}</p>
          {active.summary ? (
            <p className="text-muted-foreground truncate text-xs">{active.summary}</p>
          ) : null}
        </div>
        <Badge variant="secondary" className="shrink-0">
          {meta.label}
        </Badge>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon" className="size-8 shrink-0" onClick={handleCopy}>
              {isCopied ? <Icons.Check className="size-4" /> : <Icons.Copy className="size-4" />}
              <span className="sr-only">Copy document</span>
            </Button>
          </TooltipTrigger>
          <TooltipContent>Copy</TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="size-8 shrink-0"
              onClick={() => closeArtifact(threadId)}
            >
              <Icons.X className="size-4" />
              <span className="sr-only">Close document panel</span>
            </Button>
          </TooltipTrigger>
          <TooltipContent>Close</TooltipContent>
        </Tooltip>
      </header>

      {artifacts.length > 1 ? (
        <div className="flex shrink-0 gap-1 overflow-x-auto border-b px-3 py-2">
          {artifacts.map((artifact) => (
            <button
              key={artifact.id}
              type="button"
              onClick={() => setActiveArtifact(threadId, artifact.id)}
              className={cn(
                "whitespace-nowrap rounded-md px-2.5 py-1 text-xs transition-colors",
                artifact.id === active.id
                  ? "bg-muted font-medium"
                  : "text-muted-foreground hover:bg-muted/50",
              )}
            >
              {artifact.title}
            </button>
          ))}
        </div>
      ) : null}

      <div className="min-h-0 flex-1 overflow-y-auto p-4">
        <ArtifactBody artifact={active} />
      </div>
    </aside>
  );
}
