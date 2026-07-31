import { beforeEach, describe, expect, it, vi } from "vitest";

const service = vi.hoisted(() => ({
  getNginxConfiguration: vi.fn(),
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
import type { NginxConfiguration } from "src/types/nginx";

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

describe("nginx configuration store", () => {
  beforeEach(() => {
    resetNginxConfigurationStore();
    vi.clearAllMocks();
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
});
