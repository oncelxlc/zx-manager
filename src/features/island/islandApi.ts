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

export type NotificationPermission = "disabled" | "unavailable" | "notRequested" | "denied" | "allowed" | "error";

export interface NotificationStatus {
  sequence: number;
  permission: NotificationPermission;
}

export interface MediaStatus {
  title: string;
  artist: string;
  playing: boolean;
  positionMs: number;
  durationMs: number;
  canPrevious: boolean;
  canNext: boolean;
  canToggle: boolean;
  canSeek: boolean;
  battery: [number, boolean] | null;
  cover: { width: number; height: number; bgra: number[] } | null;
}

interface IslandEvents {
  "media-changed": null;
  "config-changed": IslandConfig;
  "state-changed": RuntimeStatus;
  "notifications-changed": NotificationStatus;
  "runtime-error": string;
  "display-changed": unknown;
  "position-changed": unknown;
}

export const islandApi = {
  isDesktop: () => typeof window !== "undefined" && isTauri() && /Windows/i.test(window.navigator.userAgent),
  config: () => invoke<IslandConfig>("island_get_config"),
  status: () => invoke<RuntimeStatus>("island_get_runtime_status"),
  notifications: () => invoke<NotificationStatus>("island_get_notifications"),
  media: () => invoke<MediaStatus>("island_get_media"),
  requestNotificationAccess: () => invoke<void>("island_request_notification_access"),
  openNotificationSettings: () => invoke<void>("island_open_notification_settings"),
  displays: () => invoke<IslandDisplay[]>("island_get_displays"),
  save: (config: IslandConfig) => invoke<IslandConfig>("island_update_config", { config }),
  validatePreview: (config: IslandConfig) => invoke<void>("island_preview", { config }),
  show: () => invoke<void>("island_show"),
  hide: () => invoke<void>("island_hide"),
  restart: () => invoke<void>("island_restart"),
  pin: (pinned: boolean) => invoke<void>("island_set_pinned", { pinned }),
  listen<K extends keyof IslandEvents>(name: K, handler: (payload: IslandEvents[K]) => void) {
    return listen<IslandEvents[K]>(`island://${name}`, ({ payload }) => handler(payload));
  },
};
