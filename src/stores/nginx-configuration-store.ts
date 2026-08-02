import { create } from "zustand";

import {
  getNginxConfiguration,
  toNginxCommandError,
} from "src/services/tauri/nginx-manager";
import type { NginxCommandError, NginxConfiguration } from "src/types/nginx";

type LoadStatus = "idle" | "loading" | "success" | "error";

interface NginxConfigurationState {
  configuration: NginxConfiguration | null;
  selectedInstanceId: string | null;
  loadStatus: LoadStatus;
  error: NginxCommandError | null;
  load: (instanceId: string) => Promise<NginxConfiguration | null>;
  clear: () => void;
}

let requestSequence = 0;

export const useNginxConfigurationStore = create<NginxConfigurationState>(
  (set) => ({
    configuration: null,
    selectedInstanceId: null,
    loadStatus: "idle",
    error: null,
    load: async (instanceId) => {
      const requestId = ++requestSequence;
      set({ selectedInstanceId: instanceId, loadStatus: "loading", error: null });
      try {
        const configuration = await getNginxConfiguration(instanceId);
        if (requestId === requestSequence) {
          set({ configuration, loadStatus: "success" });
        }
        return configuration;
      } catch (error) {
        if (requestId === requestSequence) {
          set({
            configuration: null,
            loadStatus: "error",
            error: toNginxCommandError(error),
          });
        }
        return null;
      }
    },
    clear: () => {
      requestSequence += 1;
      set({
        configuration: null,
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
