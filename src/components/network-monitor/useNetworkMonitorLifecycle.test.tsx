import { render, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

const storage = vi.hoisted(() => ({
  getPreferences: vi.fn(async () => ({})),
}));

vi.mock("src/services/storage/preferences-storage", () => storage);

import { useNetworkMonitorLifecycle } from "./useNetworkMonitorLifecycle";

function LifecycleProbe({
  initialize,
  startRealtime,
  stopRealtime,
}: {
  initialize: () => Promise<void>;
  startRealtime: () => Promise<void>;
  stopRealtime: () => Promise<void>;
}) {
  useNetworkMonitorLifecycle({initialize, startRealtime, stopRealtime});
  return null;
}

describe("useNetworkMonitorLifecycle", () => {
  it("subscribes only after the initial manager snapshot has settled", async () => {
    let resolveInitialize: (() => void) | undefined;
    const initialize = vi.fn(() => new Promise<void>((resolve) => {
      resolveInitialize = resolve;
    }));
    const startRealtime = vi.fn(async () => undefined);
    const stopRealtime = vi.fn(async () => undefined);

    render(
      <LifecycleProbe
        initialize={initialize}
        startRealtime={startRealtime}
        stopRealtime={stopRealtime}
      />,
    );

    expect(initialize).toHaveBeenCalledOnce();
    expect(startRealtime).not.toHaveBeenCalled();

    resolveInitialize?.();
    await waitFor(() => expect(startRealtime).toHaveBeenCalledOnce());
  });
});
