import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const store = vi.hoisted(() => {
  const interfaces = Array.from({ length: 25 }, (_, index) => ({
    id: `interface-${index}`,
    name: `Interface ${index}`,
    kind: "ethernet",
    state: "up",
    layer: "physical",
    isVirtual: false,
    tunnelType: null,
    traffic: {
      downloadBytesPerSecond: index,
      uploadBytesPerSecond: index,
      sessionDownloadBytes: index,
      sessionUploadBytes: index,
    },
    quality: "exact",
  }));
  const realtime = [{
    generation: 1,
    sequence: 1,
    sampledAt: Date.now(),
    elapsedMs: 5000,
    sampleState: "sample",
    device: {
      downloadBytesPerSecond: 100,
      uploadBytesPerSecond: 20,
      sessionDownloadBytes: 1000,
      sessionUploadBytes: 200,
    },
    interfaces,
    applications: [],
    proxyVpn: {
      proxyConfigured: false,
      proxyKinds: [],
      pacEnabled: false,
      vpnConnected: false,
      routeMode: null,
      virtualInterfaceIds: [],
      traffic: {
        downloadBytesPerSecond: null,
        uploadBytesPerSecond: null,
        sessionDownloadBytes: 0,
        sessionUploadBytes: 0,
      },
      quality: "unavailable",
    },
    warnings: [],
  }];
  const historyPoints = Array.from({ length: 10 }, (_, index) => ({
    from: index * 1000,
    to: (index + 1) * 1000,
    groupId: `group-${index}`,
    layer: "physical",
    downloadBytes: index,
    uploadBytes: index,
    quality: "exact",
  }));

  return {
    interfaces,
    state: {
      capabilities: null,
      status: {
        enabled: true,
        collectorState: "running",
        generation: 1,
        subscriberCount: 1,
        sampleIntervalSeconds: 5,
        databaseCreated: true,
        lastSampledAt: Date.now(),
        lastError: null,
      },
      realtime,
      history: {
        generation: 1,
        requestedFrom: 0,
        requestedTo: 10_000,
        actualFrom: 0,
        actualTo: 10_000,
        interval: "second",
        points: historyPoints,
        totalCount: 25,
        nextCursor: "10",
        partial: true,
        warnings: [],
      },
      loading: false,
      historyLoading: false,
      sampleIntervalLoading: false,
      stale: false,
      error: null,
      initialize: vi.fn(async () => undefined),
      startRealtime: vi.fn(async () => undefined),
      stopRealtime: vi.fn(async () => undefined),
      setEnabled: vi.fn(async () => true),
      setSampleInterval: vi.fn(async () => true),
      refresh: vi.fn(async () => undefined),
      queryHistory: vi.fn(async () => undefined),
      clearUsage: vi.fn(async () => null),
    },
  };
});

const storage = vi.hoisted(() => ({
  getPreferences: vi.fn(async () => ({
    networkMonitorConfigured: true,
    networkMonitorSampleIntervalSeconds: 5,
  })),
  setNetworkMonitorConfigured: vi.fn(async () => undefined),
  setNetworkMonitorSampleInterval: vi.fn(async () => undefined),
}));

vi.mock("src/stores/network-monitor-store", () => ({
  useNetworkMonitorStore: (
    selector: (state: typeof store.state) => unknown,
  ) => selector(store.state),
}));

vi.mock("src/services/storage/preferences-storage", () => storage);

vi.mock("src/layouts/MainLayout", () => ({
  useMainLayoutHeader: vi.fn(),
}));

vi.mock("@/components/ui/chart", () => ({
  ChartContainer: ({ children }: { children?: ReactNode }) => children,
  ChartTooltip: () => null,
  ChartTooltipContent: () => null,
}));

vi.mock("recharts", () => ({
  CartesianGrid: () => null,
  Line: () => null,
  LineChart: ({ children }: { children?: ReactNode }) => children,
  XAxis: () => null,
  YAxis: () => null,
}));

import { NetworkMonitorPage } from "./NetworkMonitorPage";

describe("NetworkMonitorPage tables", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    store.state.realtime[0].interfaces = store.interfaces;
    store.state.history.points = Array.from({ length: 10 }, (_, index) => ({
      from: index * 1000,
      to: (index + 1) * 1000,
      groupId: `group-${index}`,
      layer: "physical",
      downloadBytes: index,
      uploadBytes: index,
      quality: "exact",
    }));
    store.state.history.totalCount = 25;
  });

  it("uses fixed columns, a twenty-row scroll viewport, and independent pagination", async () => {
    const user = userEvent.setup();
    render(<NetworkMonitorPage />);

    const tables = screen.getAllByRole("table");
    expect(tables).toHaveLength(2);
    expect(tables[0]).toHaveClass("table-fixed", "min-w-[52rem]");
    expect(tables[1]).toHaveClass("table-fixed", "min-w-[50rem]");
    expect(tables[0]?.closest("[data-slot=table-container]")).toHaveClass(
      "max-h-[52.5rem]",
      "overflow-auto",
    );
    expect(screen.getByRole("columnheader", { name: "Kind" }))
      .toHaveClass("w-32");
    expect(screen.getByRole("columnheader", { name: "Time" }))
      .toHaveClass("w-48");
    expect(screen.getByTitle("Interface 0")).toBeInTheDocument();
    expect(screen.queryByTitle("Interface 10")).not.toBeInTheDocument();

    const secondPageButtons = screen.getAllByRole("button", {
      name: "Go to page 2",
    });
    await user.click(secondPageButtons[0]!);
    expect(screen.getByTitle("Interface 10")).toBeInTheDocument();

    await user.click(secondPageButtons[1]!);
    await waitFor(() => {
      expect(store.state.queryHistory).toHaveBeenCalledWith(
        expect.objectContaining({ cursor: "10", limit: 10 }),
      );
    });
  });

  it("keeps empty tables at the default ten-row height", () => {
    store.state.realtime[0].interfaces = [];
    store.state.history.points = [];
    store.state.history.totalCount = 0;

    render(<NetworkMonitorPage />);

    expect(screen.getByText("Waiting for the first real sample.").closest("tr"))
      .toHaveClass("h-[25rem]");
    expect(screen.getByText("No history is available for this range.").closest("tr"))
      .toHaveClass("h-[25rem]");
  });

  it("persists a successful sample interval change", async () => {
    const user = userEvent.setup();
    render(<NetworkMonitorPage />);

    await user.click(screen.getByRole("combobox", { name: "Sample interval" }));
    await user.click(await screen.findByRole("option", { name: "10 seconds" }));

    await waitFor(() => {
      expect(store.state.setSampleInterval).toHaveBeenCalledWith(10);
      expect(storage.setNetworkMonitorSampleInterval).toHaveBeenCalledWith(10);
    });
  });
});
