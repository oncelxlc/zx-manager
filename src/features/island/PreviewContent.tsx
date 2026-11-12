import * as stylex from "@stylexjs/stylex";
import { ChevronDown, ChevronUp, EyeOff, Pin } from "lucide-react";
import defaults from "./defaults.json";
import { lightSurface } from "./settingsStyles";
import type { AnimationPreset, CustomCurve, IslandConfig } from "./types";

export type PreviewState = "compact" | "expanded" | "pinned" | "edgePeek";
export function motionEasing(preset: AnimationPreset, expanding: boolean, customCurve?: CustomCurve) {
  if (preset === "custom" && customCurve) {
    if (customCurve === "linear") return "linear";
    return customCurve === "easeIn" ? "cubic-bezier(.333333,0,.666667,.333333)" : "cubic-bezier(.333333,.666667,.666667,1)";
  }
  if (preset === "spring") return "cubic-bezier(.22,1.16,.4,1)";
  if (preset === "subtle") return "ease-out";
  return expanding ? "cubic-bezier(.22,1,.36,1)" : "cubic-bezier(.4,0,1,1)";
}

export function PreviewContent({ config, state }: { config: IslandConfig; state: PreviewState }) {
  const { appearance, animation, interaction, edge } = config;
  const expanded = state === "expanded" || state === "pinned";
  const peek = state === "edgePeek";
  const economical = animation.performance === "performance";
  const motion = animation.enabled && !animation.reducedMotion && !economical;
  const contentMs = animation.enabled ? animation.reducedMotion ? Math.min(70, animation.contentMs) : animation.contentMs : 0;
  const easing = motion ? motionEasing(animation.preset, expanded, animation.customCurve) : "linear";
  const offset = motion ? animation.amplitude * (animation.preset === "subtle" ? 2 : 5) : 0;
  const scaleDelta = motion && animation.performance === "quality" ? 0.03 * animation.amplitude : 0;
  const automaticColor = appearance.theme === "system" && appearance.background.toLowerCase() === defaults.appearance.background;
  const background = automaticColor ? `light-dark(${lightSurface}, ${appearance.background})` : appearance.background;
  const foreground = automaticColor ? "light-dark(#181817, #fff)" : ink(appearance.background);
  return (
    <div {...stylex.props(s.shell, s.theme(appearance.theme),
      s.surface(background, appearance.border, appearance.radius, appearance.opacity, foreground),
      appearance.shadow && s.shadow, appearance.blur && !economical && s.blur, peek && s.peek)}>
      {expanded && <ChevronUp size={16} {...stylex.props(s.chevron)} />}
      <div {...stylex.props(s.compact, s.layer(expanded || peek ? 0 : 1, expanded ? -offset : 0, 1, contentMs, easing))}>
        <span {...stylex.props(s.dot)} /><span {...stylex.props(s.compactLabel)}>zx-once</span>
        <ChevronDown size={13} {...stylex.props(s.compactLabel)} />
      </div>
      <div {...stylex.props(s.expanded, s.layer(expanded ? 1 : 0, expanded ? 0 : offset, expanded ? 1 : 1 - scaleDelta, contentMs, easing))}>
        <div {...stylex.props(s.identity)}><span {...stylex.props(s.dot)} />zx-once</div>
        <strong {...stylex.props(s.title)}>灵动岛</strong>
        <div {...stylex.props(s.controls)}>
          {interaction.pinEnabled && <span {...stylex.props(s.icon, state === "pinned" && s.pinned)}><Pin size={17} /></span>}
          <span {...stylex.props(s.icon)}><EyeOff size={17} /></span>
        </div>
      </div>
      {peek && <div {...stylex.props(s.stripArea)}><span {...stylex.props(s.strip(edge.peekSize))} /></div>}
    </div>
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
    userSelect: "none", borderStyle: "solid", borderWidth: 1,
  },
  theme: (theme: string) => ({ colorScheme: theme === "system" ? "light dark" : theme }),
  surface: (background: string, border: string, radius: number, opacity: number, foreground: string) => ({
    borderRadius: `${radius}px`, borderColor: border, color: foreground,
    backgroundColor: `color-mix(in srgb, ${background} ${opacity * 100}%, transparent)`,
  }),
  shadow: { boxShadow: "inset 0 0 14px #0002" },
  blur: { backdropFilter: "blur(12px)" },
  peek: { borderWidth: 0, backgroundColor: "transparent", boxShadow: "none", backdropFilter: "none" },
  stripArea: {
    position: "absolute", inset: 0, display: "grid", placeItems: "center",
  },
  strip: (size: number) => ({
    backgroundColor: "currentColor", borderRadius: 999,
    width: { default: "100%", "@container (aspect-ratio < 1)": `min(100%, ${size}px)` },
    height: { default: `min(100%, ${size}px)`, "@container (aspect-ratio < 1)": "100%" },
  }),
  chevron: { position: "absolute", top: 12, right: 12 },
  compact: { position: "absolute", inset: 0, display: "flex", alignItems: "center",
    justifyContent: "center", gap: 8, flexDirection: { default: "row", "@container (aspect-ratio < 1)": "column" } },
  compactLabel: { display: { default: "inline", "@container (width < 90px)": "none" } },
  dot: { width: 10, height: 10, borderRadius: "50%", backgroundColor: "currentColor", flexShrink: 0 },
  expanded: { position: "absolute", inset: 0, display: "flex", flexDirection: "column", justifyContent: "center",
    alignItems: "flex-start", gap: 8, padding: "clamp(4px, 7cqmin, 20px)" },
  layer: (alpha: number, y: number, scale: number, ms: number, easing: string) => ({
    opacity: alpha, transform: { default: `translateY(${y}px) scale(${scale})`, "@media (prefers-reduced-motion: reduce)": "none" },
    transitionProperty: "opacity, transform", transitionTimingFunction: easing,
    transitionDuration: { default: `${ms}ms`, "@media (prefers-reduced-motion: reduce)": "0ms" },
  }),
  identity: { display: "flex", alignItems: "center", gap: 7, fontSize: 12 },
  title: { fontSize: "clamp(14px, 16cqmin, 25px)", display: { default: "block", "@container (height < 84px)": "none" } },
  controls: { display: "flex", alignItems: "center", gap: 5 },
  icon: { display: "grid", placeItems: "center", width: 32, height: 32, borderRadius: 8 },
  pinned: { backgroundColor: "light-dark(#cdece3,#275447)", transitionProperty: "background-color", transitionDuration: "150ms" },
});
