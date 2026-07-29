import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const store = vi.hoisted(() => {
  const applications = Array.from({ length: 25 }, (_, index) => ({
    applicationId: `application-${index}`,
    displayName: `App ${index}.exe`,
    networkPath: (["proxy", "direct", "unknown"] as const)[index % 3]!,
    traffic: {
      downloadBytesPerSecond: index,
      uploadBytesPerSecond: index,
      sessionDownloadBytes: index,
      sessionUploadBytes: index,
    },
    quality: index % 3 === 2 ? "partial" as const : "exact" as const,
  }));
  const realtime = [{
    generation: 1,
    sequence: 1,
    sampledAt: Date.now(),
    elapsedMs: 5000,
    sampleState: "sample" as const,
    applications,
    unknownTraffic: {
      downloadBytesPerSecond: 2,
      uploadBytesPerSecond: 2,
      sessionDownloadBytes: 2,
      sessionUploadBytes: 2,
    },
    lostEvents: 0,
    unresolvedEvents: 1,
    warnings: [],
  }];
  const historyPoints = Array.from({ length: 10 }, (_, index) => ({
    applicationId: `history-${index}`,
    displayName: `History ${index}.exe`,
    downloadBytes: index,
    uploadBytes: index,
    totalBytes: index * 2,
    includesUnknown: false,
    quality: "exact" as const,
  }));

  return {
    applications,
    historyPoints,
    state: {
      capabilities: {
        platform: "windows",
        platformSupported: true,
        requiresElevation: true,
        applicationTraffic: true,
        proxyClassification: true,
        historyStorage: true,
        retentionDays: 7,
      },
      status: {
        platformSupported: true,
        requiresElevation: true,
        enabled: true,
        collectorState: "running" as const,
        helperState: "running" as const,
        generation: 1,
        subscriberCount: 1,
        sampleIntervalSeconds: 5,
        databaseCreated: true,
        lastSampledAt: Date.now(),
        lostEvents: 0,
        unresolvedEvents: 0,
        partialData: false,
        lastError: null,
      },
      realtime,
      history: {
        generation: 1,
        requestedFrom: 0,
        requestedTo: 10_000,
        actualFrom: 0,
        actualTo: 10_000,
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

describe("NetworkMonitorPage application traffic", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    store.state.capabilities.platformSupported = true;
    store.state.realtime[0].applications = store.applications;
    store.state.history.points = store.historyPoints;
    store.state.history.totalCount = 25;
    store.state.error = null;
  });

  it("uses application columns, bounded table viewports, and independent pagination", async () => {
    const user = userEvent.setup();
    render(<NetworkMonitorPage />);

    const tables = screen.getAllByRole("table");
    expect(tables).toHaveLength(2);
    expect(tables[0]).toHaveClass("table-fixed", "min-w-224");
    expect(tables[1]).toHaveClass("table-fixed", "min-w-176");
    expect(tables[0]?.closest("[data-slot=table-container]")).toHaveClass(
      "max-h-[52.5rem]",
      "overflow-auto",
    );
    expect(screen.getByRole("columnheader", { name: "Download rate" }))
      .toHaveClass("w-40");
    expect(screen.getByRole("columnheader", { name: "Total" }))
      .toHaveClass("w-40");
    expect(screen.getByTitle("App 24.exe")).toBeInTheDocument();
    expect(screen.queryByTitle("App 12.exe")).not.toBeInTheDocument();

    const secondPageButtons = screen.getAllByRole("button", {
      name: "Go to page 2",
    });
    await user.click(secondPageButtons[0]!);
    expect(screen.getByTitle("App 12.exe")).toBeInTheDocument();

    await user.click(secondPageButtons[1]!);
    await waitFor(() => {
      expect(store.state.queryHistory).toHaveBeenCalledWith(
        expect.objectContaining({ cursor: "10", limit: 10 }),
      );
    });
  });

  it("keeps empty application and history tables at the default height", () => {
    store.state.realtime[0].applications = [];
    store.state.history.points = [];
    store.state.history.totalCount = 0;

    render(<NetworkMonitorPage />);

    expect(screen.getByText("Waiting for the first real sample.").closest("tr"))
      .toHaveClass("h-100");
    expect(screen.getByText("No history is available for this range.").closest("tr"))
      .toHaveClass("h-100");
  });

  it("keeps realtime and history path selectors independent", async () => {
    const user = userEvent.setup();
    render(<NetworkMonitorPage />);

    const proxyButtons = screen.getAllByRole("button", { name: "Proxy" });
    await user.click(proxyButtons[0]!);
    expect(screen.getByTitle("App 24.exe")).toBeInTheDocument();
    expect(screen.queryByTitle("App 23.exe")).not.toBeInTheDocument();

    const directButtons = screen.getAllByRole("button", { name: "Non-proxy" });
    await user.click(directButtons[1]!);
    await waitFor(() => {
      expect(store.state.queryHistory).toHaveBeenCalledWith(
        expect.objectContaining({ networkPath: "direct" }),
      );
    });
    expect(screen.getByTitle("App 24.exe")).toBeInTheDocument();
  });

  it("includes unknown traffic only in the realtime All selection", async () => {
    const user = userEvent.setup();
    render(<NetworkMonitorPage />);

    expect(screen.getByTitle("App 23.exe")).toBeInTheDocument();
    const proxyButtons = screen.getAllByRole("button", { name: "Proxy" });
    await user.click(proxyButtons[0]!);
    expect(screen.queryByTitle("App 23.exe")).not.toBeInTheDocument();
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

  it("shows only the Windows support notice on unsupported systems", () => {
    store.state.capabilities.platformSupported = false;
    render(<NetworkMonitorPage />);

    expect(
      screen.getByText("Network monitoring is not supported on this system"),
    ).toBeInTheDocument();
    expect(screen.queryByRole("table")).not.toBeInTheDocument();
  });

  it("uses the same unsupported-only state when the browser runtime rejects initialization", () => {
    const mutableState = store.state as unknown as {
      capabilities: typeof store.state.capabilities | null;
      error: { code: "unsupportedPlatform"; message: string } | null;
    };
    const capabilities = mutableState.capabilities;
    mutableState.capabilities = null;
    mutableState.error = {
      code: "unsupportedPlatform",
      message: "Tauri is unavailable",
    };

    render(<NetworkMonitorPage />);

    expect(
      screen.getByText("Network monitoring is not supported on this system"),
    ).toBeInTheDocument();
    expect(screen.queryByRole("table")).not.toBeInTheDocument();
    mutableState.capabilities = capabilities;
    mutableState.error = null;
  });
});
