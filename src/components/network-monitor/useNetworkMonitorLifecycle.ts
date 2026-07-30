import { useEffect, useState } from "react";

import { getPreferences } from "src/services/storage/preferences-storage";

interface NetworkMonitorLifecycleActions {
  initialize: () => Promise<void>;
  startRealtime: () => Promise<void>;
  stopRealtime: () => Promise<void>;
}

export function useNetworkMonitorLifecycle({
  initialize,
  startRealtime,
  stopRealtime,
}: NetworkMonitorLifecycleActions) {
  const [configured, setConfigured] = useState<boolean | null>(null);

  useEffect(() => {
    let active = true;
    void getPreferences().then((preferences) => {
      if (active) {
        setConfigured(
          preferences.networkMonitorStartOnLaunch
          ?? preferences.networkMonitorConfigured
          ?? true,
        );
      }
    });
    // Establish the Channel after the initial manager snapshot. Starting both
    // invokes concurrently can leave the page subscribed before startup has
    // completed, which yields an empty stream until the route is remounted.
    void initialize().finally(() => {
      if (active) {
        void startRealtime();
      }
    });
    return () => {
      active = false;
      void stopRealtime();
    };
  }, [initialize, startRealtime, stopRealtime]);

  return { configured, setConfigured };
}
