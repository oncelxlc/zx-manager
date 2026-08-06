import { create } from "zustand";

import { getPreferences } from "src/services/storage/preferences-storage";
import {
  listNginxLogSources,
  readNginxLogPage,
  subscribeNginxLog,
  toNginxCommandError,
} from "src/services/tauri/nginx-manager";
import { defaultNginxPreferences } from "src/types/preferences";
import type { NginxCommandError, NginxLogEvent, NginxLogLine, NginxLogSource } from "src/types/nginx";

interface State {
  instanceId: string | null;
  sources: NginxLogSource[];
  selectedSourceId: string | null;
  lines: NginxLogLine[];
  nextCursor: string | null;
  following: boolean;
  status: "idle" | "loading" | "error";
  error: NginxCommandError | null;
  loadSources: (instanceId: string) => Promise<void>;
  selectSource: (sourceId: string) => Promise<void>;
  loadMore: () => Promise<void>;
  stop: () => Promise<void>;
  clear: () => Promise<void>;
}

let cleanupTail: (() => Promise<void>) | null = null;
let generation = 0;
let sequence = 0;
let bufferLimit = defaultNginxPreferences.logBufferLines;

function appendEvent(event: NginxLogEvent) {
  if (event.generation < generation || (event.generation === generation && event.sequence <= sequence)) return;
  generation = event.generation;
  sequence = event.sequence;
  useNginxLogStore.setState((state) => ({
    lines: [...state.lines, ...event.lines].slice(-bufferLimit),
  }));
}

export const useNginxLogStore = create<State>((set, get) => ({
  instanceId: null,
  sources: [],
  selectedSourceId: null,
  lines: [],
  nextCursor: null,
  following: false,
  status: "idle",
  error: null,
  loadSources: async (instanceId) => {
    set({ instanceId, status: "loading", error: null });
    try {
      const [sources, preferences] = await Promise.all([listNginxLogSources(instanceId), getPreferences()]);
      const nginx = preferences.nginx ?? defaultNginxPreferences;
      bufferLimit = nginx.logBufferLines;
      set({ sources, status: "idle" });
      const first = sources.find((source) => source.availability === "available");
      if (first) await get().selectSource(first.id);
    } catch (error) {
      set({ status: "error", error: toNginxCommandError(error) });
    }
  },
  selectSource: async (sourceId) => {
    await get().stop();
    const { instanceId } = get();
    if (!instanceId) return;
    set({ selectedSourceId: sourceId, lines: [], nextCursor: null, status: "loading" });
    try {
      const page = await readNginxLogPage(instanceId, sourceId, null);
      set({ lines: page.lines.slice(-bufferLimit), nextCursor: page.nextCursor, status: "idle" });
      const preferences = await getPreferences();
      if ((preferences.nginx ?? defaultNginxPreferences).logFollow) {
        const tail = await subscribeNginxLog(instanceId, sourceId, appendEvent);
        generation = tail.subscription.initialEvent.generation;
        sequence = tail.subscription.initialEvent.sequence;
        cleanupTail = tail.cleanup;
        set({ following: true });
      }
    } catch (error) {
      set({ status: "error", error: toNginxCommandError(error) });
    }
  },
  loadMore: async () => {
    const { instanceId, selectedSourceId, nextCursor } = get();
    if (!instanceId || !selectedSourceId || !nextCursor) return;
    const page = await readNginxLogPage(instanceId, selectedSourceId, nextCursor);
    set((state) => ({
      lines: [...state.lines, ...page.lines].slice(-bufferLimit),
      nextCursor: page.nextCursor,
    }));
  },
  stop: async () => {
    const cleanup = cleanupTail;
    cleanupTail = null;
    if (cleanup) await cleanup();
    set({ following: false });
  },
  clear: async () => {
    await get().stop();
    generation = 0;
    sequence = 0;
    set({ instanceId: null, sources: [], selectedSourceId: null, lines: [], nextCursor: null, status: "idle", error: null });
  },
}));
