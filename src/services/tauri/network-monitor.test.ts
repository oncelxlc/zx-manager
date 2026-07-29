import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke, isTauri, channels } = vi.hoisted(() => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
  channels: [] as Array<{ onmessage: (value: unknown) => void }>,
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
  isTauri,
  Channel: class {
    onmessage = (_value: unknown) => undefined;

    constructor() {
      channels.push(this);
    }
  },
}));

import {
  getNetworkMonitorCapabilities,
  setNetworkMonitorSampleInterval,
  subscribeNetworkRealtime,
} from "./network-monitor";

describe("network monitor Tauri service", () => {
  beforeEach(() => {
    invoke.mockReset();
    isTauri.mockReset();
    channels.length = 0;
  });

  it("returns a structured browser downgrade before invoking IPC", async () => {
    isTauri.mockReturnValue(false);

    await expect(getNetworkMonitorCapabilities()).rejects.toMatchObject({
      code: "unsupportedPlatform",
    });
    expect(invoke).not.toHaveBeenCalled();
  });

  it("subscribes through a Channel and exposes idempotent async cleanup", async () => {
    isTauri.mockReturnValue(true);
    invoke.mockImplementation(async (command: string) => {
      if (command === "subscribe_network_realtime") {
        return { subscriptionId: 42, generation: 0, initialEvents: [] };
      }
      return undefined;
    });
    const onMessage = vi.fn();

    const result = await subscribeNetworkRealtime(onMessage);
    channels[0]?.onmessage({ sequence: 1 });
    await result.cleanup();
    await result.cleanup();

    expect(onMessage).toHaveBeenCalledWith({ sequence: 1 });
    expect(invoke).toHaveBeenCalledWith(
      "subscribe_network_realtime",
      expect.objectContaining({ channel: expect.anything() }),
    );
    expect(invoke).toHaveBeenCalledWith("unsubscribe_network_realtime", {
      subscriptionId: 42,
    });
    expect(
      invoke.mock.calls.filter(([command]) => command === "unsubscribe_network_realtime"),
    ).toHaveLength(1);
  });

  it("sets the validated sample interval through the control command", async () => {
    isTauri.mockReturnValue(true);
    invoke.mockResolvedValue({ sampleIntervalSeconds: 10 });

    await setNetworkMonitorSampleInterval(10);

    expect(invoke).toHaveBeenCalledWith(
      "set_network_monitor_sample_interval",
      { sampleIntervalSeconds: 10 },
    );
  });
});
