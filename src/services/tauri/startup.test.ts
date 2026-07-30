import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke, isTauri } = vi.hoisted(() => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
  isTauri,
}));

describe("completeStartup", () => {
  beforeEach(() => {
    vi.resetModules();
    invoke.mockReset();
    isTauri.mockReset();
  });

  it("is a no-op outside Tauri", async () => {
    isTauri.mockReturnValue(false);
    const { completeStartup } = await import("./startup");

    await expect(completeStartup()).resolves.toBeUndefined();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("shares one Tauri invocation across concurrent callers", async () => {
    isTauri.mockReturnValue(true);
    invoke.mockResolvedValue(undefined);
    const { completeStartup } = await import("./startup");

    await Promise.all([completeStartup(), completeStartup()]);

    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith("complete_startup");
  });

  it("allows a retry after a failed transition", async () => {
    isTauri.mockReturnValue(true);
    invoke
      .mockRejectedValueOnce(new Error("transition failed"))
      .mockResolvedValueOnce(undefined);
    const { completeStartup } = await import("./startup");

    await expect(completeStartup()).rejects.toThrow("transition failed");
    await expect(completeStartup()).resolves.toBeUndefined();

    expect(invoke).toHaveBeenCalledTimes(2);
  });
});
