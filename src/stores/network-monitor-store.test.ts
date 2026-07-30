import { beforeEach, describe, expect, it, vi } from "vitest";

const service = vi.hoisted(() => ({
  clearNetworkUsage: vi.fn(),
  getNetworkMonitorCapabilities: vi.fn(),
  getNetworkMonitorStatus: vi.fn(),
  queryNetworkUsage: vi.fn(),
  setNetworkMonitorEnabled: vi.fn(),
  setNetworkMonitorSampleInterval: vi.fn(),
  subscribeNetworkRealtime: vi.fn(),
  toNetworkMonitorError: vi.fn((error: unknown) => ({
    code: "unknown",
    message: String(error),
  })),
}));

vi.mock("src/services/tauri/network-monitor", () => service);

import { useNetworkMonitorStore } from "./network-monitor-store";
import type {
  NetworkRealtimeEvent,
  NetworkUsageResult,
} from "src/types/network-monitor";

function event(generation: number, sequence: number): NetworkRealtimeEvent {
  return {
    generation,
    sequence,
    sampledAt: sequence * 1000,
    elapsedMs: 1000,
    sampleState: "sample",
    applications: [{
      applicationId: "app",
      displayName: "app.exe",
      networkPath: "direct",
      traffic: {
        downloadBytesPerSecond: sequence,
        uploadBytesPerSecond: sequence,
        sessionDownloadBytes: sequence,
        sessionUploadBytes: sequence,
      },
      quality: "exact",
    }],
    unknownTraffic: {
      downloadBytesPerSecond: 0,
      uploadBytesPerSecond: 0,
      sessionDownloadBytes: 0,
      sessionUploadBytes: 0,
    },
    lostEvents: 0,
    unresolvedEvents: 0,
    warnings: [],
  };
}

function history(generation: number, applicationId: string): NetworkUsageResult {
  return {
    generation,
    requestedFrom: 0,
    requestedTo: 1000,
    actualFrom: 0,
    actualTo: 1000,
    points: [{
      applicationId,
      displayName: `${applicationId}.exe`,
      downloadBytes: 1,
      uploadBytes: 1,
      totalBytes: 2,
      includesUnknown: false,
      quality: "exact",
    }],
    totalCount: 1,
    nextCursor: null,
    partial: false,
    warnings: [],
  };
}

describe("network monitor store", () => {
  beforeEach(() => {
    useNetworkMonitorStore.getState().reset();
    vi.clearAllMocks();
    useNetworkMonitorStore.setState({
      status: {
        platformSupported: true,
      requiresElevation: true,
      authorizationReady: true,
        enabled: true,
        collectorState: "running",
        helperState: "running",
        generation: 1,
        subscriberCount: 1,
        sampleIntervalSeconds: 5,
        databaseCreated: false,
        lastSampledAt: null,
        lostEvents: 0,
        unresolvedEvents: 0,
        partialData: false,
        lastError: null,
      },
    });
  });

  it("drops out-of-order sequences and clears old data on generation changes", () => {
    const store = useNetworkMonitorStore.getState();
    store.acceptRealtimeEvent(event(1, 2));
    store.acceptRealtimeEvent(event(1, 1));
    expect(useNetworkMonitorStore.getState().realtime.map((item) => item.sequence))
      .toEqual([2]);

    store.acceptRealtimeEvent(event(2, 1));
    expect(useNetworkMonitorStore.getState().realtime).toEqual([event(2, 1)]);
    expect(useNetworkMonitorStore.getState().status?.generation).toBe(2);
  });

  it("keeps the realtime ring bounded to 600 samples", () => {
    const store = useNetworkMonitorStore.getState();
    for (let sequence = 1; sequence <= 605; sequence += 1) {
      store.acceptRealtimeEvent(event(1, sequence));
    }
    expect(useNetworkMonitorStore.getState().realtime).toHaveLength(600);
    expect(useNetworkMonitorStore.getState().realtime[0]?.sequence).toBe(6);
  });

  it("shares an in-flight realtime subscription and cleans it up after the final consumer", async () => {
    let resolveSubscription: ((value: {
      subscription: { subscriptionId: number; generation: number; initialEvents: NetworkRealtimeEvent[] };
      cleanup: () => Promise<void>;
    }) => void) | undefined;
    const cleanup = vi.fn(async () => undefined);
    service.subscribeNetworkRealtime.mockReturnValue(new Promise((resolve) => {
      resolveSubscription = resolve;
    }));

    const store = useNetworkMonitorStore.getState();
    const first = store.startRealtime();
    const second = store.startRealtime();
    expect(service.subscribeNetworkRealtime).toHaveBeenCalledOnce();

    resolveSubscription?.({
      subscription: { subscriptionId: 1, generation: 1, initialEvents: [] },
      cleanup,
    });
    await Promise.all([first, second]);

    await store.stopRealtime();
    expect(cleanup).not.toHaveBeenCalled();
    await store.stopRealtime();
    expect(cleanup).toHaveBeenCalledOnce();
  });

  it("ignores a slower history response after a newer query", async () => {
    let resolveFirst: ((value: NetworkUsageResult) => void) | undefined;
    let resolveSecond: ((value: NetworkUsageResult) => void) | undefined;
    service.queryNetworkUsage
      .mockReturnValueOnce(new Promise((resolve) => {
        resolveFirst = resolve;
      }))
      .mockReturnValueOnce(new Promise((resolve) => {
        resolveSecond = resolve;
      }));
    const request = {
      from: 0,
      to: 1000,
      networkPath: "all" as const,
      timeZone: "UTC",
    };

    const first = useNetworkMonitorStore.getState().queryHistory(request);
    const second = useNetworkMonitorStore.getState().queryHistory(request);
    resolveSecond?.(history(1, "new"));
    await second;
    resolveFirst?.(history(1, "old"));
    await first;

    expect(useNetworkMonitorStore.getState().history?.points[0]?.applicationId)
      .toBe("new");
  });

  it("invalidates history and realtime data after clearing", async () => {
    useNetworkMonitorStore.getState().acceptRealtimeEvent(event(1, 1));
    useNetworkMonitorStore.setState({ history: history(1, "existing") });
    service.clearNetworkUsage.mockResolvedValue({
      generation: 2,
      deletedBuckets: 1,
      clearedAt: 1000,
    });

    await useNetworkMonitorStore.getState().clearUsage({ scope: "all" });

    expect(useNetworkMonitorStore.getState().realtime).toEqual([]);
    expect(useNetworkMonitorStore.getState().history).toBeNull();
    expect(useNetworkMonitorStore.getState().status?.generation).toBe(2);
  });

  it("updates the sample interval without changing generation or cached data", async () => {
    useNetworkMonitorStore.getState().acceptRealtimeEvent(event(1, 1));
    service.setNetworkMonitorSampleInterval.mockResolvedValue({
      ...useNetworkMonitorStore.getState().status,
      sampleIntervalSeconds: 10,
    });

    await expect(
      useNetworkMonitorStore.getState().setSampleInterval(10),
    ).resolves.toBe(true);

    expect(service.setNetworkMonitorSampleInterval).toHaveBeenCalledWith(10);
    expect(useNetworkMonitorStore.getState().status?.sampleIntervalSeconds)
      .toBe(10);
    expect(useNetworkMonitorStore.getState().status?.generation).toBe(1);
    expect(useNetworkMonitorStore.getState().realtime).toHaveLength(1);
  });
});
