import {
  prepareNetworkMonitor,
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

  if (!(preferences.networkMonitorStartOnLaunch ?? true)) {
    return;
  }

  try {
    await prepareNetworkMonitor();
    await setNetworkMonitorEnabled(true);
  } catch {
    // The shell and manual monitoring controls remain available after a declined
    // UAC prompt or an automatic-start failure.
  }
}
