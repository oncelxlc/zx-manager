import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { IslandConfig, IslandState } from "./types";

export interface IslandDisplay {
  id: string;
  name: string;
  primary: boolean;
}

export interface RuntimeStatus {
  state: IslandState;
  sequence: number;
  lastError: string | null;
}

interface IslandEvents {
  "config-changed": IslandConfig;
  "state-changed": RuntimeStatus;
  "runtime-error": string;
  "display-changed": unknown;
  "position-changed": unknown;
}

export const islandApi = {
  isDesktop: () => typeof window !== "undefined" && isTauri() && /Windows/i.test(window.navigator.userAgent),
  config: () => invoke<IslandConfig>("island_get_config"),
  status: () => invoke<RuntimeStatus>("island_get_runtime_status"),
  displays: () => invoke<IslandDisplay[]>("island_get_displays"),
  save: (config: IslandConfig) => invoke<IslandConfig>("island_update_config", { config }),
  reset: (revision: number) => invoke<IslandConfig>("island_reset_config", { revision }),
  validatePreview: (config: IslandConfig) => invoke<void>("island_preview", { config }),
  show: () => invoke<void>("island_show"),
  hide: () => invoke<void>("island_hide"),
  restart: () => invoke<void>("island_restart"),
  pin: (pinned: boolean) => invoke<void>("island_set_pinned", { pinned }),
  interact: (event: "click" | "pointerEnter" | "pointerLeave") => invoke<void>("island_interact", { event }),
  drag: () => invoke<void>("island_drag"),
  ready: () => invoke<void>("island_ready"),
  listen<K extends keyof IslandEvents>(name: K, handler: (payload: IslandEvents[K]) => void) {
    return listen<IslandEvents[K]>(`island://${name}`, ({ payload }) => handler(payload));
  },
};
