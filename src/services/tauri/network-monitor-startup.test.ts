import { beforeEach, describe, expect, it, vi } from "vitest";

const service = vi.hoisted(() => ({
  prepareNetworkMonitor: vi.fn(),
  setNetworkMonitorEnabled: vi.fn(),
  setNetworkMonitorSampleInterval: vi.fn(),
}));

vi.mock("src/services/tauri/network-monitor", () => service);

import { restoreNetworkMonitorOnStartup } from "./network-monitor-startup";

describe("network monitor startup restore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("starts monitoring by default after authorization and interval restoration", async () => {
    service.prepareNetworkMonitor.mockResolvedValue({});
    service.setNetworkMonitorSampleInterval.mockResolvedValue({});
    service.setNetworkMonitorEnabled.mockResolvedValue({});

    await restoreNetworkMonitorOnStartup({
      networkMonitorSampleIntervalSeconds: 10,
    });

    expect(service.setNetworkMonitorSampleInterval).toHaveBeenCalledWith(10);
    expect(service.prepareNetworkMonitor).toHaveBeenCalledOnce();
    expect(service.setNetworkMonitorEnabled).toHaveBeenCalledWith(true);
    expect(
      service.setNetworkMonitorSampleInterval.mock.invocationCallOrder[0],
    ).toBeGreaterThan(service.prepareNetworkMonitor.mock.invocationCallOrder[0]!);
    expect(
      service.setNetworkMonitorEnabled.mock.invocationCallOrder[0],
    ).toBeGreaterThan(service.setNetworkMonitorSampleInterval.mock.invocationCallOrder[0]!);
  });

  it("does not start monitoring when launch preference is disabled", async () => {
    service.prepareNetworkMonitor.mockResolvedValue({});
    service.setNetworkMonitorSampleInterval.mockResolvedValue({});

    await restoreNetworkMonitorOnStartup({ networkMonitorStartOnLaunch: false });

    expect(service.setNetworkMonitorEnabled).not.toHaveBeenCalled();
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
    expect(service.setNetworkMonitorEnabled).not.toHaveBeenCalled();
  });
});
