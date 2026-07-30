import {
  prepareNetworkMonitor,
  setNetworkMonitorSampleInterval,
} from "src/services/tauri/network-monitor";
import type { UserPreferences } from "src/types/preferences";

export async function restoreNetworkMonitorOnStartup(
  preferences: Partial<UserPreferences>,
): Promise<void> {
  try {
    await prepareNetworkMonitor();
  } catch {
    // The shell and manual monitoring controls remain available after a declined UAC prompt.
  }

  try {
    await setNetworkMonitorSampleInterval(
      preferences.networkMonitorSampleIntervalSeconds ?? 5,
    );
  } catch {
    // The manager keeps its safe five-second default if preference restore fails.
  }

}
