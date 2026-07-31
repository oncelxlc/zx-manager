import { create } from "zustand";

import {
  inspectNginxDirectory,
  listNginxInstances,
  refreshNginxInstance,
  registerNginxInstance,
  selectNginxDirectory,
  toNginxCommandError,
  unregisterNginxInstance,
} from "src/services/tauri/nginx-manager";
import type {
  NginxAuthorizationLevel,
  NginxCommandError,
  NginxInspection,
  NginxInstance,
} from "src/types/nginx";

type LoadStatus = "idle" | "loading" | "success" | "error";

interface NginxState {
  instances: NginxInstance[];
  inspection: NginxInspection | null;
  loadStatus: LoadStatus;
  operationStatus: LoadStatus;
  error: NginxCommandError | null;
  loadInstances: (force?: boolean) => Promise<NginxInstance[] | null>;
  selectAndInspect: () => Promise<NginxInspection | null>;
  registerInspection: (
    name: string,
    authorizationLevel: NginxAuthorizationLevel,
  ) => Promise<NginxInstance | null>;
  refreshInstance: (instanceId: string) => Promise<NginxInstance | null>;
  unregisterInstance: (instanceId: string) => Promise<boolean>;
  clearInspection: () => void;
}

let listFlight: Promise<NginxInstance[] | null> | null = null;
let requestSequence = 0;
let appliedSequence = 0;

function applyInstance(instance: NginxInstance) {
  const instances = useNginxStore.getState().instances;
  const index = instances.findIndex((candidate) => candidate.id === instance.id);
  useNginxStore.setState({
    instances:
      index < 0
        ? [...instances, instance]
        : instances.map((candidate) =>
          candidate.id === instance.id ? instance : candidate,
        ),
  });
}

async function runList(force: boolean): Promise<NginxInstance[] | null> {
  if (!force && listFlight) {
    return listFlight;
  }
  const requestId = ++requestSequence;
  useNginxStore.setState({ loadStatus: "loading", error: null });
  const flight = listNginxInstances()
    .then((instances) => {
      if (requestId >= appliedSequence) {
        appliedSequence = requestId;
        useNginxStore.setState({ instances, loadStatus: "success" });
      }
      return instances;
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
      if (listFlight === flight) {
        listFlight = null;
      }
    });
  listFlight = flight;
  return flight;
}

export const useNginxStore = create<NginxState>((set, get) => ({
  instances: [],
  inspection: null,
  loadStatus: "idle",
  operationStatus: "idle",
  error: null,
  loadInstances: (force = false) => {
    if (!force && get().loadStatus === "success") {
      return Promise.resolve(get().instances);
    }
    return runList(force);
  },
  selectAndInspect: async () => {
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
  registerInspection: async (name, authorizationLevel) => {
    const inspection = get().inspection;
    if (!inspection) {
      return null;
    }
    set({ operationStatus: "loading", error: null });
    try {
      const instance = await registerNginxInstance({
        inspectionId: inspection.inspectionId,
        name,
        authorizationLevel,
      });
      applyInstance(instance);
      set({ inspection: null, operationStatus: "success" });
      return instance;
    } catch (error) {
      set({ operationStatus: "error", error: toNginxCommandError(error) });
      return null;
    }
  },
  refreshInstance: async (instanceId) => {
    try {
      const instance = await refreshNginxInstance(instanceId);
      applyInstance(instance);
      return instance;
    } catch (error) {
      set({ error: toNginxCommandError(error) });
      return null;
    }
  },
  unregisterInstance: async (instanceId) => {
    set({ operationStatus: "loading", error: null });
    try {
      await unregisterNginxInstance(instanceId);
      set({
        instances: get().instances.filter((instance) => instance.id !== instanceId),
        operationStatus: "success",
      });
      return true;
    } catch (error) {
      set({ operationStatus: "error", error: toNginxCommandError(error) });
      return false;
    }
  },
  clearInspection: () => set({ inspection: null, operationStatus: "idle" }),
}));

export function resetNginxStore() {
  listFlight = null;
  requestSequence = 0;
  appliedSequence = 0;
  useNginxStore.setState({
    instances: [],
    inspection: null,
    loadStatus: "idle",
    operationStatus: "idle",
    error: null,
  });
}
