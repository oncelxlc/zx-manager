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
    restorationPromise = restoreNginxOnStartup(configuredPreferences)
      .catch(() => undefined);
  }
  return restorationPromise;
}

export function resetStartupBackgroundTasks() {
  configuredPreferences = {};
  restorationPromise = null;
}
