import { beforeEach, describe, expect, it, vi } from "vitest";

const service = vi.hoisted(() => ({
  getNginxConfiguration: vi.fn(),
  readNginxConfigGraph: vi.fn(),
  readNginxConfigNode: vi.fn(),
  validateNginxConfiguration: vi.fn(),
  toNginxCommandError: vi.fn((error: unknown) => ({
    code: "NGINX_UNKNOWN",
    message: String(error),
  })),
}));

vi.mock("src/services/tauri/nginx-manager", () => service);

import {
  resetNginxConfigurationStore,
  useNginxConfigurationStore,
} from "./nginx-configuration-store";
import type { NginxConfigGraph, NginxConfiguration } from "src/types/nginx";

function configuration(instanceId: string): NginxConfiguration {
  return {
    instanceId,
    entrySourceId: "source",
    sources: [],
    diagnostics: [],
    sites: [],
    upstreams: [],
    topologyNodes: [],
    topologyEdges: [],
    pidPath: null,
    accessLogs: [],
    errorLogs: [],
  };
}

function graph(instanceId: string): NginxConfigGraph {
  return {
    instanceId,
    entrySourceId: "source",
    revision: { value: "revision" },
    sources: [],
    nodes: [],
    diagnostics: [],
  };
}

describe("nginx configuration store", () => {
  beforeEach(() => {
    resetNginxConfigurationStore();
    vi.clearAllMocks();
    service.readNginxConfigGraph.mockImplementation(async (instanceId: string) => graph(instanceId));
  });

  it("prevents an older configuration request from replacing a newer result", async () => {
    let resolveFirst!: (value: NginxConfiguration) => void;
    let resolveSecond!: (value: NginxConfiguration) => void;
    service.getNginxConfiguration
      .mockReturnValueOnce(new Promise((resolve) => { resolveFirst = resolve; }))
      .mockReturnValueOnce(new Promise((resolve) => { resolveSecond = resolve; }));

    const first = useNginxConfigurationStore.getState().load("old");
    const second = useNginxConfigurationStore.getState().load("new");
    resolveSecond(configuration("new"));
    await second;
    resolveFirst(configuration("old"));
    await first;

    expect(useNginxConfigurationStore.getState().configuration?.instanceId).toBe("new");
  });

  it("loads the lightweight graph without requesting full source text", async () => {
    await useNginxConfigurationStore.getState().loadGraph("instance");

    expect(useNginxConfigurationStore.getState().graph?.instanceId).toBe("instance");
    expect(service.getNginxConfiguration).not.toHaveBeenCalled();
  });
});
