import { render, waitFor } from "@testing-library/react";
import type { ComponentProps } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { CreateArtifactToolUIContentImpl } from "./create-artifact-tool-ui";

const runtimeState = vi.hoisted(() => ({
  currentThreadId: "thread-1" as string | null,
  isRunning: true,
}));

const artifactMocks = vi.hoisted(() => ({
  upsertArtifact: vi.fn(),
  openArtifact: vi.fn(),
  closeArtifact: vi.fn(),
  setActiveArtifact: vi.fn(),
  getThreadState: vi.fn(() => ({ artifacts: [], activeId: null, isOpen: false })),
}));

vi.mock("../../hooks/use-runtime-context", () => ({
  useRuntimeContext: () => runtimeState,
}));

vi.mock("../../hooks/use-artifacts", () => ({
  useArtifactStore: () => artifactMocks,
}));

type CreateArtifactToolUIProps = ComponentProps<typeof CreateArtifactToolUIContentImpl>;

function artifactProps(
  overrides: Partial<CreateArtifactToolUIProps> = {},
): CreateArtifactToolUIProps {
  return {
    args: {
      title: "Allocation Review",
      kind: "report",
      artifactId: "allocation-review",
      markdown: "# Allocation Review",
    },
    result: {
      artifactId: "allocation-review",
      kind: "report",
      title: "Allocation Review",
      status: "ready",
      message: "Opened",
    },
    status: { type: "complete" },
    toolCallId: "tool-call-1",
    ...overrides,
  } as CreateArtifactToolUIProps;
}

describe("CreateArtifactToolUIContentImpl", () => {
  beforeEach(() => {
    runtimeState.currentThreadId = "thread-1";
    runtimeState.isRunning = true;
    vi.clearAllMocks();
  });

  it("opens the panel when live artifact content arrives without a running status transition", async () => {
    render(<CreateArtifactToolUIContentImpl {...artifactProps()} />);

    await waitFor(() => {
      expect(artifactMocks.openArtifact).toHaveBeenCalledWith("thread-1", "allocation-review");
    });
    expect(artifactMocks.upsertArtifact).toHaveBeenCalled();
  });

  it("does not auto-open completed artifacts when reloading an old thread", async () => {
    runtimeState.isRunning = false;

    render(<CreateArtifactToolUIContentImpl {...artifactProps()} />);

    await waitFor(() => {
      expect(artifactMocks.upsertArtifact).toHaveBeenCalled();
    });
    expect(artifactMocks.openArtifact).not.toHaveBeenCalled();
  });
});
