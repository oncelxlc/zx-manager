import { Button } from "../../components/ui/button";
import { FieldDescription } from "../../components/ui/field";
import { islandApi, type NotificationPermission, type NotificationStatus } from "./islandApi";
import { Check } from "./SettingsFields";

const permissionLabels: Record<NotificationPermission, string> = {
  disabled: "系统通知监听已关闭。",
  unavailable: "当前安装方式或系统不支持通知访问。仍可从灵动岛切换窗口。",
  notRequested: "授权后，灵动岛可显示来自 Windows 通知中心的提醒。",
  denied: "通知访问已被拒绝，可在 Windows 隐私设置中更改。",
  allowed: "已获准读取 Windows 通知。",
  error: "暂时无法读取 Windows 通知，可重新请求访问。",
};

interface Props {
  enabled: boolean;
  listening: boolean;
  status: NotificationStatus | null;
  change: (enabled: boolean) => void;
  perform: (action: () => Promise<unknown>) => Promise<void>;
  refresh: () => Promise<void>;
}

export function NotificationSettings({ enabled, listening, status, change, perform, refresh }: Props) {
  return <>
    <Check label="启用通知" value={enabled} change={change} />
    <FieldDescription>灵动岛会自动显示正在运行的受支持应用。有 Windows 系统提醒时，应用图标旁会出现红点。</FieldDescription>
    {enabled && listening && <section aria-label="Windows 通知访问">
      <h3>Windows 通知访问</h3>
      <FieldDescription role="status">{status ? permissionLabels[status.permission] : "正在读取访问状态…"}</FieldDescription>
      {(status?.permission === "notRequested" || status?.permission === "error") &&
        <Button variant="outline" onClick={() => void perform(async () => {
          await islandApi.requestNotificationAccess();
          await refresh();
        })}>请求 Windows 通知访问</Button>}
      {status?.permission === "denied" && <Button variant="outline"
        onClick={() => void perform(islandApi.openNotificationSettings)}>打开 Windows 通知隐私设置</Button>}
      {!status && <Button variant="outline" onClick={() => void perform(refresh)}>重新读取访问状态</Button>}
    </section>}
  </>;
}
