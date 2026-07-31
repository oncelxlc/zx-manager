import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke, isTauri } = vi.hoisted(() => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke, isTauri }));

import {
  checkNginxUpdates,
  inspectNginxDirectory,
  registerNginxInstance,
  selectNginxDirectory,
  toNginxCommandError,
} from "./nginx-manager";

describe("nginx manager Tauri service", () => {
  beforeEach(() => {
    invoke.mockReset();
    isTauri.mockReturnValue(true);
  });

  it("passes only opaque tokens between directory selection and registration", async () => {
    invoke
      .mockResolvedValueOnce({ selectionId: "selection-1" })
      .mockResolvedValueOnce({ inspectionId: "inspection-1" })
      .mockResolvedValueOnce({ id: "instance-1" });

    const selection = await selectNginxDirectory("inspectInstance");
    await inspectNginxDirectory(selection!.selectionId);
    await registerNginxInstance({
      inspectionId: "inspection-1",
      name: "Local Nginx",
      authorizationLevel: "readOnly",
    });

    expect(invoke).toHaveBeenNthCalledWith(1, "select_nginx_directory", {
      purpose: "inspectInstance",
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "inspect_nginx_directory", {
      selectionId: "selection-1",
    });
    expect(invoke).toHaveBeenNthCalledWith(3, "register_nginx_instance", {
      input: {
        inspectionId: "inspection-1",
        name: "Local Nginx",
        authorizationLevel: "readOnly",
      },
    });
    expect(JSON.stringify(invoke.mock.calls)).not.toContain("C:\\\\nginx");
  });

  it("normalizes stable command errors", () => {
    expect(toNginxCommandError({ code: "NGINX_BINARY_NOT_FOUND", message: "x" }))
      .toEqual({ code: "NGINX_BINARY_NOT_FOUND", message: "x" });
  });

  it("returns a stable error without IPC in a browser", async () => {
    isTauri.mockReturnValue(false);

    await expect(selectNginxDirectory("inspectInstance")).rejects.toMatchObject({
      code: "NGINX_DESKTOP_REQUIRED",
    });
    expect(invoke).not.toHaveBeenCalled();
  });

  it("checks releases with a bounded policy input", async () => {
    invoke.mockResolvedValue({ latestRelease: null });

    await checkNginxUpdates("stable", true, 24);

    expect(invoke).toHaveBeenCalledWith("check_nginx_updates", {
      input: { channel: "stable", force: true, maxAgeHours: 24 },
    });
  });
});
