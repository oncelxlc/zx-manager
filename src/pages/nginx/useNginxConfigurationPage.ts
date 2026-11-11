import { useEffect, useRef } from "react";

import { useNginxConfigurationStore } from "src/stores/nginx-configuration-store";
import { useNginxStore } from "src/stores/nginx-store";

export function useNginxConfigurationPage(
  initialInstanceId?: string | null,
  mode: "legacy" | "graph" = "legacy",
) {
  const requestedInstance = useRef(initialInstanceId);
  const instance = useNginxStore((state) => state.instance);
  const loadRegistry = useNginxStore((state) => state.loadRegistry);
  const selectedInstanceId = useNginxConfigurationStore(
    (state) => state.selectedInstanceId,
  );
  const load = useNginxConfigurationStore((state) => state.load);
  const loadGraph = useNginxConfigurationStore((state) => state.loadGraph);
  const configuration = useNginxConfigurationStore((state) => state.configuration);
  const graph = useNginxConfigurationStore((state) => state.graph);

  useEffect(() => {
    void loadRegistry().then((registry) => {
      if (registry?.status !== "ready" || !registry.instance.capabilities.canRead) return;
      const current = registry.instance;
      const requested = requestedInstance.current;
      requestedInstance.current = null;
      // Legacy URLs are accepted only when they still identify the singleton.
      const hasCurrentData = selectedInstanceId === current.id
        && (mode === "graph" ? graph !== null : configuration !== null);
      if (hasCurrentData) return;
      // A legacy instance query may only identify the registered singleton.
      if (requested && requested !== current.id) requestedInstance.current = null;
      void (mode === "graph" ? loadGraph(current.id) : load(current.id));
    });
  }, [configuration, graph, load, loadGraph, loadRegistry, mode, selectedInstanceId]);

  return { instance, selectedInstanceId };
}
