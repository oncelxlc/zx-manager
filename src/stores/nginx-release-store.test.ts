import { beforeEach, describe, expect, it, vi } from "vitest";

const service = vi.hoisted(() => ({
  checkNginxUpdates: vi.fn(),
  getNginxReleaseStatus: vi.fn(),
  toNginxCommandError: vi.fn((error: unknown) => ({
    code: "NGINX_UNKNOWN",
    message: String(error),
  })),
}));

vi.mock("src/services/tauri/nginx-manager", () => service);

import {
  resetNginxReleaseStore,
  useNginxReleaseStore,
} from "./nginx-release-store";
import type { NginxReleaseStatus } from "src/types/nginx";

function status(version: string): NginxReleaseStatus {
  return {
    channel: "stable",
    latestRelease: {
      version,
      downloadUrl: `https://nginx.org/download/nginx-${version}.tar.gz`,
      signatureUrl: `https://nginx.org/download/nginx-${version}.tar.gz.asc`,
    },
    checkedAt: "2026-07-31T00:00:00Z",
    stale: false,
    source: "cache",
    updateAvailableCount: 0,
    outdatedInstanceIds: [],
  };
}

describe("nginx release store", () => {
  beforeEach(() => {
    resetNginxReleaseStore();
    vi.clearAllMocks();
  });

  it("loads cached status without forcing a network refresh", async () => {
    service.getNginxReleaseStatus.mockResolvedValue(status("1.28.0"));

    await useNginxReleaseStore.getState().loadCached();

    expect(service.getNginxReleaseStatus).toHaveBeenCalledWith("stable");
    expect(service.checkNginxUpdates).not.toHaveBeenCalled();
  });

  it("passes persisted policy and explicit force to update checks", async () => {
    service.checkNginxUpdates.mockResolvedValue(status("1.28.0"));
    useNginxReleaseStore.getState().configure({
      releaseChannel: "mainline",
      updateCheckIntervalHours: 12,
      backupRetentionCount: 5,
      logFollow: true,
      logBufferLines: 20_000,
    });

    await useNginxReleaseStore.getState().check(true);

    expect(service.checkNginxUpdates).toHaveBeenCalledWith("mainline", true, 12);
  });
});
