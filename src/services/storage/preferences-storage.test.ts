import { beforeEach, describe, expect, it, vi } from "vitest";

const { loadStore } = vi.hoisted(() => ({
  loadStore: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-store", () => ({
  load: loadStore,
}));

const nginxPreferences = {
  releaseChannel: "stable",
  updateCheckIntervalHours: 24,
  backupRetentionCount: 5,
  logFollow: true,
  logBufferLines: 20_000,
  configCustomGroups: [],
  configNodeGroups: {},
};

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

  it("migrates pre-v8 Store preferences without retaining retired fields", async () => {
    const store = createStore({
      preferences: {
        version: 6,
        locale: "en-US",
        theme: "light",
        nginx: { releaseChannel: "mainline" },
        retiredSetting: true,
      },
    });
    loadStore.mockResolvedValue(store);
    const { getPreferences } = await loadPreferencesStorage();

    await expect(getPreferences()).resolves.toEqual({
      locale: "en-US",
      theme: "light",
      nginx: { ...nginxPreferences, releaseChannel: "mainline" },
    });
    expect(store.set).toHaveBeenCalledWith("preferences", {
      version: 8,
      locale: "en-US",
      theme: "light",
      nginx: { ...nginxPreferences, releaseChannel: "mainline" },
    });
  });

  it("ignores invalid Store values", async () => {
    const store = createStore({
      preferences: { version: 99, locale: "de-DE", theme: "neon" },
    });
    loadStore.mockResolvedValue(store);
    const { getPreferences } = await loadPreferencesStorage();

    await expect(getPreferences()).resolves.toEqual({
      locale: undefined,
      theme: undefined,
      nginx: nginxPreferences,
    });
  });

  it("migrates legacy localStorage preferences after saving the Store", async () => {
    const store = createStore();
    loadStore.mockResolvedValue(store);
    window.localStorage.setItem(
      "local-console.preferences",
      JSON.stringify({ version: 6, locale: "en-US", theme: "system" }),
    );
    const { getPreferences } = await loadPreferencesStorage();

    await expect(getPreferences()).resolves.toEqual({
      locale: "en-US",
      theme: "system",
      nginx: nginxPreferences,
    });
    expect(store.set).toHaveBeenCalledWith("preferences", {
      version: 8,
      locale: "en-US",
      theme: "system",
      nginx: nginxPreferences,
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
      version: 8,
      locale: "en-US",
      theme: "light",
      nginx: nginxPreferences,
    });
  });

  it("persists Nginx preferences through the Store", async () => {
    const store = createStore({
      preferences: { version: 8, locale: "zh-CN", theme: "dark" },
    });
    loadStore.mockResolvedValue(store);
    const { setNginxPreferences } = await loadPreferencesStorage();

    await setNginxPreferences({ updateCheckIntervalHours: 48 });

    expect(store.set).toHaveBeenLastCalledWith("preferences", {
      version: 8,
      locale: "zh-CN",
      theme: "dark",
      nginx: { ...nginxPreferences, updateCheckIntervalHours: 48 },
    });
  });

  it("preserves bounded configuration groups in schema v8", async () => {
    const store = createStore({
      preferences: {
        version: 8,
        nginx: {
          configCustomGroups: [{ id: "traffic", name: " Traffic " }],
          configNodeGroups: { ["a".repeat(64)]: "traffic", invalid: "traffic" },
        },
      },
    });
    loadStore.mockResolvedValue(store);
    const { getPreferences } = await loadPreferencesStorage();

    await expect(getPreferences()).resolves.toMatchObject({
      nginx: {
        configCustomGroups: [{ id: "traffic", name: "Traffic" }],
        configNodeGroups: { ["a".repeat(64)]: "traffic" },
      },
    });
  });
});
