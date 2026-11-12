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
  vi.spyOn(islandApi, "notifications").mockResolvedValue({ sequence: 1, permission: "notRequested", windows: [], reminders: [] });
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

it("runs the browser draft preview locally", async () => {
  vi.spyOn(islandApi, "isDesktop").mockReturnValue(false);
  const validate = vi.spyOn(islandApi, "validatePreview");
  open("#/settings/island");
  fireEvent.click(screen.getByRole("button", { name: "动画效果" }));
  fireEvent.click(screen.getByRole("button", { name: "播放预览" }));
  fireEvent.click(await screen.findByRole("button", { name: "取消预览" }));
  expect(validate).not.toHaveBeenCalled();
});

it("pins the native island from settings, ignoring older runtime snapshots", async () => {
  desktop();
  const pin = vi.spyOn(islandApi, "pin").mockResolvedValue();
  const store = open("#/settings/island");
  const button = await screen.findByRole("button", { name: "固定岛屿" });
  expect(store.getState().island.status?.state).toBe("compact");
  act(() => {
    store.dispatch(island.actions.statusReceived({ state: "expanded", sequence: 3, lastError: null }));
    store.dispatch(island.actions.statusReceived({ state: "compact", sequence: 2, lastError: null }));
  });
  fireEvent.click(button);
  await waitFor(() => expect(pin).toHaveBeenCalledWith(true));
  expect(store.getState().island.status?.state).toBe("expanded");
});

it("opens the notification deep link and previews a local sample without calling Windows", async () => {
  vi.spyOn(islandApi, "isDesktop").mockReturnValue(false);
  const access = vi.spyOn(islandApi, "requestNotificationAccess");
  const activate = vi.spyOn(islandApi, "activateNotificationWindow");
  open("#/settings/island?group=notifications");
  expect(screen.getByRole("heading", { name: "通知与消息" })).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "模拟一条提醒" }));
  fireEvent.click(screen.getByRole("button", { name: "展开通知示例" }));
  expect(screen.getByText("微信 2")).toBeTruthy();
  expect(screen.queryByText("敏感内容已隐藏")).toBeNull();
  expect(access).not.toHaveBeenCalled();
  expect(activate).not.toHaveBeenCalled();
});

it("uses a registered window id and keeps authorization separate from draft controls", async () => {
  desktop();
  vi.spyOn(islandApi, "notifications").mockResolvedValue({
    sequence: 2, permission: "denied", windows: [{ id: 7, source: "wechat", label: "微信 2" }], reminders: [],
  });
  const access = vi.spyOn(islandApi, "requestNotificationAccess");
  const activate = vi.spyOn(islandApi, "activateNotificationWindow").mockResolvedValue();
  open("#/settings/island?group=notifications");
  fireEvent.click(await screen.findByRole("checkbox", { name: "显示通知正文预览" }));
  expect(screen.getByLabelText("摘要保留时长（秒）")).toBeTruthy();
  expect(screen.getByRole("button", { name: "打开 Windows 通知隐私设置" })).toBeTruthy();
  expect(access).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "切换到微信 2" }));
  await waitFor(() => expect(activate).toHaveBeenCalledWith(7));
});
