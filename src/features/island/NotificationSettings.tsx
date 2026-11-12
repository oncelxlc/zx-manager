import * as stylex from "@stylexjs/stylex";
import { islandApi, type NotificationPermission, type NotificationStatus } from "./islandApi";
import { NotificationExample } from "./NotificationExample";
import { Amount, Check } from "./SettingsFields";
import { ui } from "./settingsStyles";
import type { IslandConfig } from "./types";

const permissionLabels: Record<NotificationPermission, string> = {
  disabled: "监听已关闭",
  unavailable: "当前安装方式或系统不支持通知访问，可继续使用窗口入口",
  notRequested: "尚未授权",
  denied: "通知访问已被拒绝，可在 Windows 设置中更改",
  allowed: "已获准读取系统通知",
  error: "监听暂时不可用，可重新读取状态",
};

interface Props {
  config: IslandConfig;
  saved: IslandConfig | null;
  desktop: boolean;
  status: NotificationStatus | null;
  update: (patch: Partial<IslandConfig["notifications"]>) => void;
  perform: (action: () => Promise<unknown>, message?: string) => Promise<void>;
  refresh: () => Promise<void>;
}

export function NotificationSettings({ config, saved, desktop, status, update, perform, refresh }: Props) {
  const options = config.notifications;
  const listening = saved?.enabled && saved.notifications.enabled && saved.notifications.windowsNotificationsEnabled
    && (saved.notifications.wechatEnabled || saved.notifications.qqEnabled);
  return <>
    <Check label="启用通知与消息" value={options.enabled} change={enabled => update({ enabled })} />
    {options.enabled && <>
      <h3>消息来源</h3>
      <Check label="微信" value={options.wechatEnabled} change={wechatEnabled => update({ wechatEnabled })} />
      <Check label="QQ" value={options.qqEnabled} change={qqEnabled => update({ qqEnabled })} />
      <Check label="从 Windows 通知中心接收提醒" value={options.windowsNotificationsEnabled}
        change={windowsNotificationsEnabled => update({ windowsNotificationsEnabled })} />
      <p {...stylex.props(ui.message)}>红点表示该应用有系统提醒。系统未提供聊天未读数，也未指明通知来自哪个窗口。</p>
      {options.windowsNotificationsEnabled && <>
        <Check label="显示通知正文预览" value={options.showPreview} change={showPreview => update({ showPreview })} />
        <p {...stylex.props(ui.message)}>{options.showPreview
          ? "摘要只在内存中短暂显示，验证码会在显示前遮罩。"
          : "正文预览已关闭，灵动岛只使用通知来源等元数据。"}</p>
        {options.showPreview && <Amount label="摘要保留时长（秒）" value={options.previewTimeoutSeconds}
          min={1} max={60} change={previewTimeoutSeconds => update({ previewTimeoutSeconds })} />}
        <Check label="新提醒到达时轻微强调图标" value={options.animateNewMessage}
          change={animateNewMessage => update({ animateNewMessage })} />
      </>}
      <Check label="展开时显示窗口列表" value={options.accountListEnabled}
        change={accountListEnabled => update({ accountListEnabled })} />
      <NotificationExample config={config} />
    </>}
    {desktop && <section aria-label="桌面连接状态">
      <h3>桌面连接状态</h3>
      <p role="status" {...stylex.props(ui.message)}>{status ? permissionLabels[status.permission] : "正在读取通知访问状态…"}</p>
      <div {...stylex.props(ui.actions)}>
        {listening && (status?.permission === "notRequested" || status?.permission === "error") &&
          <button type="button" {...stylex.props(ui.button)} onClick={() => void perform(async () => {
            await islandApi.requestNotificationAccess();
            await refresh();
          })}>请求 Windows 通知访问</button>}
        {listening && status?.permission === "denied" && <button type="button" {...stylex.props(ui.button)}
          onClick={() => void perform(islandApi.openNotificationSettings)}>打开 Windows 通知隐私设置</button>}
        <button type="button" {...stylex.props(ui.button)} onClick={() => void perform(refresh)}>刷新连接状态</button>
      </div>
      {saved?.enabled && saved.notifications.enabled && status && <>
        <h3>可切换的窗口</h3>
        {status.windows.length ? <div {...stylex.props(ui.actions)}>{status.windows.map(({ id, label }) =>
          <button key={id} type="button" {...stylex.props(ui.button)}
            onClick={() => void perform(() => islandApi.activateNotificationWindow(id))}>切换到{label}</button>)}</div>
          : <p {...stylex.props(ui.message)}>尚未发现已启用来源的可切换窗口。</p>}
        {!!status.reminders.length && <p {...stylex.props(ui.message)}>当前有系统提醒的应用 · {status.reminders.map(
          source => source === "wechat" ? "微信" : "QQ",
        ).join("、")}</p>}
      </>}
    </section>}
  </>;
}
