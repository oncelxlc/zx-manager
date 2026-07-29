import { beforeEach, describe, expect, it, vi } from "vitest";

const service = vi.hoisted(() => ({
  setNetworkMonitorEnabled: vi.fn(),
  setNetworkMonitorSampleInterval: vi.fn(),
}));

vi.mock("src/services/tauri/network-monitor", () => service);

import { restoreNetworkMonitorOnStartup } from "./network-monitor-startup";

describe("network monitor startup restore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("applies the persisted interval before enabling launch monitoring", async () => {
    service.setNetworkMonitorSampleInterval.mockResolvedValue({});
    service.setNetworkMonitorEnabled.mockResolvedValue({});

    await restoreNetworkMonitorOnStartup({
      networkMonitorConfigured: true,
      networkMonitorStartOnLaunch: true,
      networkMonitorSampleIntervalSeconds: 10,
    });

    expect(service.setNetworkMonitorSampleInterval).toHaveBeenCalledWith(10);
    expect(service.setNetworkMonitorEnabled).toHaveBeenCalledWith(true);
    expect(
      service.setNetworkMonitorSampleInterval.mock.invocationCallOrder[0],
    ).toBeLessThan(service.setNetworkMonitorEnabled.mock.invocationCallOrder[0]!);
  });

  it("keeps startup non-blocking and still attempts the enabled preference", async () => {
    service.setNetworkMonitorSampleInterval.mockRejectedValue(
      new Error("interval unavailable"),
    );
    service.setNetworkMonitorEnabled.mockRejectedValue(
      new Error("collector unavailable"),
    );

    await expect(
      restoreNetworkMonitorOnStartup({
        networkMonitorConfigured: true,
        networkMonitorStartOnLaunch: true,
      }),
    ).resolves.toBeUndefined();
    expect(service.setNetworkMonitorEnabled).toHaveBeenCalledWith(true);
  });
});
