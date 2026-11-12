import * as stylex from "@stylexjs/stylex";
import { ChevronLeft } from "lucide-react";
import { useState } from "react";
import { Link, useSearchParams } from "react-router";
import defaults from "./defaults.json";
import { islandApi } from "./islandApi";
import { island } from "./islandSlice";
import { IslandPreview } from "./IslandPreview";
import { NotificationSettings } from "./NotificationSettings";
import { Check } from "./SettingsFields";
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

export function IslandSettingsPage() {
  const { config, displays, status, notifications, desktop, dispatch, error, report, retry } = useIsland();
  const saved = desktop ? config : browserDefaults;
  const [edit, setEdit] = useState<IslandConfig | null>(null);
  const draft = edit ?? saved;
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
  function editConfig(next: IslandConfig) { setEdit(next); setNotice(""); }
  return (
    <main {...stylex.props(ui.page)}>
      <div {...stylex.props(ui.wrap)}>
        <Link to="/" {...stylex.props(ui.back)}><ChevronLeft size={18} aria-hidden="true" /> 首页</Link>
        <h1 {...stylex.props(ui.heading)}>灵动岛设置</h1>
        <div {...stylex.props(ui.layout)}>
          <nav aria-label="设置分组" {...stylex.props(ui.nav)}>
            {groupNames.map(key => <button key={key} type="button" aria-current={group === key ? "page" : undefined}
              {...stylex.props(ui.navItem, group === key && ui.current)} onClick={() => setSearch(current => {
                if (key === "overview") current.delete("group"); else current.set("group", key);
                return current;
              }, { replace: true })}>{groups[key]}</button>)}
          </nav>
          <form {...stylex.props(ui.panel)} onSubmit={event => {
            event.preventDefault();
            if (desktop && edit && !busy && !conflict) void commit(() => islandApi.save(edit));
          }}>
            <h2 {...stylex.props(ui.subheading)}>{groups[group]}</h2>
            {!desktop && <p {...stylex.props(ui.message)}>浏览器中可以调整并预览。保存设置和控制窗口请打开 Windows 桌面应用。</p>}
            {error && <p role="alert" {...stylex.props(ui.error)}>{error}</p>}
            {draft ? <>
              <fieldset disabled={busy} {...stylex.props(ui.fields)}>
                {group === "overview" && <>
                  <p {...stylex.props(ui.message)}>窗口状态 · {status ? stateLabels[status.state] : desktop ? "正在连接" : "浏览器预览"}</p>
                  <Check label="启用灵动岛" value={draft.enabled} change={enabled => editConfig({ ...draft, enabled })} />
                  <div {...stylex.props(ui.actions)}>
                    <button type="button" disabled={!desktop || !config?.enabled} {...stylex.props(ui.button)} onClick={() => void perform(islandApi.show)}>显示岛屿</button>
                    <button type="button" disabled={!desktop || !config?.enabled} {...stylex.props(ui.button)} onClick={() => void perform(islandApi.hide)}>隐藏岛屿</button>
                    <button type="button" disabled={!desktop || !config?.interaction.pinEnabled || !status || !["compact", "expanded", "pinned"].includes(status.state)}
                      aria-pressed={status?.state === "pinned"} {...stylex.props(ui.button)}
                      onClick={() => void perform(() => islandApi.pin(status?.state !== "pinned"))}>{status?.state === "pinned" ? "取消固定" : "固定岛屿"}</button>
                    <button type="button" disabled={!desktop || !config?.enabled} {...stylex.props(ui.button)} onClick={() => void perform(islandApi.restart)}>重启岛屿</button>
                  </div>
                </>}
                <SettingsGroups group={group} config={draft} displays={displays}
                  update={(section, patch) => editConfig({ ...draft, [section]: { ...draft[section], ...patch } })} />
                {group === "notifications" && <NotificationSettings config={draft} saved={saved} desktop={desktop}
                  status={notifications} perform={perform} update={patch => editConfig({
                    ...draft, notifications: { ...draft.notifications, ...patch },
                  })} refresh={async () => { dispatch(island.actions.notificationsReceived(await islandApi.notifications())); }} />}
                {(group === "animation" || group === "appearance") && <IslandPreview config={draft} desktop={desktop} report={report} />}
              </fieldset>
              {conflict && <p role="alert" {...stylex.props(ui.error)}>配置已在其他窗口更新。放弃草稿可载入最新设置。</p>}
              <div {...stylex.props(ui.actions)}>
                <button type="submit" disabled={!desktop || !edit || busy || Boolean(conflict)} {...stylex.props(ui.button, ui.primary)}>保存设置</button>
                {edit && <button type="button" disabled={busy} {...stylex.props(ui.button)}
                  onClick={() => { setEdit(null); setNotice(""); }}>放弃草稿</button>}
                {group === "overview" && <button type="button" disabled={!desktop || !saved || busy} {...stylex.props(ui.button)}
                  onClick={() => { if (saved) void commit(() => islandApi.reset(saved.revision)); }}>恢复默认设置</button>}
              </div>
              <p role="status" {...stylex.props(ui.message)}>{busy ? "正在处理…" : notice || (edit ? "有未保存的更改" : "")}</p>
            </> : error ? <button type="button" {...stylex.props(ui.button)} onClick={retry}>重新读取设置</button>
              : <p role="status">正在读取桌面设置…</p>}
          </form>
        </div>
      </div>
    </main>
  );
}
