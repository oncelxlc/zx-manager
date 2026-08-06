import { beforeEach, describe, expect, it, vi } from "vitest";

const service = vi.hoisted(() => ({
  getNginxGlobalConfiguration: vi.fn(),
  validateNginxGlobalConfigurationPatch: vi.fn(),
  applyNginxGlobalConfigurationPatch: vi.fn(),
  toNginxCommandError: vi.fn((error: unknown) => error),
}));

vi.mock("src/services/tauri/nginx-manager", () => service);

import { useNginxGlobalConfigStore } from "./nginx-global-config-store";

const configuration = {
  instanceId: "instance",
  revision: { value: "revision", modifiedAt: null },
  workerProcesses: "1",
  workerRlimitNofile: null,
  pid: null,
  errorLog: "logs/error.log",
  topLevelIncludes: [],
  workerConnections: "1024",
  multiAccept: "off",
  acceptMutex: null,
  acceptMutexDelay: null,
};

describe("nginx global configuration store", () => {
  beforeEach(() => {
    useNginxGlobalConfigStore.getState().clear();
    vi.clearAllMocks();
    service.getNginxGlobalConfiguration.mockResolvedValue(configuration);
  });

  it("keeps a draft separate from the disk revision", async () => {
    await useNginxGlobalConfigStore.getState().load("instance");
    useNginxGlobalConfigStore.getState().update("workerProcesses", "auto");

    expect(useNginxGlobalConfigStore.getState().source?.workerProcesses).toBe("1");
    expect(useNginxGlobalConfigStore.getState().draft?.workerProcesses).toBe("auto");
  });

  it("preserves the draft when the backend reports a revision conflict", async () => {
    await useNginxGlobalConfigStore.getState().load("instance");
    useNginxGlobalConfigStore.getState().update("workerProcesses", "auto");
    service.applyNginxGlobalConfigurationPatch.mockRejectedValue({
      code: "NGINX_CONFIG_REVISION_CONFLICT",
      message: "conflict",
    });

    await useNginxGlobalConfigStore.getState().apply("reload");

    expect(useNginxGlobalConfigStore.getState().draft?.workerProcesses).toBe("auto");
    expect(useNginxGlobalConfigStore.getState().error).toMatchObject({
      code: "NGINX_CONFIG_REVISION_CONFLICT",
    });
  });
});
