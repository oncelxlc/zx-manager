import { beforeEach, describe, expect, it, vi } from "vitest";

const service = vi.hoisted(() => ({
  prepareNetworkMonitor: vi.fn(),
  setNetworkMonitorSampleInterval: vi.fn(),
}));

vi.mock("src/services/tauri/network-monitor", () => service);

import { restoreNetworkMonitorOnStartup } from "./network-monitor-startup";

describe("network monitor startup restore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("prepares authorization before applying the persisted interval", async () => {
    service.prepareNetworkMonitor.mockResolvedValue({});
    service.setNetworkMonitorSampleInterval.mockResolvedValue({});

    await restoreNetworkMonitorOnStartup({
      networkMonitorSampleIntervalSeconds: 10,
    });

    expect(service.setNetworkMonitorSampleInterval).toHaveBeenCalledWith(10);
    expect(service.prepareNetworkMonitor).toHaveBeenCalledOnce();
    expect(
      service.setNetworkMonitorSampleInterval.mock.invocationCallOrder[0],
    ).toBeGreaterThan(service.prepareNetworkMonitor.mock.invocationCallOrder[0]!);
  });

  it("keeps authorization and preference restoration non-blocking", async () => {
    service.prepareNetworkMonitor.mockRejectedValue(new Error("consent cancelled"));
    service.setNetworkMonitorSampleInterval.mockRejectedValue(
      new Error("interval unavailable"),
    );

    await expect(
      restoreNetworkMonitorOnStartup({
      }),
    ).resolves.toBeUndefined();
    expect(service.prepareNetworkMonitor).toHaveBeenCalledOnce();
  });
});
