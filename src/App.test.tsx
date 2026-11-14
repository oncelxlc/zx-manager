// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { Provider } from "react-redux";
import { HashRouter } from "react-router";
import { afterEach, expect, it, vi } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import entryHtml from "../index.html?raw";
import App from "./App";
import { defaults } from "./app/navigation";
import { islandApi } from "./features/island/islandApi";
import { mainWindowSession, restoreMainWindow } from "./mainWindowSession";
import { createAppStore } from "./store";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  window.localStorage?.clear();
});

function renderAt(hash: string) {
  window.history.replaceState(null, "", hash);
  const store = createAppStore();
  render(
    <Provider store={store}>
      <HashRouter>
        <App />
      </HashRouter>
    </Provider>,
  );
  return store;
}

it("shows the overview and lists its main dependencies", () => {
  renderAt("#/");

  expect(screen.getByRole("heading", { level: 1, name: "工作台总览" })).toBeTruthy();
  const section = within(screen.getByRole("region", { name: "主要依赖" }));
  const dependencies = within(section.getByRole("list")).getAllByRole("listitem");
  expect(dependencies.map((item) => item.textContent).sort()).toEqual([
    "React", "React Router", "Redux Toolkit", "StyleX", "Tauri", "TypeScript", "Vite", "lucide-react", "shadcn/ui",
  ]);
});

it("toggles the dependency list through Redux", () => {
  const store = renderAt("#/");
  const toggle = screen.getByRole("button", { name: "主要依赖" });
  const section = within(screen.getByRole("region", { name: "主要依赖" }));

  expect(toggle.getAttribute("aria-expanded")).toBe("true");
  fireEvent.click(toggle);
  expect(store.getState().welcome.dependenciesExpanded).toBe(false);
  expect(toggle.getAttribute("aria-expanded")).toBe("false");
  expect(section.queryByRole("list")).toBeNull();

  fireEvent.click(toggle);
  expect(store.getState().welcome.dependenciesExpanded).toBe(true);
  expect(toggle.getAttribute("aria-expanded")).toBe("true");
  expect(section.getByRole("list")).toBeTruthy();
});

it("keeps navigation groups static and reaches recent projects from the collapsed sidebar", async () => {
  vi.stubGlobal("innerWidth", 1280);
  renderAt("#/");
  const nav = within(screen.getByRole("navigation", { name: "主导航" }));
  expect(nav.getByText("工作台")).toBeTruthy();
  expect(nav.queryByRole("button", { name: "工作台" })).toBeNull();

  fireEvent.click(screen.getByRole("button", { name: "收起侧栏", expanded: true }));
  expect(nav.queryByText("工作台")).toBeNull();
  fireEvent.click(nav.getByRole("link", { name: "最近使用" }));

  await waitFor(() => expect(window.location.hash).toBe("#/?section=recent"));
  expect(nav.getByRole("link", { name: "最近使用", current: "page" })).toBeTruthy();
  const recent = screen.getByRole("region", { name: "最近项目" });
  expect(document.activeElement).toBe(recent.closest("[tabindex]"));

  fireEvent.click(screen.getByRole("button", { name: "展开侧栏", expanded: false }));
  expect(nav.getByText("工作台")).toBeTruthy();
});

it("redirects an unknown hash route to the overview", async () => {
  renderAt("#/missing");

  await waitFor(() => expect(window.location.hash).toBe("#/"));
  expect(screen.getByRole("heading", { level: 1, name: "工作台总览" })).toBeTruthy();
});

function mockLocalStorage() {
  const storage = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => storage.get(key) ?? null,
    setItem: (key: string, value: string) => storage.set(key, value),
  });
}

it.each([
  { saved: '{"theme":"dark","groups":[]}', systemDark: false, theme: "dark", resolved: "dark" },
  { saved: '{"theme":"light","sidebar":false}', systemDark: true, theme: "light", resolved: "light" },
  { saved: '{"theme":"system"}', systemDark: true, theme: "system", resolved: "dark" },
  { saved: '{"theme":"invalid"}', systemDark: false, theme: "system", resolved: "light" },
  { saved: "malformed", systemDark: true, theme: "system", resolved: "dark" },
])("keeps the first-paint and mounted themes in sync for $saved", ({ saved, systemDark, theme, resolved }) => {
  mockLocalStorage();
  localStorage.setItem("zx-once.workbench.preferences.v1", saved);
  vi.stubGlobal("matchMedia", (media: string) => Object.assign(new EventTarget(), {
    matches: media === "(prefers-color-scheme: dark)" && systemDark,
  }));
  const entry = new DOMParser().parseFromString(entryHtml, "text/html");
  const bootstrap = entry.head.querySelector("script + link[rel=stylesheet]")?.previousElementSibling;
  if (!bootstrap?.textContent) throw new Error("Missing theme bootstrap before the stylesheet");
  document.documentElement.removeAttribute("data-theme");
  window.eval(bootstrap.textContent);
  expect(document.documentElement.dataset.theme).toBe(resolved);
  expect(document.documentElement.classList.contains("dark")).toBe(resolved === "dark");

  const store = renderAt("#/");
  expect(store.getState().preferences).toEqual({ ...defaults, theme });
  expect(document.documentElement.dataset.theme).toBe(resolved);
});

it("follows system changes, keeps explicit themes across reloads, and preserves workbench data", async () => {
  mockLocalStorage();
  const query = Object.assign(new EventTarget(), { matches: true });
  vi.stubGlobal("matchMedia", (media: string) => media === "(prefers-color-scheme: dark)" ? query : new EventTarget());
  const store = renderAt("#/settings/appearance");
  const { preferences, ...business } = store.getState();
  const root = document.documentElement;
  const expectTheme = (theme: "light" | "dark") => {
    expect(root.dataset.theme).toBe(theme);
    expect(root.classList.contains("dark")).toBe(theme === "dark");
  };
  function changeSystem(dark: boolean) {
    query.matches = dark;
    act(() => { query.dispatchEvent(new Event("change")); });
  }

  expect(preferences.theme).toBe("system");
  expectTheme("dark");
  changeSystem(false);
  expectTheme("light");
  const dark = await screen.findByRole("button", { name: "深色" });
  fireEvent.click(dark);
  expectTheme("dark");
  changeSystem(false);
  expectTheme("dark");
  fireEvent.click(screen.getByRole("button", { name: "浅色" }));
  changeSystem(true);
  expectTheme("light");
  cleanup();
  const refreshed = renderAt("#/settings/appearance");
  expect(refreshed.getState().preferences).toEqual({ ...preferences, theme: "light" });
  expectTheme("light");
  fireEvent.click(await screen.findByRole("button", { name: "跟随系统" }));
  expectTheme("dark");
  changeSystem(false);
  expectTheme("light");
  expect(store.getState()).toEqual({ ...business, preferences: { ...preferences, theme: "light" } });
  expect(refreshed.getState()).toEqual({ ...business, preferences });
});

it("restores the settings group and welcome disclosure before mounting a rebuilt window", async () => {
  const route = "#/settings/island?group=animation";
  window.history.replaceState(null, "", "#/");
  vi.spyOn(islandApi, "isDesktop").mockReturnValue(true);
  mockIPC(command => {
    expect(command).toBe("main_window_restore");
    return { route, dependenciesExpanded: false, draft: null };
  });
  try {
    await restoreMainWindow();
    expect(window.location.hash).toBe(route);
    expect(createAppStore(mainWindowSession.dependenciesExpanded).getState().welcome.dependenciesExpanded).toBe(false);
    expect(window.__zxMainSession).toBe(mainWindowSession);
  } finally {
    clearMocks();
    vi.restoreAllMocks();
    mainWindowSession.dependenciesExpanded = true;
    delete window.__zxMainSession;
  }
});
