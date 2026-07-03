import { act, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it } from "vitest";

import type { Artifact } from "../types";
import { ArtifactProvider, useArtifactStore } from "./use-artifacts";

function makeArtifact(overrides: Partial<Artifact> = {}): Artifact {
  return {
    id: "tool-call-1",
    toolCallId: "tool-call-1",
    threadId: "thread-1",
    kind: "report",
    title: "Allocation Review",
    markdown: "# Partial",
    ...overrides,
  };
}

function renderStore() {
  return renderHook(() => useArtifactStore(), {
    wrapper: ({ children }: { children: ReactNode }) => (
      <ArtifactProvider>{children}</ArtifactProvider>
    ),
  });
}

describe("useArtifactStore renameArtifact", () => {
  it("re-keys an artifact in place, keeping tab order and the active pointer", () => {
    const { result } = renderStore();

    act(() => {
      result.current.upsertArtifact(makeArtifact({ id: "other", toolCallId: "other" }));
      result.current.upsertArtifact(makeArtifact());
      result.current.openArtifact("thread-1", "tool-call-1");
    });

    act(() => {
      result.current.renameArtifact("thread-1", "tool-call-1", "generated-id");
    });

    const state = result.current.getThreadState("thread-1");
    expect(state.artifacts.map((a) => a.id)).toEqual(["other", "generated-id"]);
    expect(state.activeId).toBe("generated-id");
  });

  it("drops the stale slot when the new id already exists", () => {
    const { result } = renderStore();

    act(() => {
      result.current.upsertArtifact(makeArtifact({ id: "allocation-review" }));
      result.current.upsertArtifact(makeArtifact({ id: "tool-call-2", toolCallId: "tool-call-2" }));
    });

    act(() => {
      result.current.renameArtifact("thread-1", "tool-call-2", "allocation-review");
    });

    const state = result.current.getThreadState("thread-1");
    expect(state.artifacts.map((a) => a.id)).toEqual(["allocation-review"]);
  });

  it("ignores renames for unknown ids", () => {
    const { result } = renderStore();

    act(() => {
      result.current.upsertArtifact(makeArtifact());
    });

    act(() => {
      result.current.renameArtifact("thread-1", "missing", "generated-id");
    });

    const state = result.current.getThreadState("thread-1");
    expect(state.artifacts.map((a) => a.id)).toEqual(["tool-call-1"]);
  });
});
