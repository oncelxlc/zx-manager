import type { IslandDisplay } from "./islandApi";
import { FieldDescription } from "../../components/ui/field";
import defaults from "./defaults.json";
import { isDefaultDarkSurface, lightSurface } from "./settingsStyles";
import type { IslandConfig } from "./types";
import { Amount, Check, Choice, ColorField, IslandSlider } from "./SettingsFields";

export const groups = {
  overview: "总览", startup: "启动管理", placement: "位置与方向", edge: "边界隐藏",
  notifications: "通知", appearance: "外观", animation: "动画效果", interaction: "鼠标交互",
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
  const { startup, placement, edge, appearance, animation, interaction } = config;
  switch (group) {
    case "notifications":
    case "overview": return null;
    case "startup": return <>
      <Check label="登录 Windows 时启动" value={startup.launchAtLogin} change={launchAtLogin => update(group, { launchAtLogin })} />
      <Check label="应用启动时显示岛屿" value={startup.showOnStart} change={showOnStart => update(group, { showOnStart })} />
      <Check label="启动时收起" value={startup.startCompact} change={startCompact => update(group, { startCompact })} />
    </>;
    case "placement": {
      const options: [string, string][] = [["", "跟随主显示器"], ...displays.map((d): [string, string] => [d.id, d.primary ? `${d.name}（主显示器）` : d.name])];
      if (placement.displayId && !displays.some(d => d.id === placement.displayId)) {
        options.push([placement.displayId, `${placement.displayId}（未连接）`]);
      }
      const side = placement.anchor === "centerLeft" || placement.anchor === "centerRight";
      return <>
        <Choice label="目标显示器" value={placement.displayId ?? ""} options={options} change={id => update(group, { displayId: id || null })} />
        <Choice label="停靠位置" value={placement.anchor} change={anchor => update(group, { anchor, offsetX: 0, offsetY: 0 })} options={[
          ["topCenter", "顶部"], ["centerLeft", "左侧"], ["centerRight", "右侧"],
        ]} />
        {side ? <IslandSlider position="vertical" value={placement.offsetY} change={offsetY => update(group, { offsetY })} />
          : <IslandSlider position="horizontal" value={placement.offsetX} change={offsetX => update(group, { offsetX })} />}
        <Check label="记住拖动后的位置" value={placement.rememberPosition} change={rememberPosition => update(group, { rememberPosition })} />
      </>;
    }
    case "edge": {
      const mode = edge.autoHide && edge.mode !== "disabled" ? edge.mode : "compact";
      return <>
        <Choice label="指针离开并收起后" value={mode} change={mode => update(group, { mode, autoHide: true })} options={[
          ["compact", "保持收起"], ["peek", "仅露出边缘条"], ["fullHide", "完全隐藏"],
        ]} />
        {mode === "fullHide" && <FieldDescription>完全隐藏后，可从托盘菜单重新显示。</FieldDescription>}
        {mode !== "compact" && <Amount label="隐藏延迟（毫秒）" value={edge.hideDelayMs} min={0} max={60000} change={hideDelayMs => update(group, { hideDelayMs })} />}
        {mode === "peek" && <Amount label="唤醒延迟（毫秒）" value={edge.wakeDelayMs} min={0} max={60000} change={wakeDelayMs => update(group, { wakeDelayMs })} />}
        <Check label="全屏应用运行时隐藏" value={edge.hideOnFullscreen} change={hideOnFullscreen => update(group, { hideOnFullscreen })} />
      </>;
    }
    case "appearance": return <>
      <Choice label="岛屿主题" value={appearance.theme} change={theme => {
        const base = defaults.appearance.background;
        const usePalette = isDefaultDarkSurface(appearance.background) || appearance.background.toLowerCase() === lightSurface;
        const background = usePalette ? (theme === "light" ? lightSurface : base) : appearance.background;
        const border = appearance.border.toLowerCase() === appearance.background.toLowerCase() ? background : appearance.border;
        update(group, { theme, background, border });
      }} options={[["system", "跟随系统"], ["light", "浅色"], ["dark", "深色"]]} />
      <ColorField label="背景色" value={appearance.background} change={background => update(group, { background })} />
      <ColorField label="边框色" value={appearance.border} change={border => update(group, { border })} />
      <Choice label="岛体形态" value={appearance.shape} change={shape => update(group, { shape })}
        options={[["notch", "刘海"], ["pill", "胶囊"]]} />
      <IslandSlider value={appearance.islandSize} change={islandSize => update(group, { islandSize })} />
      <Check label="极简模式" value={appearance.minimal} change={minimal => update(group, { minimal })} />
      <Check label="显示伙伴" value={appearance.pet} change={pet => update(group, { pet })} />
      <Check label="显示电池" value={appearance.showBattery} change={showBattery => update(group, { showBattery })} />
      <Check label="展开时显示歌词" value={appearance.lyricsEnabled} change={lyricsEnabled => update(group, { lyricsEnabled })} />
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
      <Check label="点击展开" value={interaction.clickExpand} change={clickExpand => update(group, { clickExpand })} />
      <Check label="允许固定" value={interaction.pinEnabled} change={pinEnabled => update(group, { pinEnabled })} />
      <Check label="允许拖动" value={interaction.dragEnabled} change={dragEnabled => update(group, { dragEnabled })} />
    </>;
  }
}
