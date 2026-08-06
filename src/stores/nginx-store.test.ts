import { beforeEach, describe, expect, it, vi } from "vitest";

const service = vi.hoisted(() => ({
  controlNginxInstance: vi.fn(),
  getNginxRegistryState: vi.fn(),
  inspectNginxDirectory: vi.fn(),
  refreshNginxInstance: vi.fn(),
  registerNginxInstance: vi.fn(),
  resolveNginxRegistryMigration: vi.fn(),
  selectNginxDirectory: vi.fn(),
  subscribeNginxStatus: vi.fn(),
  toNginxCommandError: vi.fn((error: unknown) => ({
    code: "NGINX_UNKNOWN",
    message: String(error),
  })),
  unregisterNginxInstance: vi.fn(),
  upgradeNginxInstance: vi.fn(),
}));

vi.mock("src/services/tauri/nginx-manager", () => service);

import { resetNginxStore, useNginxStore } from "./nginx-store";
import type { NginxInstance, NginxRegistryState } from "src/types/nginx";

function instance(id: string): NginxInstance {
  return {
    id,
    name: "Nginx",
    kind: "external",
    rootPath: `C:\\${id}`,
    binaryPath: `C:\\${id}\\nginx.exe`,
    configPath: null,
    authorizedRoots: [`C:\\${id}`],
    authorizationLevel: "full",
    version: "1.28.0",
    configureArguments: [],
    binaryFingerprint: "hash",
    providerIdentity: { provider: "portable", externalId: null },
    controlBackend: "portable",
    lifecycleState: "available",
    runtimeStatus: "running",
    capabilities: {
      canRead: true,
      canEdit: true,
      canControl: true,
      canUnregister: true,
    },
    createdAt: "2026-07-31T00:00:00Z",
    updatedAt: "2026-07-31T00:00:00Z",
  };
}

const ready = (id: string): NginxRegistryState => ({
  status: "ready",
  instance: instance(id),
  migrationCandidates: [],
});

describe("nginx store", () => {
  beforeEach(() => {
    resetNginxStore();
    vi.clearAllMocks();
  });

  it("prevents an older registry request from replacing a newer result", async () => {
    let resolveFirst!: (value: NginxRegistryState) => void;
    let resolveSecond!: (value: NginxRegistryState) => void;
    service.getNginxRegistryState
      .mockReturnValueOnce(new Promise((resolve) => { resolveFirst = resolve; }))
      .mockReturnValueOnce(new Promise((resolve) => { resolveSecond = resolve; }));

    const first = useNginxStore.getState().loadRegistry(true);
    const second = useNginxStore.getState().loadRegistry(true);
    resolveSecond(ready("new"));
    await second;
    resolveFirst(ready("old"));
    await first;

    expect(useNginxStore.getState().instance?.id).toBe("new");
  });

  it("does not inspect when the native picker is cancelled", async () => {
    useNginxStore.setState({
      registryState: { status: "empty", instance: null, migrationCandidates: [] },
    });
    service.selectNginxDirectory.mockResolvedValue(null);
    await useNginxStore.getState().selectAndInspect();
    expect(service.inspectNginxDirectory).not.toHaveBeenCalled();
    expect(useNginxStore.getState().operationStatus).toBe("idle");
  });

  it("uses the singleton id for a real fixed control action", async () => {
    useNginxStore.setState({ instance: instance("controlled") });
    service.controlNginxInstance.mockResolvedValue({ success: true });
    const success = await useNginxStore.getState().controlInstance("reload");
    expect(success).toBe(true);
    expect(service.controlNginxInstance).toHaveBeenCalledWith("controlled", "reload");
  });

  it("discards an older status event after a newer sequence", async () => {
    service.subscribeNginxStatus.mockImplementation(async (onMessage) => {
      onMessage({ generation: 2, sequence: 2, observedAt: "new", instance: instance("new"), runtimeDetails: null, operationPhase: null });
      onMessage({ generation: 2, sequence: 1, observedAt: "old", instance: instance("old"), runtimeDetails: null, operationPhase: null });
      return {
        subscription: { subscriptionId: 1, initialEvent: { generation: 1, sequence: 9, observedAt: "older", instance: instance("older"), runtimeDetails: null, operationPhase: null } },
        cleanup: vi.fn(),
      };
    });
    await useNginxStore.getState().ensureStatusSubscription();
    expect(useNginxStore.getState().instance?.id).toBe("new");
  });
});
