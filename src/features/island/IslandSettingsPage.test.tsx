// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { Provider } from "react-redux";
import { HashRouter } from "react-router";
import { afterEach, expect, it, vi } from "vitest";
import App from "../../App";
import { mainWindowSession } from "../../mainWindowSession";
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

async function selectOption(label: string, option: string) {
  fireEvent.mouseDown(screen.getByRole("combobox", { name: label }), { button: 0 });
  const item = await screen.findByRole("option", { name: option });
  fireEvent.pointerDown(item, { pointerType: "mouse", button: 0 });
  fireEvent.click(item);
}

function desktop() {
  vi.spyOn(islandApi, "isDesktop").mockReturnValue(true);
  vi.spyOn(islandApi, "listen").mockResolvedValue(() => {});
  vi.spyOn(islandApi, "config").mockResolvedValue(config);
  vi.spyOn(islandApi, "status").mockResolvedValue({ state: "compact", sequence: 1, lastError: null });
  vi.spyOn(islandApi, "displays").mockResolvedValue([]);
  vi.spyOn(islandApi, "notifications").mockResolvedValue({ sequence: 1, permission: "notRequested" });
  vi.spyOn(islandApi, "media").mockResolvedValue({ title: "", artist: "", playing: false,
    positionMs: 0, durationMs: 0, canPrevious: false, canNext: false, canToggle: false, canSeek: false, battery: null, cover: null });
}

it("keeps animation preview cancellable without saving the draft", async () => {
  desktop();
  const validate = vi.spyOn(islandApi, "validatePreview").mockResolvedValue();
  const save = vi.spyOn(islandApi, "save").mockResolvedValue(config);
  open("#/settings/island");
  fireEvent.click(screen.getByRole("button", { name: "动画效果" }));
  const field = await screen.findByLabelText("展开时长（毫秒）");
  expect(screen.queryByLabelText("过渡曲线")).toBeNull();
  await selectOption("动画预设", "自定义 · Custom");
  await selectOption("过渡曲线", "渐慢");
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

it("switches a matching capsule border with the theme and keeps a custom border", async () => {
  vi.spyOn(islandApi, "isDesktop").mockReturnValue(false);
  open("#/settings/island?group=appearance");
  const border = screen.getByLabelText<HTMLInputElement>("边框色");
  const background = screen.getByLabelText<HTMLInputElement>("背景色");
  await selectOption("岛屿主题", "浅色");
  expect(background.value).toBe("#f5f5f4");
  expect(border.value).toBe(background.value);
  fireEvent.change(border, { target: { value: "#123456" } });
  await selectOption("岛屿主题", "跟随系统");
  expect(background.value).toBe(defaults.appearance.background);
  expect(border.value).toBe("#123456");
});

it("restores the settings group and unsaved values from the tray session", async () => {
  desktop();
  mainWindowSession.draft = { ...config, edge: { ...config.edge, mode: "peek", hideDelayMs: 650 } };
  open("#/settings/island?group=edge");
  expect((await screen.findByLabelText<HTMLInputElement>("隐藏延迟（毫秒）")).valueAsNumber).toBe(650);
  expect(screen.getByRole("button", { name: "边界隐藏" }).getAttribute("aria-current")).toBe("page");
  fireEvent.click(screen.getByRole("button", { name: "放弃草稿" }));
  expect(screen.queryByLabelText("隐藏延迟（毫秒）")).toBeNull();
  expect(mainWindowSession.draft).toBeNull();
});

it("clears a captured tray draft when its save completed while the main window was closing", async () => {
  desktop();
  mainWindowSession.draft = { ...config, notifications: { enabled: false } };
  vi.spyOn(islandApi, "config").mockResolvedValue({ ...mainWindowSession.draft, revision: 1 });
  open("#/settings/island?group=notifications");
  await screen.findByRole("checkbox", { name: "启用通知" });
  expect(screen.queryByRole("alert")).toBeNull();
  expect(screen.getByRole("button", { name: "保存设置" }).hasAttribute("disabled")).toBe(true);
  expect(mainWindowSession.draft).toBeNull();
});

it("keeps the last serializable tray draft while a number field is empty", async () => {
  desktop();
  open("#/settings/island?group=animation");
  const duration = await screen.findByLabelText("展开时长（毫秒）");
  fireEvent.change(duration, { target: { value: "450" } });
  fireEvent.change(duration, { target: { value: "" } });
  expect(mainWindowSession.draft?.animation.expandMs).toBe(450);
});

it("runs the browser draft preview locally", async () => {
  vi.spyOn(islandApi, "isDesktop").mockReturnValue(false);
  const validate = vi.spyOn(islandApi, "validatePreview");
  open("#/settings/island");
  fireEvent.click(screen.getByRole("button", { name: "动画效果" }));
  fireEvent.click(screen.getByRole("button", { name: "播放预览" }));
  expect(screen.getByText("暂无音乐")).toBeTruthy();
  expect(screen.getByText("00:00", { selector: "text" })).toBeTruthy();
  expect(screen.getByText("-00:00")).toBeTruthy();
  fireEvent.click(await screen.findByRole("button", { name: "取消预览" }));
  expect(validate).not.toHaveBeenCalled();
});

it("offers only edge anchors and lets the user move along the selected edge", async () => {
  vi.spyOn(islandApi, "isDesktop").mockReturnValue(false);
  open("#/settings/island?group=placement");
  fireEvent.change(screen.getByLabelText("左右位置（%）"), { target: { value: "51" } });
  fireEvent.mouseDown(screen.getByRole("combobox", { name: "停靠位置" }), { button: 0 });
  const anchors = await screen.findAllByRole("option");
  expect(anchors.map(anchor => anchor.textContent)).toEqual(["顶部", "左侧", "右侧"]);
  const right = screen.getByRole("option", { name: "右侧" });
  fireEvent.pointerDown(right, { pointerType: "mouse", button: 0 });
  fireEvent.click(right);
  expect(screen.queryByLabelText("左右位置（%）")).toBeNull();
  fireEvent.change(screen.getByLabelText("上下位置（%）"), { target: { value: "-28" } });
  expect(mainWindowSession.draft?.placement).toMatchObject({ anchor: "centerRight", offsetX: 0, offsetY: -28 });
});

it("previews current desktop media in the settings without saving", async () => {
  desktop();
  vi.spyOn(islandApi, "media").mockResolvedValue({
    title: "测试曲目", artist: "测试音乐人", playing: false, positionMs: 62500, durationMs: 180000, cover: null, battery: [75, false],
    canPrevious: false, canNext: true, canToggle: true, canSeek: true,
  });
  vi.spyOn(islandApi, "validatePreview").mockResolvedValue();
  open("#/settings/island?group=animation");
  await screen.findByText("75", { selector: "text" });
  fireEvent.click(screen.getByRole("button", { name: "播放预览" }));
  expect(await screen.findByText("测试曲目")).toBeTruthy();
  expect(screen.getByText("测试音乐人")).toBeTruthy();
  expect(screen.getByText("01:02")).toBeTruthy();
  expect(screen.getByText("-01:57")).toBeTruthy();
});

it("sets the upstream size in five-percent steps and restores its default", () => {
  vi.spyOn(islandApi, "isDesktop").mockReturnValue(false);
  open("#/settings/island?group=appearance");
  const size = screen.getByRole<HTMLInputElement>("slider", { name: "岛体大小（%）" });
  expect([size.min, size.max, size.step, size.value]).toEqual(["80", "140", "5", "100"]);
  fireEvent.change(size, { target: { value: "135" } });
  expect(mainWindowSession.draft?.appearance.islandSize).toBe(135);
  fireEvent.click(screen.getByRole("button", { name: /^恢复默认$/ }));
  expect(size.value).toBe("100");
});

it("updates the preview time and date at midnight without interaction", () => {
  vi.useFakeTimers();
  try {
    vi.setSystemTime(new Date(2026, 0, 1, 23, 59, 59, 900));
    vi.spyOn(islandApi, "isDesktop").mockReturnValue(false);
    open("#/settings/island?group=animation");
    fireEvent.click(screen.getByRole("button", { name: "播放预览" }));
    expect(screen.getByText("23:59", { selector: "time" })).toBeTruthy();
    act(() => vi.advanceTimersByTime(100));
    expect(screen.getByText("00:00", { selector: "time" })).toBeTruthy();
    expect(screen.getByText("1月2日 周五")).toBeTruthy();
  } finally {
    cleanup();
    vi.useRealTimers();
  }
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

it("opens the notification deep link with a single switch and no desktop permission prompt in the browser", () => {
  vi.spyOn(islandApi, "isDesktop").mockReturnValue(false);
  const access = vi.spyOn(islandApi, "requestNotificationAccess");
  open("#/settings/island?group=notifications");
  expect(screen.getByRole("heading", { name: "通知" })).toBeTruthy();
  expect(screen.getAllByRole("checkbox")).toHaveLength(1);
  const enabled = screen.getByRole("checkbox", { name: "启用通知" });
  fireEvent.click(enabled);
  expect(screen.getByText("有未保存的更改")).toBeTruthy();
  fireEvent.click(enabled);
  expect(screen.queryByText("有未保存的更改")).toBeNull();
  expect(screen.queryByRole("button", { name: "请求 Windows 通知访问" })).toBeNull();
  expect(access).not.toHaveBeenCalled();
});

it("keeps drafts across groups and retries a failed save as a single configuration update", async () => {
  desktop();
  vi.spyOn(islandApi, "notifications").mockResolvedValue({ sequence: 2, permission: "denied" });
  const save = vi.spyOn(islandApi, "save").mockRejectedValueOnce("保存失败，磁盘空间不足")
    .mockImplementationOnce(async draft => ({ ...draft, revision: 1 }));
  const access = vi.spyOn(islandApi, "requestNotificationAccess");
  const store = open("#/settings/island?group=notifications");
  await screen.findByRole("button", { name: "打开 Windows 通知隐私设置" });
  fireEvent.click(screen.getByRole("checkbox", { name: "启用通知" }));
  fireEvent.click(screen.getByRole("button", { name: "鼠标交互" }));
  fireEvent.click(screen.getByRole("checkbox", { name: "点击展开" }));
  expect(save).not.toHaveBeenCalled();
  expect(access).not.toHaveBeenCalled();
  const submit = screen.getByRole("button", { name: "保存设置" });
  fireEvent.click(submit);
  expect((await screen.findByRole("alert")).textContent).toContain("磁盘空间不足");
  expect(store.getState().island.config).toEqual(config);
  fireEvent.click(submit);
  await screen.findByText("设置已保存");
  expect(save).toHaveBeenCalledTimes(2);
  expect(save.mock.calls[0]).toEqual(save.mock.calls[1]);
  expect(store.getState().island.config).toEqual({
    ...config, revision: 1, interaction: { ...config.interaction, clickExpand: false }, notifications: { enabled: false },
  });
  expect(submit.hasAttribute("disabled")).toBe(true);
});

it("stages defaults at the saved revision and allows discarding them", async () => {
  desktop();
  const saved = { ...config, revision: 7, enabled: false };
  vi.spyOn(islandApi, "config").mockResolvedValue(saved);
  const save = vi.spyOn(islandApi, "save").mockImplementation(async value => ({ ...value, revision: 8 }));
  open("#/settings/island");
  const reset = await screen.findByRole("button", { name: "恢复默认设置" });
  fireEvent.click(reset);
  expect(save).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "放弃草稿" }));
  expect(screen.getByRole("checkbox", { name: "启用灵动岛" }).getAttribute("aria-checked")).toBe("false");
  fireEvent.click(reset);
  fireEvent.click(screen.getByRole("button", { name: "保存设置" }));
  await screen.findByText("设置已保存");
  expect(save).toHaveBeenCalledExactlyOnceWith({ ...config, revision: 7 });
  fireEvent.click(reset);
  expect(screen.queryByText("有未保存的更改")).toBeNull();
});
