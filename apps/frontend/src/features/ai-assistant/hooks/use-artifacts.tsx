/**
 * Artifact panel store.
 *
 * Holds the documents the assistant authors via the `create_artifact` tool,
 * scoped per thread. The `create_artifact` tool UI upserts an artifact when its
 * tool call resolves; the `ArtifactPane` (a sibling of the thread) reads the
 * active artifact and renders it. Both share this store as the single source of
 * truth, so chat and panel stay in sync. Latest write per artifact id wins —
 * re-rendering a persisted thread re-upserts the same ids idempotently.
 */

import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import type { Artifact } from "../types";

interface ThreadArtifacts {
  /** Artifact ids in first-seen order (panel tab order). */
  order: string[];
  items: Record<string, Artifact>;
}

interface ArtifactStore {
  upsertArtifact: (artifact: Artifact) => void;
  openArtifact: (threadId: string, id: string) => void;
  closeArtifact: (threadId: string) => void;
  setActiveArtifact: (threadId: string, id: string) => void;
  getThreadState: (threadId: string | null | undefined) => {
    artifacts: Artifact[];
    activeId: string | null;
    isOpen: boolean;
  };
}

const ArtifactContext = createContext<ArtifactStore | null>(null);

const EMPTY_STATE = { artifacts: [] as Artifact[], activeId: null, isOpen: false };

export function ArtifactProvider({ children }: { children: ReactNode }) {
  const [byThread, setByThread] = useState<Record<string, ThreadArtifacts>>({});
  const [activeByThread, setActiveByThread] = useState<Record<string, string | null>>({});
  const [openByThread, setOpenByThread] = useState<Record<string, boolean>>({});

  const upsertArtifact = useCallback((artifact: Artifact) => {
    const { threadId, id } = artifact;
    setByThread((prev) => {
      const current = prev[threadId] ?? { order: [], items: {} };
      const isNew = current.items[id] === undefined;
      return {
        ...prev,
        [threadId]: {
          order: isNew ? [...current.order, id] : current.order,
          items: { ...current.items, [id]: artifact },
        },
      };
    });
    // Default the active artifact to the first one seen, without forcing the
    // panel open (auto-open is the tool UI's call, only for live generations).
    setActiveByThread((prev) => (prev[threadId] ? prev : { ...prev, [threadId]: id }));
  }, []);

  const openArtifact = useCallback((threadId: string, id: string) => {
    setActiveByThread((prev) => ({ ...prev, [threadId]: id }));
    setOpenByThread((prev) => ({ ...prev, [threadId]: true }));
  }, []);

  const closeArtifact = useCallback((threadId: string) => {
    setOpenByThread((prev) => ({ ...prev, [threadId]: false }));
  }, []);

  const setActiveArtifact = useCallback((threadId: string, id: string) => {
    setActiveByThread((prev) => ({ ...prev, [threadId]: id }));
  }, []);

  const getThreadState = useCallback(
    (threadId: string | null | undefined) => {
      if (!threadId) return EMPTY_STATE;
      const thread = byThread[threadId];
      if (!thread || thread.order.length === 0) return EMPTY_STATE;
      const artifacts = thread.order.map((id) => thread.items[id]).filter(Boolean);
      const activeId = activeByThread[threadId] ?? artifacts[0]?.id ?? null;
      return { artifacts, activeId, isOpen: openByThread[threadId] ?? false };
    },
    [byThread, activeByThread, openByThread],
  );

  const value = useMemo<ArtifactStore>(
    () => ({ upsertArtifact, openArtifact, closeArtifact, setActiveArtifact, getThreadState }),
    [upsertArtifact, openArtifact, closeArtifact, setActiveArtifact, getThreadState],
  );

  return <ArtifactContext.Provider value={value}>{children}</ArtifactContext.Provider>;
}

export function useArtifactStore(): ArtifactStore {
  const ctx = useContext(ArtifactContext);
  if (!ctx) {
    throw new Error("useArtifactStore must be used within an ArtifactProvider");
  }
  return ctx;
}
