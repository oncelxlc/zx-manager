import { beforeEach, describe, expect, it, vi } from "vitest";

const stores = vi.hoisted(() => ({
  configure: vi.fn(),
  loadRegistry: vi.fn(),
  ensureStatusSubscription: vi.fn(),
  check: vi.fn(),
}));

vi.mock("src/stores/nginx-release-store", () => ({
  useNginxReleaseStore: {
    getState: () => ({ configure: stores.configure, check: stores.check }),
  },
}));
vi.mock("src/stores/nginx-store", () => ({
  useNginxStore: {
    getState: () => ({
      loadRegistry: stores.loadRegistry,
      ensureStatusSubscription: stores.ensureStatusSubscription,
    }),
  },
}));

import { restoreNginxOnStartup } from "./nginx-startup";

describe("Nginx startup restoration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    stores.loadRegistry.mockResolvedValue(null);
    stores.ensureStatusSubscription.mockResolvedValue(undefined);
    stores.check.mockResolvedValue(null);
  });

  it("restores the registry and performs a non-forced cached update check", async () => {
    const preferences = {
      releaseChannel: "mainline" as const,
      updateCheckIntervalHours: 12,
      backupRetentionCount: 10,
      logFollow: false,
      logBufferLines: 50_000,
    };

    await restoreNginxOnStartup({ nginx: preferences });

    expect(stores.configure).toHaveBeenCalledWith(preferences);
    expect(stores.loadRegistry).toHaveBeenCalledWith(true);
    expect(stores.ensureStatusSubscription).toHaveBeenCalledOnce();
    expect(stores.check).toHaveBeenCalledWith(false);
  });
});
