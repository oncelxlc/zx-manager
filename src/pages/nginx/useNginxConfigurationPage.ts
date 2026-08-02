import { useEffect, useRef } from "react";

import { useNginxConfigurationStore } from "src/stores/nginx-configuration-store";
import { useNginxStore } from "src/stores/nginx-store";

export function useNginxConfigurationPage(initialInstanceId?: string | null) {
  const requestedInstance = useRef(initialInstanceId);
  const instance = useNginxStore((state) => state.instance);
  const loadRegistry = useNginxStore((state) => state.loadRegistry);
  const selectedInstanceId = useNginxConfigurationStore(
    (state) => state.selectedInstanceId,
  );
  const load = useNginxConfigurationStore((state) => state.load);

  useEffect(() => {
    void loadRegistry().then((registry) => {
      if (registry?.status !== "ready" || !registry.instance.capabilities.canRead) return;
      const current = registry.instance;
      const requested = requestedInstance.current;
      requestedInstance.current = null;
      // Legacy URLs are accepted only when they still identify the singleton.
      if ((!requested || requested === current.id) && selectedInstanceId !== current.id) {
        void load(current.id);
      } else if (requested && requested !== current.id && selectedInstanceId !== current.id) {
        void load(current.id);
      }
    });
  }, [load, loadRegistry, selectedInstanceId]);

  return { instance, selectedInstanceId };
}
