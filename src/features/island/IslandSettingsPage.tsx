import * as stylex from "@stylexjs/stylex";
import { useLayoutEffect, useState } from "react";
import { useSearchParams } from "react-router";
import { PageHeader, Panel } from "../../app/workbenchUi";
import { Alert, AlertDescription } from "../../components/ui/alert";
import { Button } from "../../components/ui/button";
import { FieldDescription, FieldGroup, FieldSet } from "../../components/ui/field";
import { mainWindowSession } from "../../mainWindowSession";
import defaults from "./defaults.json";
import { islandApi } from "./islandApi";
import { island } from "./islandSlice";
import { IslandPreview } from "./IslandPreview";
import { NotificationSettings } from "./NotificationSettings";
import { Check, IslandFieldsDisabled } from "./SettingsFields";
import { groups, SettingsGroups, type Group } from "./SettingsGroups";
import { ui } from "./settingsStyles";
import type { IslandConfig, IslandState } from "./types";
import { useIsland } from "./useIsland";

// The Rust defaults test compares this JSON fixture with IslandConfig::default().
const browserDefaults = defaults as IslandConfig;
const stateLabels: Record<IslandState, string> = {
  disabled: "已停用", hidden: "已隐藏", edgePeek: "边缘待唤醒", compact: "已收起",
  expanded: "已展开", pinned: "已固定", suspended: "全屏暂停",
};

function settingsJson(config: IslandConfig) {
  let valid = true;
  const json = JSON.stringify(config, (key, value: unknown) => {
    if (key === "revision") return undefined;
    if (typeof value === "number" && !Number.isFinite(value)) valid = false;
    return value && typeof value === "object"
      ? Object.fromEntries(Object.entries(value).sort(([a], [b]) => a.localeCompare(b)))
      : value;
  });
  return valid ? json : undefined;
}

export function IslandSettingsPage() {
  const { config, displays, status, notifications, desktop, dispatch, error, report, retry } = useIsland();
  const saved = desktop ? config : browserDefaults;
  const [edit, setEdit] = useState<IslandConfig | null>(() => mainWindowSession.draft);
  useLayoutEffect(() => {
    const pending = edit && settingsJson(edit);
    // Keep the last serializable snapshot while a numeric input is temporarily empty.
    if (pending === undefined) return;
    if (saved && pending === settingsJson(saved)) {
      mainWindowSession.draft = null;
      setEdit(null);
    } else {
      mainWindowSession.draft = edit;
    }
  }, [edit, saved]);
  useLayoutEffect(() => () => { mainWindowSession.draft = null; }, []);
  const draft = saved && (edit ?? saved);
  const [search, setSearch] = useSearchParams();
  const groupNames = Object.keys(groups) as Group[];
  const group = groupNames.find(name => name === search.get("group")) ?? "overview";
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const conflict = edit && saved && edit.revision !== saved.revision;

  async function perform(action: () => Promise<unknown>, message = "") {
    setBusy(true);
    setNotice("");
    dispatch(island.actions.errorChanged(null));
    try { await action(); setNotice(message); }
    catch (reason) { report(reason); }
    finally { setBusy(false); }
  }
  async function commit(action: () => Promise<IslandConfig>) {
    await perform(async () => {
      dispatch(island.actions.configReceived(await action()));
      setEdit(null);
    }, "设置已保存");
  }
  function editConfig(next: IslandConfig) {
    setEdit(next);
    setNotice("");
  }
  return (
    <>
      <PageHeader eyebrow="设置" title="灵动岛设置" description="管理岛体外观、停靠位置和桌面交互。" />
      <div {...stylex.props(ui.layout)}>
        <nav aria-label="设置分组" {...stylex.props(ui.nav)}>
          {groupNames.map(key => <Button key={key} size="sm" variant={group === key ? "secondary" : "ghost"}
            aria-current={group === key ? "page" : undefined} className="justify-start" onClick={() => setSearch(current => {
              if (key === "overview") current.delete("group"); else current.set("group", key);
              return current;
            }, { replace: true })}>{groups[key]}</Button>)}
        </nav>
        <Panel title={groups[group]}>
          <form onSubmit={event => {
            event.preventDefault();
            if (desktop && saved && edit && !busy && !conflict) void commit(() => islandApi.save(edit));
          }}>
            <FieldGroup>
              {!desktop && <Alert role="note"><AlertDescription>浏览器中可以调整并预览。保存设置和控制窗口请打开 Windows 桌面应用。</AlertDescription></Alert>}
              {error && <Alert variant="destructive"><AlertDescription>{error}</AlertDescription></Alert>}
              {draft ? <>
                <IslandFieldsDisabled.Provider value={busy}>
                  <FieldSet disabled={busy} aria-label={groups[group]}>
                    {group === "overview" && <>
                      <FieldDescription>窗口状态 · {status ? stateLabels[status.state] : desktop ? "正在连接" : "浏览器预览"}</FieldDescription>
                      <Check label="启用灵动岛" value={draft.enabled} change={enabled => editConfig({ ...draft, enabled })} />
                      <div {...stylex.props(ui.actions)}>
                        <Button variant="outline" disabled={!desktop || !config?.enabled} onClick={() => void perform(islandApi.show)}>显示岛屿</Button>
                        <Button variant="outline" disabled={!desktop || !config?.enabled} onClick={() => void perform(islandApi.hide)}>隐藏岛屿</Button>
                        <Button variant="outline" disabled={!desktop || !config?.interaction.pinEnabled || !status || !["compact", "expanded", "pinned"].includes(status.state)}
                          aria-pressed={status?.state === "pinned"}
                          onClick={() => void perform(() => islandApi.pin(status?.state !== "pinned"))}>{status?.state === "pinned" ? "取消固定" : "固定岛屿"}</Button>
                        <Button variant="outline" disabled={!desktop || !config?.enabled} onClick={() => void perform(islandApi.restart)}>重启岛屿</Button>
                      </div>
                    </>}
                    <SettingsGroups group={group} config={draft} displays={displays}
                      update={(section, patch) => editConfig({ ...draft, [section]: { ...draft[section], ...patch } })} />
                    {group === "notifications" && <NotificationSettings enabled={draft.notifications.enabled}
                      listening={Boolean(desktop && saved?.enabled && saved.notifications.enabled)} status={notifications}
                      change={enabled => editConfig({ ...draft, notifications: { enabled } })} perform={perform}
                      refresh={async () => { dispatch(island.actions.notificationsReceived(await islandApi.notifications())); }} />}
                    {(group === "animation" || group === "appearance") && <IslandPreview config={draft} desktop={desktop} report={report} />}
                  </FieldSet>
                </IslandFieldsDisabled.Provider>
                {conflict && <Alert variant="destructive"><AlertDescription>配置已在其他窗口更新。放弃草稿可载入最新设置。</AlertDescription></Alert>}
                <div {...stylex.props(ui.actions)}>
                  <Button type="submit" disabled={!desktop || !saved || !edit || busy || Boolean(conflict)}>保存设置</Button>
                  {edit && <Button variant="outline" disabled={busy}
                    onClick={() => { setEdit(null); setNotice(""); }}>放弃草稿</Button>}
                  {group === "overview" && <Button variant="outline" disabled={!saved || busy || Boolean(conflict)}
                    onClick={() => { if (saved) editConfig({ ...browserDefaults, revision: saved.revision }); }}>恢复默认设置</Button>}
                </div>
                <FieldDescription role="status">{busy ? "正在处理…" : notice || (edit ? "有未保存的更改" : "")}</FieldDescription>
              </> : error ? <Button variant="outline" onClick={retry}>重新读取设置</Button>
                : <p role="status">正在读取桌面设置…</p>}
            </FieldGroup>
          </form>
        </Panel>
      </div>
    </>
  );
}
