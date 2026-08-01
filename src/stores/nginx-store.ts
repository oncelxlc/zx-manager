import { create } from "zustand";

import {
  controlNginxInstance,
  getNginxRegistryState,
  inspectNginxDirectory,
  refreshNginxInstance,
  registerNginxInstance,
  resolveNginxRegistryMigration,
  selectNginxDirectory,
  subscribeNginxStatus,
  toNginxCommandError,
  unregisterNginxInstance,
  upgradeNginxInstance,
} from "src/services/tauri/nginx-manager";
import type {
  NginxAuthorizationLevel,
  NginxCommandError,
  NginxControlAction,
  NginxInspection,
  NginxInstance,
  NginxOperationPhase,
  NginxOperationRecord,
  NginxRegistryState,
  NginxStatusEvent,
  NginxUpgradeProgress,
  NginxUpgradeResult,
} from "src/types/nginx";

type LoadStatus = "idle" | "loading" | "success" | "error";

interface NginxState {
  registryState: NginxRegistryState | null;
  instance: NginxInstance | null;
  inspection: NginxInspection | null;
  observedAt: string | null;
  operationPhase: NginxOperationPhase | null;
  loadStatus: LoadStatus;
  operationStatus: LoadStatus;
  upgradeProgress: NginxUpgradeProgress | null;
  lastUpgradeResult: NginxUpgradeResult | null;
  lastOperation: NginxOperationRecord | null;
  error: NginxCommandError | null;
  loadRegistry: (force?: boolean) => Promise<NginxRegistryState | null>;
  ensureStatusSubscription: () => Promise<void>;
  selectAndInspect: () => Promise<NginxInspection | null>;
  registerInspection: (
    authorizationLevel: NginxAuthorizationLevel,
  ) => Promise<NginxInstance | null>;
  resolveMigration: (keepInstanceId: string) => Promise<boolean>;
  refreshInstance: () => Promise<NginxInstance | null>;
  unregisterInstance: () => Promise<boolean>;
  controlInstance: (action: NginxControlAction) => Promise<boolean>;
  upgradeInstance: (input: {
    channel: "stable" | "mainline";
    targetVersion: string;
    backupRetentionCount: number;
  }) => Promise<NginxUpgradeResult | null>;
  clearInspection: () => void;
}

let registryFlight: Promise<NginxRegistryState | null> | null = null;
let statusFlight: Promise<void> | null = null;
let statusCleanup: (() => Promise<void>) | null = null;
let requestSequence = 0;
let appliedSequence = 0;
let latestGeneration = -1;
let latestStatusSequence = -1;

function applyRegistry(registryState: NginxRegistryState) {
  useNginxStore.setState({
    registryState,
    instance: registryState.status === "ready" ? registryState.instance : null,
  });
}

function applyStatusEvent(event: NginxStatusEvent) {
  if (
    event.generation < latestGeneration
    || (event.generation === latestGeneration
      && event.sequence <= latestStatusSequence)
  ) {
    return;
  }
  latestGeneration = event.generation;
  latestStatusSequence = event.sequence;
  const state = useNginxStore.getState();
  useNginxStore.setState({
    instance: event.instance,
    observedAt: event.observedAt,
    operationPhase: event.operationPhase,
    registryState: state.registryState?.status === "migrationRequired"
      ? state.registryState
      : event.instance
        ? { status: "ready", instance: event.instance, migrationCandidates: [] }
        : { status: "empty", instance: null, migrationCandidates: [] },
  });
}

async function runRegistry(force: boolean) {
  if (!force && registryFlight) return registryFlight;
  const requestId = ++requestSequence;
  useNginxStore.setState({ loadStatus: "loading", error: null });
  const flight = getNginxRegistryState()
    .then((state) => {
      if (requestId >= appliedSequence) {
        appliedSequence = requestId;
        applyRegistry(state);
        useNginxStore.setState({ loadStatus: "success" });
      }
      return state;
    })
    .catch((error: unknown) => {
      if (requestId >= appliedSequence) {
        useNginxStore.setState({
          loadStatus: "error",
          error: toNginxCommandError(error),
        });
      }
      return null;
    })
    .finally(() => {
      if (registryFlight === flight) registryFlight = null;
    });
  registryFlight = flight;
  return flight;
}

export const useNginxStore = create<NginxState>((set, get) => ({
  registryState: null,
  instance: null,
  inspection: null,
  observedAt: null,
  operationPhase: null,
  loadStatus: "idle",
  operationStatus: "idle",
  upgradeProgress: null,
  lastUpgradeResult: null,
  lastOperation: null,
  error: null,
  loadRegistry: (force = false) => {
    if (!force && get().loadStatus === "success" && get().registryState) {
      return Promise.resolve(get().registryState);
    }
    return runRegistry(force);
  },
  ensureStatusSubscription: () => {
    if (statusCleanup) return Promise.resolve();
    if (statusFlight) return statusFlight;
    statusFlight = subscribeNginxStatus(applyStatusEvent)
      .then((result) => {
        statusCleanup = result.cleanup;
        applyStatusEvent(result.subscription.initialEvent);
      })
      .catch((error: unknown) => {
        set({ error: toNginxCommandError(error) });
      })
      .finally(() => {
        statusFlight = null;
      });
    return statusFlight;
  },
  selectAndInspect: async () => {
    if (get().registryState?.status !== "empty") return null;
    set({ operationStatus: "loading", error: null });
    try {
      const selection = await selectNginxDirectory("inspectInstance");
      if (!selection) {
        set({ operationStatus: "idle" });
        return null;
      }
      const inspection = await inspectNginxDirectory(selection.selectionId);
      set({ inspection, operationStatus: "success" });
      return inspection;
    } catch (error) {
      set({ operationStatus: "error", error: toNginxCommandError(error) });
      return null;
    }
  },
  registerInspection: async (authorizationLevel) => {
    const inspection = get().inspection;
    if (!inspection) return null;
    set({ operationStatus: "loading", error: null });
    try {
      const instance = await registerNginxInstance({
        inspectionId: inspection.inspectionId,
        authorizationLevel,
      });
      applyRegistry({ status: "ready", instance, migrationCandidates: [] });
      set({ inspection: null, operationStatus: "success" });
      return instance;
    } catch (error) {
      set({ operationStatus: "error", error: toNginxCommandError(error) });
      return null;
    }
  },
  resolveMigration: async (keepInstanceId) => {
    set({ operationStatus: "loading", error: null });
    try {
      applyRegistry(await resolveNginxRegistryMigration(keepInstanceId));
      set({ operationStatus: "success" });
      return true;
    } catch (error) {
      set({ operationStatus: "error", error: toNginxCommandError(error) });
      return false;
    }
  },
  refreshInstance: async () => {
    const instance = get().instance;
    if (!instance) return null;
    try {
      const refreshed = await refreshNginxInstance(instance.id);
      applyRegistry({ status: "ready", instance: refreshed, migrationCandidates: [] });
      return refreshed;
    } catch (error) {
      set({ error: toNginxCommandError(error) });
      return null;
    }
  },
  unregisterInstance: async () => {
    const instance = get().instance;
    if (!instance || get().operationStatus === "loading") return false;
    set({ operationStatus: "loading", error: null });
    try {
      await unregisterNginxInstance(instance.id);
      applyRegistry({ status: "empty", instance: null, migrationCandidates: [] });
      set({ operationStatus: "success" });
      return true;
    } catch (error) {
      set({ operationStatus: "error", error: toNginxCommandError(error) });
      return false;
    }
  },
  controlInstance: async (action) => {
    const instance = get().instance;
    if (!instance || get().operationStatus === "loading") return false;
    set({ operationStatus: "loading", error: null, lastOperation: null });
    try {
      const operation = await controlNginxInstance(instance.id, action);
      set({ operationStatus: "success", lastOperation: operation });
      return operation.success;
    } catch (error) {
      set({ operationStatus: "error", error: toNginxCommandError(error) });
      return false;
    }
  },
  upgradeInstance: async (input) => {
    const instance = get().instance;
    if (!instance || get().operationStatus === "loading") return null;
    set({
      operationStatus: "loading",
      error: null,
      upgradeProgress: null,
      lastUpgradeResult: null,
    });
    try {
      const result = await upgradeNginxInstance(
        { instanceId: instance.id, ...input },
        (upgradeProgress) => set({ upgradeProgress }),
      );
      set({ operationStatus: "success", lastUpgradeResult: result });
      await get().loadRegistry(true);
      return result;
    } catch (error) {
      set({ operationStatus: "error", error: toNginxCommandError(error) });
      return null;
    }
  },
  clearInspection: () => set({ inspection: null, operationStatus: "idle" }),
}));

export function resetNginxStore() {
  if (statusCleanup) void statusCleanup();
  registryFlight = null;
  statusFlight = null;
  statusCleanup = null;
  requestSequence = 0;
  appliedSequence = 0;
  latestGeneration = -1;
  latestStatusSequence = -1;
  useNginxStore.setState({
    registryState: null,
    instance: null,
    inspection: null,
    observedAt: null,
    operationPhase: null,
    loadStatus: "idle",
    operationStatus: "idle",
    upgradeProgress: null,
    lastUpgradeResult: null,
    lastOperation: null,
    error: null,
  });
}
