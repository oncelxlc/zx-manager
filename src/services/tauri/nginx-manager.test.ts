import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke, isTauri } = vi.hoisted(() => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke, isTauri }));

import {
  checkNginxUpdates,
  controlNginxInstance,
  getNginxConfiguration,
  getNginxRuntimeDetails,
  inspectNginxDirectory,
  readNginxConfigGraph,
  readNginxConfigNode,
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

  it("reads configuration by opaque registered instance id", async () => {
    invoke.mockResolvedValue({ instanceId: "instance-1", sources: [] });

    await getNginxConfiguration("instance-1");

    expect(invoke).toHaveBeenCalledWith("get_nginx_configuration", {
      instanceId: "instance-1",
    });
  });

  it("reads graph nodes without accepting a path", async () => {
    invoke.mockResolvedValue({ node: { id: "node-1" } });

    await readNginxConfigGraph("instance-1");
    await readNginxConfigNode("instance-1", "node-1");

    expect(invoke).toHaveBeenNthCalledWith(1, "read_nginx_config_graph", {
      instanceId: "instance-1",
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "read_nginx_config_node", {
      instanceId: "instance-1",
      nodeId: "node-1",
    });
    expect(JSON.stringify(invoke.mock.calls)).not.toContain("path");
  });

  it("reads runtime details by opaque registered instance id", async () => {
    invoke.mockResolvedValue({ instanceId: "instance-1", processes: [] });

    await getNginxRuntimeDetails("instance-1");

    expect(invoke).toHaveBeenCalledWith("get_nginx_runtime_details", {
      instanceId: "instance-1",
    });
  });

  it("sends only an enumerated control action and instance id", async () => {
    invoke.mockResolvedValue({ success: true });

    await controlNginxInstance("instance-1", "reload");

    expect(invoke).toHaveBeenCalledWith("control_nginx_instance", {
      input: { instanceId: "instance-1", action: "reload" },
    });
  });
});
