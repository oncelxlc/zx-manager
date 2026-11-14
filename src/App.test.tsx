// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { Provider } from "react-redux";
import { HashRouter } from "react-router";
import { afterEach, expect, it, vi } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import App from "./App";
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
