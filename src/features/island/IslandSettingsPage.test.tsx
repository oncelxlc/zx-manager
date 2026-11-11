// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { Provider } from "react-redux";
import { HashRouter } from "react-router";
import { afterEach, expect, it, vi } from "vitest";
import App from "../../App";
import { createAppStore } from "../../store";
import defaults from "./defaults.json";
import { islandApi } from "./islandApi";
import { island } from "./islandSlice";
import type { IslandConfig } from "./types";

const config = defaults as IslandConfig;
afterEach(() => { cleanup(); vi.restoreAllMocks(); });

function open(hash: string) {
  window.history.replaceState(null, "", hash);
  const store = createAppStore();
  render(<Provider store={store}><HashRouter><App /></HashRouter></Provider>);
  return store;
}

function desktop() {
  vi.spyOn(islandApi, "isDesktop").mockReturnValue(true);
  vi.spyOn(islandApi, "listen").mockResolvedValue(() => {});
  vi.spyOn(islandApi, "config").mockResolvedValue(config);
  vi.spyOn(islandApi, "status").mockResolvedValue({ state: "compact", sequence: 1, lastError: null });
  vi.spyOn(islandApi, "displays").mockResolvedValue([]);
}

it("keeps animation preview cancellable without saving the draft", async () => {
  desktop();
  const validate = vi.spyOn(islandApi, "validatePreview").mockResolvedValue();
  const save = vi.spyOn(islandApi, "save").mockResolvedValue(config);
  open("#/settings/island");
  fireEvent.click(screen.getByRole("button", { name: "动画效果" }));
  const field = await screen.findByLabelText("展开时长（毫秒）");
  expect(screen.queryByLabelText("过渡曲线")).toBeNull();
  fireEvent.change(screen.getByLabelText("动画预设"), { target: { value: "custom" } });
  fireEvent.change(screen.getByLabelText("过渡曲线"), { target: { value: "easeOut" } });
  fireEvent.change(field, { target: { value: "340" } });
  fireEvent.click(screen.getByRole("button", { name: "播放预览" }));
  await waitFor(() => expect(validate).toHaveBeenCalledWith({
    ...config, animation: { ...config.animation, preset: "custom", customCurve: "easeOut", expandMs: 340 },
  }));
  fireEvent.click(screen.getByRole("button", { name: "取消预览" }));
  expect(screen.getByRole("button", { name: "播放预览" })).toBeTruthy();
  expect(save).not.toHaveBeenCalled();
});

it("keeps an edited revision intact when another window saves", async () => {
  desktop();
  const store = open("#/settings/island");
  const enabled = await screen.findByRole("checkbox", { name: "启用灵动岛" });
  fireEvent.click(enabled);
  act(() => {
    store.dispatch(island.actions.configReceived({ ...config, revision: 2 }));
    store.dispatch(island.actions.configReceived({ ...config, revision: 1 }));
  });
  expect(screen.getByRole("alert").textContent).toContain("配置已在其他窗口更新");
  expect(screen.getByRole("button", { name: "保存设置" }).hasAttribute("disabled")).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "放弃草稿" }));
  expect(store.getState().island.config?.revision).toBe(2);
  expect(screen.queryByText("有未保存的更改")).toBeNull();
});

it("signals readiness and routes clicks through Rust, ignoring older runtime snapshots", async () => {
  desktop();
  const ready = vi.spyOn(islandApi, "ready").mockResolvedValue();
  const click = vi.spyOn(islandApi, "interact").mockResolvedValue();
  const pin = vi.spyOn(islandApi, "pin").mockResolvedValue();
  const store = open("#/island");
  const openButton = await screen.findByRole("button", { name: "展开灵动岛" });
  await waitFor(() => expect(ready).toHaveBeenCalledTimes(1));
  fireEvent.click(openButton);
  expect(click).toHaveBeenCalledWith("click");
  expect(store.getState().island.status?.state).toBe("compact");
  act(() => {
    store.dispatch(island.actions.statusReceived({ state: "expanded", sequence: 3, lastError: null }));
    store.dispatch(island.actions.statusReceived({ state: "compact", sequence: 2, lastError: null }));
  });
  fireEvent.click(screen.getByRole("button", { name: "固定岛屿" }));
  expect(pin).toHaveBeenCalledWith(true);
  expect(store.getState().island.status?.state).toBe("expanded");
});
