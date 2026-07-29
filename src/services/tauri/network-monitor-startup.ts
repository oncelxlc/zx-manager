import {
  setNetworkMonitorEnabled,
  setNetworkMonitorSampleInterval,
} from "src/services/tauri/network-monitor";
import type { UserPreferences } from "src/types/preferences";

export async function restoreNetworkMonitorOnStartup(
  preferences: Partial<UserPreferences>,
): Promise<void> {
  try {
    await setNetworkMonitorSampleInterval(
      preferences.networkMonitorSampleIntervalSeconds ?? 5,
    );
  } catch {
    // The manager keeps its safe five-second default if preference restore fails.
  }

  if (
    preferences.networkMonitorConfigured
    && preferences.networkMonitorStartOnLaunch
  ) {
    try {
      await setNetworkMonitorEnabled(true);
    } catch {
      // Monitoring is optional; startup failures must not block the shell.
    }
  }
}
