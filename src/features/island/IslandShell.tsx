import * as stylex from "@stylexjs/stylex";
import { ChevronDown, ChevronUp, EyeOff, Pin } from "lucide-react";
import { useEffect, useState, type HTMLAttributes } from "react";
import defaults from "./defaults.json";
import type { AnimationPreset, IslandConfig, IslandState } from "./types";

export const stateLabels: Record<IslandState, string> = {
  disabled: "已停用", hidden: "已隐藏", edgePeek: "边缘待唤醒", compact: "已收起",
  expanded: "已展开", pinned: "已固定", suspended: "全屏暂停",
};

export const lightSurface = "#f5f5f4";

export function motionEasing(preset: AnimationPreset, expanding: boolean) {
  if (preset === "spring") return "cubic-bezier(.22,1.16,.4,1)";
  if (preset === "subtle") return "ease-out";
  return expanding ? "cubic-bezier(.22,1,.36,1)" : "cubic-bezier(.4,0,1,1)";
}

interface ShellProps {
  config: IslandConfig;
  state: IslandState;
  onClick?: () => void;
  onPin?: () => void;
  onHide?: () => void;
  events?: Pick<HTMLAttributes<HTMLElement>,
    "onPointerEnter" | "onPointerLeave" | "onPointerDown" | "onPointerMove" | "onPointerUp" | "onPointerCancel">;
}

function CompactView() {
  return <><span aria-hidden="true" {...stylex.props(s.dot)} /><span {...stylex.props(s.compactLabel)}>zx-once</span></>;
}

function ExpandedView({ state, canPin, onPin, onHide }: {
  state: IslandState; canPin: boolean; onPin?: () => void; onHide?: () => void;
}) {
  const pinned = state === "pinned";
  return (
    <>
      <div {...stylex.props(s.identity)}><span aria-hidden="true" {...stylex.props(s.dot)} />zx-once</div>
      <strong {...stylex.props(s.title)}>灵动岛</strong>
      <div data-island-actions="" {...stylex.props(s.controls)}>
        {canPin && <button type="button" aria-label={pinned ? "取消固定" : "固定岛屿"} aria-pressed={pinned}
          {...stylex.props(s.iconButton, pinned && s.pinned)} onClick={onPin}>
          <Pin size={17} aria-hidden="true" />
        </button>}
        <button type="button" aria-label="隐藏岛屿" {...stylex.props(s.iconButton)} onClick={onHide}>
          <EyeOff size={17} aria-hidden="true" />
        </button>
      </div>
    </>
  );
}

export function IslandShell({ config, state, events, onClick, onPin, onHide }: ShellProps) {
  const { appearance, animation, interaction, edge } = config;
  const expanded = state === "expanded" || state === "pinned";
  const peek = state === "edgePeek";
  const visible = !["hidden", "disabled", "suspended"].includes(state);
  const [entered, setEntered] = useState(false);
  useEffect(() => {
    if (!visible) { setEntered(false); return; }
    let next = 0;
    const frame = requestAnimationFrame(() => { next = requestAnimationFrame(() => setEntered(true)); });
    return () => { cancelAnimationFrame(frame); cancelAnimationFrame(next); };
  }, [visible]);
  if (!visible) return null;

  const economical = animation.performance === "performance";
  const motion = animation.enabled && !animation.reducedMotion && !economical;
  const clickEnabled = peek || (interaction.clickExpand && state !== "pinned");
  const contentMs = animation.enabled ? animation.reducedMotion ? Math.min(70, animation.contentMs) : animation.contentMs : 0;
  const easing = motion ? motionEasing(animation.preset, expanded) : "linear";
  const offset = motion ? animation.amplitude * (animation.preset === "subtle" ? 2 : 5) : 0;
  const scaleDelta = motion && animation.performance === "quality" ? 0.03 * animation.amplitude : 0;
  const automaticColor = appearance.theme === "system" && appearance.background.toLowerCase() === defaults.appearance.background;
  const background = automaticColor ? `light-dark(${lightSurface}, ${appearance.background})` : appearance.background;
  const foreground = automaticColor ? "light-dark(#181817, #fff)" : ink(appearance.background);
  return (
    <section aria-label="灵动岛" {...events} {...stylex.props(s.shell, s.theme(appearance.theme),
      s.surface(background, appearance.border, appearance.radius, appearance.opacity, foreground),
      appearance.shadow && s.shadow, appearance.blur && !economical && s.blur, peek && s.peek,
      s.entry(entered || !animation.enabled ? 1 : 0, entered || !motion ? 1 : 1 - 0.025 * animation.amplitude,
        animation.enabled ? motion ? 200 : 60 : 0))}>
      <button type="button" aria-label={peek ? "唤醒灵动岛" : expanded ? "收起灵动岛" : "展开灵动岛"} aria-expanded={expanded}
        aria-disabled={!clickEnabled} tabIndex={clickEnabled ? 0 : -1}
        onClick={clickEnabled ? onClick : undefined} {...stylex.props(s.hit)}>
        {expanded && <ChevronUp size={16} aria-hidden="true" {...stylex.props(s.chevron)} />}
      </button>
      <div aria-hidden="true" {...stylex.props(s.compact, s.layer(expanded || peek ? 0 : 1, expanded ? -offset : 0, 1, contentMs, easing))}>
        <CompactView /><ChevronDown size={13} aria-hidden="true" {...stylex.props(s.compactLabel)} />
      </div>
      <div aria-hidden={!expanded} inert={!expanded}
        {...stylex.props(s.expanded, s.layer(expanded ? 1 : 0, expanded ? 0 : offset, expanded ? 1 : 1 - scaleDelta, contentMs, easing))}>
        <ExpandedView state={state} canPin={interaction.pinEnabled} onPin={onPin} onHide={onHide} />
      </div>
      {peek && <div aria-hidden="true" {...stylex.props(s.stripArea)}><span {...stylex.props(s.strip(edge.peekSize))} /></div>}
    </section>
  );
}

function ink(hex: string) {
  const channels = [1, 3, 5].map(start => {
    const channel = parseInt(hex.slice(start, start + 2), 16) / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722 > 0.179 ? "#111111" : "#ffffff";
}

const s = stylex.create({
  shell: {
    position: "relative", width: "100%", height: "100%", overflow: "hidden", containerType: "size",
    userSelect: "none", touchAction: "none", borderStyle: "solid", borderWidth: 1,
    transformOrigin: "center", transitionProperty: "opacity, transform", transitionTimingFunction: "ease-out",
  },
  theme: (theme: string) => ({ colorScheme: theme === "system" ? "light dark" : theme }),
  surface: (background: string, border: string, radius: number, opacity: number, foreground: string) => ({
    borderRadius: `${radius}px`, borderColor: border, color: foreground,
    backgroundColor: `color-mix(in srgb, ${background} ${opacity * 100}%, transparent)`,
  }),
  entry: (opacity: number, scale: number, ms: number) => ({
    opacity, transform: { default: `scale(${scale})`, "@media (prefers-reduced-motion: reduce)": "none" },
    transitionDuration: { default: `${ms}ms`, "@media (prefers-reduced-motion: reduce)": "0ms" },
  }),
  shadow: { boxShadow: "inset 0 0 14px #0002" },
  blur: { backdropFilter: "blur(12px)" },
  peek: { borderWidth: 0, backgroundColor: "transparent", boxShadow: "none", backdropFilter: "none" },
  stripArea: {
    position: "absolute", inset: 0, display: "grid", placeItems: "center", pointerEvents: "none",
  },
  strip: (size: number) => ({
    backgroundColor: "currentColor", borderRadius: 999,
    width: { default: "100%", "@container (aspect-ratio < 1)": `min(100%, ${size}px)` },
    height: { default: `min(100%, ${size}px)`, "@container (aspect-ratio < 1)": "100%" },
  }),
  hit: {
    position: "absolute", inset: 0, width: "100%", height: "100%", color: "inherit", borderRadius: "inherit",
    backgroundColor: "transparent", borderWidth: 0, padding: 0, cursor: "pointer", outlineOffset: -4,
  },
  chevron: { position: "absolute", top: 12, right: 12 },
  compact: { position: "absolute", inset: 0, pointerEvents: "none", display: "flex", alignItems: "center",
    justifyContent: "center", gap: 8, flexDirection: { default: "row", "@container (aspect-ratio < 1)": "column" } },
  compactLabel: { display: { default: "inline", "@container (width < 90px)": "none" } },
  dot: { width: 10, height: 10, borderRadius: "50%", backgroundColor: "currentColor", flexShrink: 0 },
  expanded: { position: "absolute", inset: 0, display: "flex", flexDirection: "column", justifyContent: "center",
    alignItems: "flex-start", gap: 8, padding: "clamp(4px, 7cqmin, 20px)", pointerEvents: "none" },
  layer: (alpha: number, y: number, scale: number, ms: number, easing: string) => ({
    opacity: alpha, transform: { default: `translateY(${y}px) scale(${scale})`, "@media (prefers-reduced-motion: reduce)": "none" },
    transitionProperty: "opacity, transform", transitionTimingFunction: easing,
    transitionDuration: { default: `${ms}ms`, "@media (prefers-reduced-motion: reduce)": "0ms" },
  }),
  identity: { display: "flex", alignItems: "center", gap: 7, fontSize: 12 },
  title: { fontSize: "clamp(14px, 16cqmin, 25px)", display: { default: "block", "@container (height < 84px)": "none" } },
  controls: { display: "flex", alignItems: "center", gap: 5, pointerEvents: "auto" },
  iconButton: { display: "grid", placeItems: "center", width: 32, height: 32, borderRadius: 8, borderWidth: 0,
    cursor: "pointer", color: "inherit", backgroundColor: { default: "transparent", ":hover": "light-dark(#0001,#fff2)" } },
  pinned: { backgroundColor: "light-dark(#cdece3,#275447)", transitionProperty: "background-color", transitionDuration: "150ms" },
});
