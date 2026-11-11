import { create } from "zustand";

import {
  getNginxConfiguration,
  readNginxConfigGraph,
  readNginxConfigNode,
  toNginxCommandError,
  validateNginxConfiguration,
} from "src/services/tauri/nginx-manager";
import type {
  NginxCommandError,
  NginxConfigGraph,
  NginxConfigNodeDetail,
  NginxConfigValidationResult,
  NginxConfiguration,
} from "src/types/nginx";

type LoadStatus = "idle" | "loading" | "success" | "error";

interface NginxConfigurationState {
  configuration: NginxConfiguration | null;
  graph: NginxConfigGraph | null;
  selectedNode: NginxConfigNodeDetail | null;
  validation: NginxConfigValidationResult | null;
  selectedInstanceId: string | null;
  loadStatus: LoadStatus;
  error: NginxCommandError | null;
  load: (instanceId: string) => Promise<NginxConfiguration | null>;
  loadGraph: (instanceId: string) => Promise<NginxConfigGraph | null>;
  selectNode: (nodeId: string) => Promise<NginxConfigNodeDetail | null>;
  clearSelectedNode: () => void;
  validate: () => Promise<NginxConfigValidationResult | null>;
  clear: () => void;
}

let requestSequence = 0;

export const useNginxConfigurationStore = create<NginxConfigurationState>(
  (set) => ({
    configuration: null,
    graph: null,
    selectedNode: null,
    validation: null,
    selectedInstanceId: null,
    loadStatus: "idle",
    error: null,
    load: async (instanceId) => {
      const requestId = ++requestSequence;
      set({ selectedInstanceId: instanceId, loadStatus: "loading", error: null });
      try {
        const configuration = await getNginxConfiguration(instanceId);
        if (requestId === requestSequence) {
          set({ configuration, selectedNode: null, validation: null, loadStatus: "success" });
        }
        return configuration;
      } catch (error) {
        if (requestId === requestSequence) {
          set({
            configuration: null,
            graph: null,
            loadStatus: "error",
            error: toNginxCommandError(error),
          });
        }
        return null;
      }
    },
    loadGraph: async (instanceId) => {
      const requestId = ++requestSequence;
      set({ selectedInstanceId: instanceId, loadStatus: "loading", error: null });
      try {
        const graph = await readNginxConfigGraph(instanceId);
        if (requestId === requestSequence) {
          set({ graph, selectedNode: null, validation: null, loadStatus: "success" });
        }
        return graph;
      } catch (error) {
        if (requestId === requestSequence) {
          set({ graph: null, loadStatus: "error", error: toNginxCommandError(error) });
        }
        return null;
      }
    },
    selectNode: async (nodeId) => {
      const instanceId = useNginxConfigurationStore.getState().selectedInstanceId;
      if (!instanceId) return null;
      try {
        const selectedNode = await readNginxConfigNode(instanceId, nodeId);
        set({ selectedNode, error: null });
        return selectedNode;
      } catch (error) {
        set({ selectedNode: null, error: toNginxCommandError(error) });
        return null;
      }
    },
    clearSelectedNode: () => set({ selectedNode: null }),
    validate: async () => {
      const instanceId = useNginxConfigurationStore.getState().selectedInstanceId;
      if (!instanceId) return null;
      try {
        const validation = await validateNginxConfiguration(instanceId);
        set({ validation, error: null });
        return validation;
      } catch (error) {
        set({ validation: null, error: toNginxCommandError(error) });
        return null;
      }
    },
    clear: () => {
      requestSequence += 1;
      set({
        configuration: null,
        graph: null,
        selectedNode: null,
        validation: null,
        selectedInstanceId: null,
        loadStatus: "idle",
        error: null,
      });
    },
  }),
);

export function resetNginxConfigurationStore() {
  requestSequence = 0;
  useNginxConfigurationStore.getState().clear();
  requestSequence = 0;
}
