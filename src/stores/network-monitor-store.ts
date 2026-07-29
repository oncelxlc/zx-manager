import { create } from "zustand";

import {
  clearNetworkUsage,
  getNetworkMonitorCapabilities,
  getNetworkMonitorStatus,
  queryNetworkUsage,
  setNetworkMonitorEnabled,
  setNetworkMonitorSampleInterval,
  subscribeNetworkRealtime,
  toNetworkMonitorError,
  type NetworkRealtimeSubscription,
} from "src/services/tauri/network-monitor";
import type {
  ClearNetworkUsageRequest,
  ClearNetworkUsageResult,
  NetworkMonitorCapabilities,
  NetworkMonitorCommandError,
  NetworkMonitorStatus,
  NetworkRealtimeEvent,
  NetworkUsageQuery,
  NetworkUsageResult,
} from "src/types/network-monitor";
import type { NetworkMonitorSampleInterval } from "src/types/preferences";

const realtimeLimit = 600;

interface NetworkMonitorStoreState {
  capabilities: NetworkMonitorCapabilities | null;
  status: NetworkMonitorStatus | null;
  realtime: NetworkRealtimeEvent[];
  history: NetworkUsageResult | null;
  loading: boolean;
  historyLoading: boolean;
  sampleIntervalLoading: boolean;
  stale: boolean;
  error: NetworkMonitorCommandError | null;
  initialize: () => Promise<void>;
  startRealtime: () => Promise<void>;
  stopRealtime: () => Promise<void>;
  setEnabled: (enabled: boolean) => Promise<boolean>;
  setSampleInterval: (
    interval: NetworkMonitorSampleInterval,
  ) => Promise<boolean>;
  refresh: () => Promise<void>;
  queryHistory: (request: NetworkUsageQuery) => Promise<void>;
  clearUsage: (
    request: ClearNetworkUsageRequest,
  ) => Promise<ClearNetworkUsageResult | null>;
  acceptRealtimeEvent: (event: NetworkRealtimeEvent) => void;
  reset: () => void;
}

let realtimeSubscription: NetworkRealtimeSubscription | null = null;
let statusRequestSequence = 0;
let historyRequestSequence = 0;

function initialState() {
  return {
    capabilities: null,
    status: null,
    realtime: [],
    history: null,
    loading: false,
    historyLoading: false,
    sampleIntervalLoading: false,
    stale: false,
    error: null,
  };
}

export const useNetworkMonitorStore = create<NetworkMonitorStoreState>(
  (set, get) => ({
    ...initialState(),

    initialize: async () => {
      const requestSequence = ++statusRequestSequence;
      set({ loading: true, error: null });
      try {
        const [capabilities, status] = await Promise.all([
          getNetworkMonitorCapabilities(),
          getNetworkMonitorStatus(),
        ]);
        if (requestSequence !== statusRequestSequence) {
          return;
        }
        const generationChanged =
          get().status !== null && get().status?.generation !== status.generation;
        set({
          capabilities,
          status,
          realtime: generationChanged ? [] : get().realtime,
          history: generationChanged ? null : get().history,
          loading: false,
          stale: false,
        });
      } catch (error) {
        if (requestSequence !== statusRequestSequence) {
          return;
        }
        set({
          loading: false,
          stale: get().status !== null,
          error: toNetworkMonitorError(error),
        });
      }
    },

    startRealtime: async () => {
      if (realtimeSubscription) {
        return;
      }
      try {
        const subscription = await subscribeNetworkRealtime((event) => {
          get().acceptRealtimeEvent(event);
        });
        realtimeSubscription = subscription;
        for (const event of subscription.subscription.initialEvents) {
          get().acceptRealtimeEvent(event);
        }
      } catch (error) {
        set({ error: toNetworkMonitorError(error) });
      }
    },

    stopRealtime: async () => {
      const subscription = realtimeSubscription;
      realtimeSubscription = null;
      if (!subscription) {
        return;
      }
      try {
        await subscription.cleanup();
      } catch (error) {
        set({ error: toNetworkMonitorError(error) });
      }
    },

    setEnabled: async (enabled) => {
      set({ loading: true, error: null });
      try {
        const status = await setNetworkMonitorEnabled(enabled);
        const generationChanged =
          get().status !== null && get().status?.generation !== status.generation;
        set({
          status,
          realtime: generationChanged ? [] : get().realtime,
          history: generationChanged ? null : get().history,
          loading: false,
          stale: false,
        });
        return true;
      } catch (error) {
        set({ loading: false, error: toNetworkMonitorError(error) });
        return false;
      }
    },

    setSampleInterval: async (interval) => {
      set({ sampleIntervalLoading: true, error: null });
      try {
        const status = await setNetworkMonitorSampleInterval(interval);
        set({
          status,
          sampleIntervalLoading: false,
          stale: false,
        });
        return true;
      } catch (error) {
        set({
          sampleIntervalLoading: false,
          error: toNetworkMonitorError(error),
        });
        return false;
      }
    },

    refresh: async () => {
      const requestSequence = ++statusRequestSequence;
      set({ loading: true, error: null });
      try {
        const status = await getNetworkMonitorStatus();
        if (requestSequence !== statusRequestSequence) {
          return;
        }
        set({ status, loading: false, stale: false });
      } catch (error) {
        if (requestSequence !== statusRequestSequence) {
          return;
        }
        set({
          loading: false,
          stale: get().status !== null,
          error: toNetworkMonitorError(error),
        });
      }
    },

    queryHistory: async (request) => {
      const requestSequence = ++historyRequestSequence;
      const generation = get().status?.generation;
      set({ historyLoading: true, error: null });
      try {
        const history = await queryNetworkUsage(request);
        if (
          requestSequence !== historyRequestSequence
          || (generation !== undefined && history.generation !== generation)
        ) {
          return;
        }
        set({ history, historyLoading: false, stale: false });
      } catch (error) {
        if (requestSequence !== historyRequestSequence) {
          return;
        }
        set({
          historyLoading: false,
          stale: get().history !== null,
          error: toNetworkMonitorError(error),
        });
      }
    },

    clearUsage: async (request) => {
      historyRequestSequence += 1;
      set({ loading: true, error: null });
      try {
        const result = await clearNetworkUsage(request);
        set((state) => ({
          status: state.status
            ? { ...state.status, generation: result.generation }
            : state.status,
          realtime: [],
          history: null,
          loading: false,
          stale: false,
        }));
        return result;
      } catch (error) {
        set({ loading: false, error: toNetworkMonitorError(error) });
        return null;
      }
    },

    acceptRealtimeEvent: (event) => {
      const state = get();
      const currentGeneration =
        state.status?.generation
        ?? state.realtime[state.realtime.length - 1]?.generation;
      if (currentGeneration !== undefined && event.generation < currentGeneration) {
        return;
      }
      const generationChanged =
        currentGeneration !== undefined && event.generation > currentGeneration;
      const currentRealtime = generationChanged ? [] : state.realtime;
      const latestSequence =
        currentRealtime[currentRealtime.length - 1]?.sequence ?? -1;
      if (!generationChanged && event.sequence <= latestSequence) {
        return;
      }
      const realtime = [...currentRealtime, event].slice(-realtimeLimit);
      set({
        realtime,
        history: generationChanged ? null : state.history,
        status: state.status
          ? {
              ...state.status,
              generation: event.generation,
              lastSampledAt: event.sampledAt,
            }
          : state.status,
      });
    },

    reset: () => {
      statusRequestSequence += 1;
      historyRequestSequence += 1;
      realtimeSubscription = null;
      set(initialState());
    },
  }),
);
