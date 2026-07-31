import { beforeEach, describe, expect, it, vi } from "vitest";

const service = vi.hoisted(() => ({
  inspectNginxDirectory: vi.fn(),
  listNginxInstances: vi.fn(),
  refreshNginxInstance: vi.fn(),
  registerNginxInstance: vi.fn(),
  selectNginxDirectory: vi.fn(),
  toNginxCommandError: vi.fn((error: unknown) => ({
    code: "NGINX_UNKNOWN",
    message: String(error),
  })),
  unregisterNginxInstance: vi.fn(),
}));

vi.mock("src/services/tauri/nginx-manager", () => service);

import { resetNginxStore, useNginxStore } from "./nginx-store";
import type { NginxInstance } from "src/types/nginx";

function instance(id: string): NginxInstance {
  return {
    id,
    name: id,
    kind: "external",
    rootPath: `C:\\${id}`,
    binaryPath: `C:\\${id}\\nginx.exe`,
    configPath: null,
    authorizedRoots: [`C:\\${id}`],
    authorizationLevel: "readOnly",
    version: "1.28.0",
    configureArguments: [],
    binaryFingerprint: "hash",
    providerIdentity: { provider: "portable", externalId: null },
    controlBackend: "portable",
    lifecycleState: "available",
    runtimeStatus: "unknown",
    capabilities: {
      canRead: true,
      canEdit: false,
      canControl: false,
      canUnregister: true,
    },
    createdAt: "2026-07-31T00:00:00Z",
    updatedAt: "2026-07-31T00:00:00Z",
  };
}

describe("nginx store", () => {
  beforeEach(() => {
    resetNginxStore();
    vi.clearAllMocks();
  });

  it("prevents an older instance request from replacing a newer result", async () => {
    let resolveFirst!: (value: NginxInstance[]) => void;
    let resolveSecond!: (value: NginxInstance[]) => void;
    service.listNginxInstances
      .mockReturnValueOnce(new Promise((resolve) => { resolveFirst = resolve; }))
      .mockReturnValueOnce(new Promise((resolve) => { resolveSecond = resolve; }));

    const first = useNginxStore.getState().loadInstances(true);
    const second = useNginxStore.getState().loadInstances(true);
    resolveSecond([instance("new")]);
    await second;
    resolveFirst([instance("old")]);
    await first;

    expect(useNginxStore.getState().instances.map(({ id }) => id)).toEqual(["new"]);
  });

  it("does not inspect when the native picker is cancelled", async () => {
    service.selectNginxDirectory.mockResolvedValue(null);

    await useNginxStore.getState().selectAndInspect();

    expect(service.inspectNginxDirectory).not.toHaveBeenCalled();
    expect(useNginxStore.getState().operationStatus).toBe("idle");
  });
});
