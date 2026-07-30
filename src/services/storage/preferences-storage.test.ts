import { beforeEach, describe, expect, it, vi } from "vitest";

const { loadStore } = vi.hoisted(() => ({
  loadStore: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-store", () => ({
  load: loadStore,
}));

function createStore(initialValues: Record<string, unknown> = {}) {
  const values = new Map(Object.entries(initialValues));
  return {
    get: vi.fn(async (key: string) => values.get(key)),
    save: vi.fn(async () => undefined),
    set: vi.fn(async (key: string, value: unknown) => {
      values.set(key, value);
    }),
  };
}

async function loadPreferencesStorage() {
  await vi.resetModules();
  return import("./preferences-storage");
}

describe("preferences storage", () => {
  beforeEach(() => {
    window.localStorage.clear();
    loadStore.mockReset();
  });

  it("loads validated preferences from the Tauri Store", async () => {
    const store = createStore({
      preferences: { version: 1, locale: "en-US", theme: "light" },
    });
    loadStore.mockResolvedValue(store);
    const { getPreferences } = await loadPreferencesStorage();

    await expect(getPreferences()).resolves.toEqual({
      locale: "en-US",
      theme: "light",
      networkMonitorSampleIntervalSeconds: 5,
    });
    expect(loadStore).toHaveBeenCalledWith("preferences.json", { autoSave: false });
  });

  it("ignores invalid Store values", async () => {
    const store = createStore({
      preferences: { version: 1, locale: "de-DE", theme: "neon" },
    });
    loadStore.mockResolvedValue(store);
    const { getPreferences } = await loadPreferencesStorage();

    await expect(getPreferences()).resolves.toEqual({
      locale: undefined,
      theme: undefined,
      networkMonitorSampleIntervalSeconds: 5,
    });
  });

  it("migrates legacy localStorage preferences after saving the Store", async () => {
    const store = createStore();
    loadStore.mockResolvedValue(store);
    window.localStorage.setItem(
      "local-console.preferences",
      JSON.stringify({ version: 1, locale: "en-US", theme: "system" }),
    );
    const { getPreferences } = await loadPreferencesStorage();

    await expect(getPreferences()).resolves.toEqual({
      locale: "en-US",
      theme: "system",
      networkMonitorSampleIntervalSeconds: 5,
    });
    expect(store.set).toHaveBeenCalledWith("preferences", {
      version: 4,
      locale: "en-US",
      theme: "system",
      networkMonitorConfigured: false,
      networkMonitorSampleIntervalSeconds: 5,
    });
    expect(store.save).toHaveBeenCalledOnce();
    expect(window.localStorage.getItem("local-console.preferences")).toBeNull();
  });

  it("falls back to localStorage when the Tauri Store is unavailable", async () => {
    loadStore.mockRejectedValue(new Error("Tauri is unavailable"));
    window.localStorage.setItem("vite-ui-theme", "light");
    const { getPreferences, setLocale } = await loadPreferencesStorage();

    await expect(getPreferences()).resolves.toEqual({ theme: "light" });
    await setLocale("en-US");
    expect(JSON.parse(window.localStorage.getItem("local-console.preferences") ?? "{}")).toEqual({
      version: 4,
      locale: "en-US",
      theme: "light",
      networkMonitorConfigured: false,
      networkMonitorSampleIntervalSeconds: 5,
    });
  });

  it("saves updates through the Store", async () => {
    const store = createStore({
      preferences: { version: 1, locale: "zh-CN", theme: "dark" },
    });
    loadStore.mockResolvedValue(store);
    const { setTheme } = await loadPreferencesStorage();

    await setTheme("system");

    expect(store.set).toHaveBeenCalledWith("preferences", {
      version: 4,
      locale: "zh-CN",
      theme: "system",
      networkMonitorConfigured: false,
      networkMonitorSampleIntervalSeconds: 5,
    });
    expect(store.save).toHaveBeenCalledOnce();
  });

  it("ignores the retired start-on-launch preference during migration", async () => {
    const store = createStore({
      preferences: {
        version: 2,
        locale: "zh-CN",
        theme: "dark",
        networkMonitorConfigured: true,
        networkMonitorStartOnLaunch: false,
      },
    });
    loadStore.mockResolvedValue(store);
    const {
      getPreferences,
      setNetworkMonitorSampleInterval,
    } = await loadPreferencesStorage();

    await expect(getPreferences()).resolves.toEqual({
      locale: "zh-CN",
      theme: "dark",
      networkMonitorConfigured: true,
      networkMonitorSampleIntervalSeconds: 5,
    });
    await setNetworkMonitorSampleInterval(10);
    expect(store.set).toHaveBeenLastCalledWith("preferences", {
      version: 4,
      locale: "zh-CN",
      theme: "dark",
      networkMonitorConfigured: true,
      networkMonitorSampleIntervalSeconds: 10,
    });
  });

  it("validates and persists the network monitor sample interval", async () => {
    const store = createStore({
      preferences: {
        version: 3,
        locale: "zh-CN",
        theme: "dark",
        networkMonitorConfigured: true,
        networkMonitorSampleIntervalSeconds: 2,
      },
    });
    loadStore.mockResolvedValue(store);
    const {
      getPreferences,
      setNetworkMonitorSampleInterval,
    } = await loadPreferencesStorage();

    await expect(getPreferences()).resolves.toMatchObject({
      networkMonitorSampleIntervalSeconds: 5,
    });
    await setNetworkMonitorSampleInterval(10);
    expect(store.set).toHaveBeenLastCalledWith("preferences", {
      version: 4,
      locale: "zh-CN",
      theme: "dark",
      networkMonitorConfigured: true,
      networkMonitorSampleIntervalSeconds: 10,
    });
  });
});
