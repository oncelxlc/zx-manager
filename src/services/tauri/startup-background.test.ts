import { beforeEach, describe, expect, it, vi } from "vitest";

const services = vi.hoisted(() => ({
  restoreNginxOnStartup: vi.fn(),
}));

vi.mock("src/services/tauri/nginx-startup", () => ({
  restoreNginxOnStartup: services.restoreNginxOnStartup,
}));

import {
  configureStartupBackgroundTasks,
  resetStartupBackgroundTasks,
  restoreStartupBackgroundTasks,
} from "./startup-background";

describe("startup background tasks", () => {
  beforeEach(() => {
    resetStartupBackgroundTasks();
    vi.clearAllMocks();
    services.restoreNginxOnStartup.mockResolvedValue(undefined);
  });

  it("restores Nginx with one shared flight", async () => {
    const preferences = { locale: "en-US" as const };
    configureStartupBackgroundTasks(preferences);

    await Promise.all([
      restoreStartupBackgroundTasks(),
      restoreStartupBackgroundTasks(),
    ]);

    expect(services.restoreNginxOnStartup).toHaveBeenCalledOnce();
    expect(services.restoreNginxOnStartup).toHaveBeenCalledWith(preferences);
  });

  it("does not reject the shell when one background task fails", async () => {
    services.restoreNginxOnStartup.mockRejectedValue(new Error("offline"));

    await expect(restoreStartupBackgroundTasks()).resolves.toBeUndefined();
  });
});
