import * as stylex from "@stylexjs/stylex";
import { isDefaultDarkSurface, lightSurface } from "./settingsStyles";
import type { AnimationPreset, CustomCurve, IslandConfig } from "./types";

export type PreviewState = "compact" | "expanded" | "edgePeek" | "hidden";
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
  const { appearance, animation, edge } = config;
  const expanded = state === "expanded";
  const peek = state === "edgePeek";
  const economical = animation.performance === "performance";
  const motion = animation.enabled && !animation.reducedMotion && !economical;
  const contentMs = motion ? animation.contentMs : 0;
  const easing = motion ? motionEasing(animation.preset, expanded, animation.customCurve) : "linear";
  const automaticColor = appearance.theme === "system" && isDefaultDarkSurface(appearance.background);
  const background = automaticColor ? `light-dark(${lightSurface}, ${appearance.background})` : appearance.background;
  const foreground = automaticColor ? "light-dark(#181817, #fff)" : ink(appearance.background);
  const accent = automaticColor ? "light-dark(#177154, #69ddb2)" : foreground === "#ffffff" ? "#69ddb2" : "#177154";
  const now = new Date();
  return (
    <div {...stylex.props(s.shell, s.theme(appearance.theme),
      s.surface(background, appearance.border, appearance.radius, appearance.opacity, foreground),
      appearance.shadow && s.shadow, appearance.blur && !economical && s.blur, peek && s.peek)}>
      <div {...stylex.props(s.face, s.clockLayer(peek ? 0 : 1, expanded ? -10 : 0, contentMs, easing))}>
        <span {...stylex.props(s.ring(accent))}><span {...stylex.props(s.dot)} /></span>
        <time {...stylex.props(s.clock)}>{now.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false })}</time>
      </div>
      <span {...stylex.props(s.date, s.clockLayer(expanded ? 1 : 0, expanded ? 0 : animation.amplitude * 4, contentMs, easing))}>
        {now.toLocaleDateString("zh-CN", { month: "long", day: "numeric" })}
      </span>
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
  face: { position: "absolute", inset: 0, display: "flex", alignItems: "center", justifyContent: "center", gap: 8,
    flexDirection: { default: "row", "@container (aspect-ratio < 1)": "column" } },
  ring: (accent: string) => ({ display: "grid", placeItems: "center", flexShrink: 0, width: 16, height: 16, color: accent,
    borderWidth: 2, borderStyle: "solid", borderColor: "currentColor", borderRadius: "50%" }),
  dot: { width: 4, height: 4, borderRadius: "50%", backgroundColor: "currentColor" },
  clock: { fontWeight: 600, fontVariantNumeric: "tabular-nums", fontSize: "clamp(11px, 10cqw, 22px)", lineHeight: 1,
    display: { default: "inline", "@container (width < 32px)": "none" } },
  date: { position: "absolute", top: "calc(50% + 12px)", insetInline: 0, textAlign: "center", fontSize: 12,
    display: { default: "block", "@container (height < 84px)": "none" } },
  clockLayer: (alpha: number, y: number, ms: number, easing: string) => ({
    opacity: alpha, transform: `translateY(${y}px)`,
    transitionProperty: "opacity, transform", transitionTimingFunction: easing,
    transitionDuration: { default: `${ms}ms`, "@media (prefers-reduced-motion: reduce)": "0ms" },
  }),
});
