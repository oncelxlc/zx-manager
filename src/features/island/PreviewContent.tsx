import * as stylex from "@stylexjs/stylex";
import { useEffect, useId, useState } from "react";
import type { MediaStatus } from "./islandApi";
import { BatteryRing, MusicWave, PreviewMusic } from "./PreviewMusic";
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

// The native notch samples the same power curve at fourteen intervals on each shoulder.
function notchPath(width: number, height: number, radius: number, expanded: boolean) {
  const wing = Math.min(expanded ? 17.6 : 5, width / 4);
  const depth = Math.min(expanded ? 16 : 8, height / 2);
  const r = Math.min(radius * (expanded ? 1 : 12.5 / 33), height / 2, width / 2 - wing);
  const shoulder = Array.from({ length: 15 }, (_, i) => {
    const t = i / 14;
    return { x: wing * (1 - (1 - t) ** 2.4), y: depth * t };
  });
  const right = shoulder.map(({ x, y }) => `L${width - x} ${y}`).join(" ");
  const left = shoulder.reverse().map(({ x, y }) => `L${x} ${y}`).join(" ");
  return `M0 0 ${right} V${height - r} a${r} ${r} 0 0 1 ${-r} ${r} H${wing + r} a${r} ${r} 0 0 1 ${-r} ${-r} ${left} Z`;
}

function ink(hex: string) {
  const channels = [1, 3, 5].map(start => {
    const channel = parseInt(hex.slice(start, start + 2), 16) / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722 > 0.179 ? "#111111" : "#ffffff";
}

export function PreviewContent({ config, state, width, height, docked, media, sampledAt }: {
  config: IslandConfig; state: PreviewState; width: number; height: number; docked: boolean;
  media?: MediaStatus; sampledAt: number;
}) {
  const { appearance: a, animation, edge } = config;
  const expanded = state === "expanded";
  const motion = animation.enabled && !animation.reducedMotion && animation.performance !== "performance";
  const automatic = a.theme === "system" && isDefaultDarkSurface(a.background);
  const background = automatic ? `light-dark(${lightSurface}, ${a.background})` : a.background;
  const foreground = automatic ? "light-dark(#181817, #fff)" : ink(a.background);
  const border = automatic && a.background === "#000000" && a.border === "#000000" ? background : a.border;
  const id = useId();
  const path = docked && width > 0 && height > 0 ? notchPath(width, height, a.radius, expanded) : undefined;
  const [now, setNow] = useState(() => new Date());
  const interval = expanded && media?.playing ? 1_000 : 60_000;
  useEffect(() => {
    const timer = setTimeout(() => setNow(new Date()), interval - Date.now() % interval);
    return () => clearTimeout(timer);
  }, [now, interval]);
  const clock = now.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false });
  const vertical = height > width;
  const space = Math.max(width, height) - (vertical ? 20 : Math.min(44, width));
  const pet = !expanded && !vertical && space >= 44;
  const battery = !expanded && media?.battery && space - (pet ? 44 : 0) >= 32 ? media.battery : null;
  const wave = !expanded && media?.title && space - (pet ? 44 : 0) - (battery ? 32 : 0) >= 26;
  if (state === "edgePeek") return <div {...stylex.props(s.stripArea, s.colors(a.theme, foreground))}><span {...stylex.props(s.strip(edge.peekSize))} /></div>;
  return <div {...stylex.props(s.shell, s.colors(a.theme, foreground))}>
    {path && <svg width="0" height="0" {...stylex.props(s.overlay)}>
      <defs><clipPath id={id} clipPathUnits="objectBoundingBox"><path d={path} transform={`scale(${1 / width} ${1 / height})`} /></clipPath></defs>
    </svg>}
    <div {...stylex.props(s.body, s.surface(background, a.opacity, path ? 0 : a.radius, path ? `url(#${id})` : "none"),
      !path && s.border(border), a.shadow && s.shadow, a.blur && animation.performance !== "performance" && s.blur)}>
      <div key={expanded ? "music" : "bar"} {...stylex.props(s.content, motion && s.reveal(animation.contentMs, motionEasing(animation.preset, expanded, animation.customCurve), animation.amplitude))}>
        {expanded && width >= 96 && height >= 56 ? <PreviewMusic media={media} sampledAt={sampledAt} now={now} clock={clock} narrow={width < height * 1.5} motion={motion} />
          : <div {...stylex.props(s.bar)}>
            {battery && <BatteryRing battery={battery} />}
            {pet && <span {...stylex.props(s.pet, motion && s.blink)}>
              <i {...stylex.props(s.eye)} /><i {...stylex.props(s.eye)} />
            </span>}
            <time {...stylex.props(s.clock)}>{clock}</time>
            {wave && <MusicWave playing={media.playing} motion={motion} />}
          </div>}
      </div>
      {path && <svg width="100%" height="100%" viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" {...stylex.props(s.overlay)}>
        <path d={path} fill="none" stroke={border} strokeWidth={2} vectorEffect="non-scaling-stroke" />
      </svg>}
    </div>
  </div>;
}

const blink = stylex.keyframes({ "0%": { transform: "scaleY(1)" }, "96.8%": { transform: "scaleY(.45)" }, "100%": { transform: "scaleY(.45)" } });
const reveal = stylex.keyframes({ from: { opacity: 0, transform: "translateY(var(--preview-shift))" }, to: { opacity: 1, transform: "translateY(0)" } });
const s = stylex.create({
  shell: { position: "relative", width: "100%", height: "100%", userSelect: "none", fontFamily: '"Segoe UI", "Microsoft YaHei UI", sans-serif' },
  colors: (theme: string, color: string) => ({ colorScheme: theme === "system" ? "light dark" : theme, color }),
  body: { position: "absolute", inset: 0, overflow: "hidden", containerType: "size" },
  surface: (base: string, opacity: number, radius: number, clip: string) => ({
    backgroundColor: `color-mix(in srgb, ${base} ${opacity * 100}%, transparent)`, borderRadius: radius, clipPath: clip,
  }),
  border: (color: string) => ({ borderWidth: 1, borderStyle: "solid", borderColor: color }),
  shadow: { boxShadow: "inset 0 0 14px #0002" },
  blur: { backdropFilter: "blur(12px)" },
  overlay: { position: "absolute", inset: 0, pointerEvents: "none" },
  content: { position: "absolute", inset: 0 },
  reveal: (ms: number, easing: string, amplitude: number) => ({ animationName: reveal,
    "--preview-shift": `${amplitude * 4}px`, animationTimingFunction: easing, animationDuration: { default: `${ms}ms`, "@media (prefers-reduced-motion: reduce)": "0ms" } }),
  stripArea: { width: "100%", height: "100%", display: "grid", placeItems: "center", containerType: "size" },
  strip: (size: number) => ({ backgroundColor: "currentColor", borderRadius: 999,
    width: { default: "100%", "@container (aspect-ratio < 1)": `min(100%, ${size}px)` },
    height: { default: `min(100%, ${size}px)`, "@container (aspect-ratio < 1)": "100%" } }),
  bar: { position: "absolute", inset: 0, display: "flex", alignItems: "center", justifyContent: "center", gap: 12,
    flexDirection: { default: "row", "@container (aspect-ratio < 1)": "column" } },
  pet: { width: 32, display: "flex", gap: 7.6, justifyContent: "center", flexShrink: 0 },
  eye: { width: 3.6, height: 8, borderRadius: 2, backgroundColor: "currentColor" },
  blink: { animationName: blink, animationDuration: "5.6s", animationIterationCount: "infinite", animationTimingFunction: "step-end",
    animationPlayState: { default: "running", "@media (prefers-reduced-motion: reduce)": "paused" } },
  clock: { maxWidth: "100%", whiteSpace: "nowrap", fontWeight: 700, fontVariantNumeric: "tabular-nums", lineHeight: 1, textAlign: "center",
    fontSize: { default: 15.5, "@container (28px <= width < 40px)": 11, "@container (width < 28px)": 9 } },
});
