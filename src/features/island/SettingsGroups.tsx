import type { IslandDisplay } from "./islandApi";
import defaults from "./defaults.json";
import { lightSurface } from "./settingsStyles";
import type { IslandConfig } from "./types";
import { Amount, Check, Choice, TextField } from "./SettingsFields";

export const groups = {
  overview: "总览", startup: "启动管理", placement: "位置与方向", edge: "边界隐藏",
  appearance: "外观", animation: "动画效果", interaction: "交互与快捷键",
} as const;
export type Group = keyof typeof groups;

type Section = Exclude<keyof IslandConfig, "schemaVersion" | "revision" | "enabled">;

interface Props {
  group: Group;
  config: IslandConfig;
  displays: IslandDisplay[];
  update: <K extends Section>(section: K, patch: Partial<IslandConfig[K]>) => void;
}

export function SettingsGroups({ group, config, displays, update }: Props) {
  const { startup, placement, edge, appearance, animation, interaction, hotkeys } = config;
  switch (group) {
    case "overview": return null;
    case "startup": return <>
      <Check label="登录 Windows 时启动" value={startup.launchAtLogin} change={launchAtLogin => update(group, { launchAtLogin })} />
      <Check label="应用启动时显示岛屿" value={startup.showOnStart} change={showOnStart => update(group, { showOnStart })} />
      <Check label="启动时收起" value={startup.startCompact} change={startCompact => update(group, { startCompact })} />
      <Check label="关闭主窗口后留在托盘" value={startup.closeToTray} change={closeToTray => update(group, { closeToTray })} />
    </>;
    case "placement": {
      const options: [string, string][] = [["", "跟随主显示器"], ...displays.map((d): [string, string] => [d.id, d.primary ? `${d.name}（主显示器）` : d.name])];
      if (placement.displayId && !displays.some(d => d.id === placement.displayId)) {
        options.push([placement.displayId, `${placement.displayId}（未连接）`]);
      }
      return <>
        <Choice label="目标显示器" value={placement.displayId ?? ""} options={options} change={id => update(group, { displayId: id || null })} />
        <Choice label="停靠位置" value={placement.anchor} change={anchor => update(group, { anchor })} options={[
          ["topLeft", "左上"], ["topCenter", "上方居中"], ["topRight", "右上"], ["centerLeft", "左侧居中"], ["center", "中央"],
          ["centerRight", "右侧居中"], ["bottomLeft", "左下"], ["bottomCenter", "下方居中"], ["bottomRight", "右下"], ["custom", "自定义坐标"],
        ]} />
        <Amount label={placement.anchor === "custom" ? "工作区 X 坐标（逻辑像素）" : "水平偏移（逻辑像素）"} value={placement.offsetX}
          min={-65535} max={65535} step="any" change={offsetX => update(group, { offsetX })} />
        <Amount label={placement.anchor === "custom" ? "工作区 Y 坐标（逻辑像素）" : "垂直偏移（逻辑像素）"} value={placement.offsetY}
          min={-65535} max={65535} step="any" change={offsetY => update(group, { offsetY })} />
        <Choice label="布局方向" value={placement.orientation} change={orientation => update(group, { orientation })}
          options={[["auto", "自动"], ["horizontal", "横向"], ["vertical", "纵向"]]} />
        <Choice label="展开方向" value={placement.expandDirection} change={expandDirection => update(group, { expandDirection })}
          options={[["auto", "自动"], ["up", "向上"], ["down", "向下"], ["left", "向左"], ["right", "向右"]]} />
        <Check label="记住拖动后的位置" value={placement.rememberPosition} change={rememberPosition => update(group, { rememberPosition })} />
      </>;
    }
    case "edge": return <>
      <Choice label="边界模式" value={edge.mode} change={mode => update(group, { mode })} options={[
        ["disabled", "始终完整显示"], ["compact", "离开后收起"], ["peek", "仅露出边缘条"], ["fullHide", "完全隐藏"],
      ]} />
      {edge.mode === "fullHide" && <p>完全隐藏后，可从托盘或使用全局快捷键唤醒。</p>}
      <Check label="指针离开后自动隐藏" value={edge.autoHide} change={autoHide => update(group, { autoHide })} />
      <Amount label="隐藏延迟（毫秒）" value={edge.hideDelayMs} min={0} max={60000} change={hideDelayMs => update(group, { hideDelayMs })} />
      <Amount label="唤醒延迟（毫秒）" value={edge.wakeDelayMs} min={0} max={60000} change={wakeDelayMs => update(group, { wakeDelayMs })} />
      <Amount label="边缘触发宽度（逻辑像素）" value={edge.triggerWidth} min={2} max={20} change={triggerWidth => update(group, { triggerWidth })} />
      <Amount label="边缘露出尺寸（逻辑像素）" value={edge.peekSize} min={2} max={20} change={peekSize => update(group, { peekSize })} />
      <Check label="全屏应用运行时隐藏" value={edge.hideOnFullscreen} change={hideOnFullscreen => update(group, { hideOnFullscreen })} />
    </>;
    case "appearance": return <>
      <Choice label="岛屿主题" value={appearance.theme} change={theme => {
        const base = defaults.appearance.background;
        const usePalette = [base, lightSurface].includes(appearance.background.toLowerCase());
        update(group, { theme, background: usePalette ? theme === "light" ? lightSurface : base : appearance.background });
      }} options={[["system", "跟随系统"], ["light", "浅色"], ["dark", "深色"]]} />
      <TextField color label="背景色" value={appearance.background} change={background => update(group, { background })} />
      <TextField color label="边框色" value={appearance.border} change={border => update(group, { border })} />
      <Amount label="收起宽度（逻辑像素）" value={appearance.compactWidth} min={24} max={2048} change={compactWidth => update(group, { compactWidth })} />
      <Amount label="收起高度（逻辑像素）" value={appearance.compactHeight} min={24} max={2048} change={compactHeight => update(group, { compactHeight })} />
      <Amount label="展开宽度（逻辑像素）" value={appearance.expandedWidth} min={Math.max(48, appearance.compactWidth)} max={4096} change={expandedWidth => update(group, { expandedWidth })} />
      <Amount label="展开高度（逻辑像素）" value={appearance.expandedHeight} min={Math.max(48, appearance.compactHeight)} max={4096} change={expandedHeight => update(group, { expandedHeight })} />
      <Amount label="圆角（逻辑像素）" value={appearance.radius} min={0} max={1024} change={radius => update(group, { radius })} />
      <Amount label="背景不透明度" value={appearance.opacity} step={0.01} min={0} max={1} change={opacity => update(group, { opacity })} />
      <Check label="阴影" value={appearance.shadow} change={shadow => update(group, { shadow })} />
      <Check label="毛玻璃效果" value={appearance.blur} change={blur => update(group, { blur })} />
    </>;
    case "animation": return <>
      <Check label="启用动画" value={animation.enabled} change={enabled => update(group, { enabled })} />
      <Choice label="动画预设" value={animation.preset} change={preset => update(group, { preset })}
        options={[["subtle", "轻盈 · Subtle"], ["smooth", "平滑 · Smooth"], ["spring", "弹性 · Spring"], ["custom", "自定义 · Custom"]]} />
      {animation.preset === "custom" && <Choice label="过渡曲线" value={animation.customCurve ?? ""}
        options={[["", "默认平滑"], ["linear", "匀速"], ["easeIn", "渐快"], ["easeOut", "渐慢"]]}
        change={curve => update(group, { customCurve: curve || undefined })} />}
      <Amount label="展开时长（毫秒）" value={animation.expandMs} min={0} max={2000} change={expandMs => update(group, { expandMs })} />
      <Amount label="收起时长（毫秒）" value={animation.collapseMs} min={0} max={2000} change={collapseMs => update(group, { collapseMs })} />
      <Amount label="边缘时长（毫秒）" value={animation.edgeMs} min={0} max={2000} change={edgeMs => update(group, { edgeMs })} />
      <Amount label="内容切换时长（毫秒）" value={animation.contentMs} min={0} max={2000} change={contentMs => update(group, { contentMs })} />
      <Amount label="动画幅度" value={animation.amplitude} min={0} max={2} step={0.1} change={amplitude => update(group, { amplitude })} />
      <Check label="减少动态效果" value={animation.reducedMotion} change={reducedMotion => update(group, { reducedMotion })} />
      <Choice label="性能档位" value={animation.performance} change={performance => update(group, { performance })}
        options={[["performance", "性能优先"], ["balanced", "均衡"], ["quality", "高质量"]]} />
    </>;
    case "interaction": return <>
      <Check label="悬停展开" value={interaction.hoverExpand} change={hoverExpand => update(group, { hoverExpand })} />
      <Check label="点击切换展开状态" value={interaction.clickExpand} change={clickExpand => update(group, { clickExpand })} />
      <Check label="点击岛屿外部时收起" value={interaction.outsideCollapse} change={outsideCollapse => update(group, { outsideCollapse })} />
      <Check label="离开后自动收起" value={interaction.autoCollapse} change={autoCollapse => update(group, { autoCollapse })} />
      <Amount label="收起延迟（毫秒）" value={interaction.collapseDelayMs} min={0} max={60000} change={collapseDelayMs => update(group, { collapseDelayMs })} />
      <Check label="允许固定" value={interaction.pinEnabled} change={pinEnabled => update(group, { pinEnabled })} />
      <Check label="允许拖动" value={interaction.dragEnabled} change={dragEnabled => update(group, { dragEnabled })} />
      <h3>全局快捷键</h3>
      <p>输入修饰键和一个字母、数字或功能键，例如 Ctrl+Alt+I。留空可停用该快捷键。</p>
      <TextField label="切换显示" value={hotkeys.toggle} change={toggle => update("hotkeys", { toggle })} />
      <TextField label="显示岛屿" value={hotkeys.show} change={show => update("hotkeys", { show })} />
      <TextField label="隐藏岛屿" value={hotkeys.hide} change={hide => update("hotkeys", { hide })} />
    </>;
  }
}
