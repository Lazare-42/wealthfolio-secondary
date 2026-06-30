import { Icons } from "@wealthfolio/ui/components/ui/icons";

import type { Artifact, CreateArtifactArgs, CreateArtifactOutput } from "../../types";
import { ArtifactMarkdown } from "./artifact-markdown";
import { ArtifactTable } from "./artifact-table";

/** Per-kind label + icon for the panel header and inline chip. */
export const ARTIFACT_KIND_META = {
  report: { label: "Report", icon: Icons.FileText },
  table: { label: "Table", icon: Icons.FileSpreadsheet },
} as const;

/**
 * Build the normalized panel artifact from a `create_artifact` tool call. The
 * document content lives in the tool ARGS; the OUTPUT only supplies the
 * canonical id (the model may reuse an `artifactId` slug to revise a document).
 */
export function buildArtifact(
  args: CreateArtifactArgs | undefined,
  output: CreateArtifactOutput | undefined,
  toolCallId: string,
  threadId: string,
): Artifact | null {
  if (!args?.title || !args.kind) return null;
  const id = args.artifactId?.trim() || output?.artifactId?.trim() || toolCallId;
  return {
    id,
    toolCallId,
    threadId,
    kind: args.kind,
    title: args.title,
    summary: args.summary,
    markdown: args.markdown,
    table: args.table,
  };
}

/** Render an artifact's body by kind. */
export function ArtifactBody({ artifact }: { artifact: Artifact }) {
  if (artifact.kind === "table" && artifact.table) {
    return <ArtifactTable table={artifact.table} />;
  }
  if (artifact.kind === "report" && artifact.markdown) {
    return <ArtifactMarkdown markdown={artifact.markdown} />;
  }
  return <p className="text-muted-foreground text-sm">This document has no content yet.</p>;
}
