import { useEffect, useRef } from "react";

import { useNginxConfigurationStore } from "src/stores/nginx-configuration-store";
import { useNginxStore } from "src/stores/nginx-store";

export function useNginxConfigurationPage(initialInstanceId?: string | null) {
  const requestedInstance = useRef(initialInstanceId);
  const instances = useNginxStore((state) => state.instances);
  const loadInstances = useNginxStore((state) => state.loadInstances);
  const selectedInstanceId = useNginxConfigurationStore(
    (state) => state.selectedInstanceId,
  );
  const load = useNginxConfigurationStore((state) => state.load);

  useEffect(() => {
    void loadInstances().then((loaded) => {
      const target = requestedInstance.current
        ?? selectedInstanceId
        ?? loaded?.find((instance) => instance.capabilities.canRead)?.id;
      requestedInstance.current = null;
      if (target && target !== selectedInstanceId) {
        void load(target);
      }
    });
  }, [load, loadInstances, selectedInstanceId]);

  return { instances, selectedInstanceId, load };
}
