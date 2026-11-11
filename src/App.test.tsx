// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { Provider } from "react-redux";
import { HashRouter } from "react-router";
import { afterEach, expect, it } from "vitest";
import App from "./App";
import { createAppStore } from "./store";

afterEach(cleanup);

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

it("shows the welcome page and lists its main dependencies", () => {
  renderAt("#/");

  expect(screen.getByRole("heading", { level: 1 }).textContent).toMatch(/欢迎使用\s*ZxManager/);
  const dependencies = within(screen.getByRole("list")).getAllByRole("listitem");
  expect(dependencies.map((item) => item.textContent).sort()).toEqual([
    "React", "React Router", "Redux Toolkit", "StyleX", "Tauri", "TypeScript", "Vite", "lucide-react", "shadcn/ui",
  ]);
});

it("toggles the dependency list through Redux", () => {
  const store = renderAt("#/");
  const toggle = screen.getByRole("button", { name: "主要依赖" });

  expect(toggle.getAttribute("aria-expanded")).toBe("true");
  fireEvent.click(toggle);
  expect(store.getState().welcome.dependenciesExpanded).toBe(false);
  expect(toggle.getAttribute("aria-expanded")).toBe("false");
  expect(screen.queryByRole("list")).toBeNull();

  fireEvent.click(toggle);
  expect(store.getState().welcome.dependenciesExpanded).toBe(true);
  expect(toggle.getAttribute("aria-expanded")).toBe("true");
  expect(screen.getByRole("list")).toBeTruthy();
});

it("redirects an unknown hash route to the welcome page", async () => {
  renderAt("#/missing");

  await waitFor(() => expect(window.location.hash).toBe("#/"));
  expect(screen.getByRole("heading", { level: 1 }).textContent).toMatch(/ZxManager/);
});
