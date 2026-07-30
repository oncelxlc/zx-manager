import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

const storage = vi.hoisted(() => ({
  getPreferences: vi.fn(),
  setNetworkMonitorStartOnLaunch: vi.fn(),
}));

vi.mock("src/services/storage/preferences-storage", () => storage);

import { NetworkMonitorLaunchSwitcher } from "./NetworkMonitorLaunchSwitcher";

describe("NetworkMonitorLaunchSwitcher", () => {
  beforeEach(() => {
    storage.getPreferences.mockResolvedValue({});
    storage.setNetworkMonitorStartOnLaunch.mockReset();
  });

  it("defaults to starting the monitor when the application launches", async () => {
    render(<NetworkMonitorLaunchSwitcher />);

    const control = await screen.findByRole("switch", {
      name: "Start monitoring when the app launches",
    });
    expect(control).toBeChecked();
  });

  it("persists changes without changing the current session", async () => {
    const user = userEvent.setup();
    storage.getPreferences.mockResolvedValue({
      networkMonitorStartOnLaunch: true,
    });
    render(<NetworkMonitorLaunchSwitcher />);

    const control = await screen.findByRole("switch", {
      name: "Start monitoring when the app launches",
    });
    await user.click(control);

    await waitFor(() => {
      expect(storage.setNetworkMonitorStartOnLaunch).toHaveBeenCalledWith(false);
    });
    expect(control).not.toBeChecked();
  });
});
