import { restoreNetworkMonitorOnStartup } from "src/services/tauri/network-monitor-startup";
import { restoreNginxOnStartup } from "src/services/tauri/nginx-startup";
import type { UserPreferences } from "src/types/preferences";

let configuredPreferences: Partial<UserPreferences> = {};
let restorationPromise: Promise<void> | null = null;

export function configureStartupBackgroundTasks(
  preferences: Partial<UserPreferences>,
) {
  configuredPreferences = preferences;
}

export function restoreStartupBackgroundTasks(): Promise<void> {
  if (!restorationPromise) {
    restorationPromise = Promise.allSettled([
      restoreNetworkMonitorOnStartup(configuredPreferences),
      restoreNginxOnStartup(configuredPreferences),
    ]).then(() => undefined);
  }
  return restorationPromise;
}

export function resetStartupBackgroundTasks() {
  configuredPreferences = {};
  restorationPromise = null;
}
