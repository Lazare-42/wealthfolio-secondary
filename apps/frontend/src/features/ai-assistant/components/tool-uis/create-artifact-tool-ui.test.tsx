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
  renameArtifact: vi.fn(),
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

  it("re-keys the store entry when the resolved id changes after streaming", async () => {
    // While streaming, args carry no artifactId yet -> id falls back to the
    // toolCallId.
    const { rerender } = render(
      <CreateArtifactToolUIContentImpl
        {...artifactProps({
          args: { title: "Allocation Review", kind: "report", markdown: "# Partial" },
          result: undefined,
          status: { type: "running" },
        })}
      />,
    );

    await waitFor(() => {
      expect(artifactMocks.upsertArtifact).toHaveBeenCalledWith(
        expect.objectContaining({ id: "tool-call-1" }),
      );
    });
    expect(artifactMocks.renameArtifact).not.toHaveBeenCalled();

    // On completion the backend-generated artifactId takes over.
    rerender(
      <CreateArtifactToolUIContentImpl
        {...artifactProps({
          args: { title: "Allocation Review", kind: "report", markdown: "# Allocation Review" },
          result: {
            artifactId: "generated-id",
            kind: "report",
            title: "Allocation Review",
            status: "ready",
            message: "Opened",
          },
        })}
      />,
    );

    await waitFor(() => {
      expect(artifactMocks.renameArtifact).toHaveBeenCalledWith(
        "thread-1",
        "tool-call-1",
        "generated-id",
      );
    });
    expect(artifactMocks.upsertArtifact).toHaveBeenLastCalledWith(
      expect.objectContaining({ id: "generated-id" }),
    );
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
