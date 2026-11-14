import * as stylex from "@stylexjs/stylex";
import { useEffect, useId, useState } from "react";
import type { MediaStatus } from "./islandApi";
import { notchOutline, type PreviewLayout } from "./previewLayout";
import { PreviewMusic } from "./PreviewMusic";
import { BatteryRing, MusicWave } from "./PreviewSymbols";
import { islandPalette, isDefaultDarkSurface, lightSurface } from "./settingsStyles";
import type { AnimationPreset, CustomCurve, IslandConfig } from "./types";

export function motionEasing(preset: AnimationPreset, expanding: boolean, customCurve?: CustomCurve) {
  if (preset === "custom" && customCurve) {
    if (customCurve === "linear") return "linear";
    return customCurve === "easeIn" ? "cubic-bezier(.333333,0,.666667,.333333)" : "cubic-bezier(.333333,.666667,.666667,1)";
  }
  if (preset === "spring") return "cubic-bezier(.22,1.16,.4,1)";
  if (preset === "subtle") return "ease-out";
  return expanding ? "cubic-bezier(.22,1,.36,1)" : "cubic-bezier(.4,0,1,1)";
}

function isLightSurface(hex: string) {
  const channels = [1, 3, 5].map(start => {
    const channel = parseInt(hex.slice(start, start + 2), 16) / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722 > 0.179;
}

function Companion({ x, motion }: { x: number; motion: boolean }) {
  const [cycle, setCycle] = useState(0);
  useEffect(() => {
    if (!motion) return;
    const timer = setInterval(() => setCycle(n => n + 1), 5600);
    return () => clearInterval(timer);
  }, [motion]);
  let hash = BigInt.asUintN(64, BigInt(cycle) * 0x9e3779b97f4a7c15n);
  hash ^= hash >> 31n;
  hash = BigInt.asUintN(64, hash * 0xbf58476d1ce4e5b9n);
  hash ^= hash >> 29n;
  const delay = 2 + Number(hash >> 40n) / 2 ** 24 * 2.7;
  return <g key={cycle} {...stylex.props(motion && s.blink(delay))}>
    <rect x={x - 7.4} y="10.5" width="3.6" height="8" rx="1.8" />
    <rect x={x + 3.8} y="10.5" width="3.6" height="8" rx="1.8" />
  </g>;
}

function Compact({ layout, battery, pet, music, playing, motion, clock }: {
  layout: PreviewLayout; battery?: [number, boolean] | null; pet: boolean; music: boolean; playing: boolean; motion: boolean; clock: string;
}) {
  if (layout.side) {
    const [hour, minute] = clock.split(":");
    return <>
      <g fontSize="13.5" fontWeight="700" textAnchor="middle"><text x="18" y="22">{hour}</text><text x="18" y="36">{minute}</text></g>
      {battery && <BatteryRing x={18} y={49} battery={battery} scale={0.72} />}
      {music && <MusicWave x={18} y={48 + (battery ? 20 : 0)} playing={playing} motion={motion} />}
    </>;
  }
  const used = 44 + (battery ? 32 : 0) + (pet ? 44 : 0) + (music ? 26 : 0);
  const left = (layout.bodyWidth - used) / 2;
  const petX = left + (battery ? 32 : 0);
  const clockX = petX + (pet ? 44 : 0);
  return <>
    {battery && <BatteryRing x={left + 10} y={15} battery={battery} />}
    {pet && <Companion x={petX + 16} motion={motion} />}
    <text x={clockX + 22} y="21" fontWeight="700" fontSize="15.5" textAnchor="middle">{clock}</text>
    {music && <MusicWave x={clockX + 63} y={15} playing={playing} motion={motion} />}
  </>;
}

function Outline({ layout, grow = 0, transform }: { layout: PreviewLayout; grow?: number; transform?: string }) {
  if (!layout.pill) return <path d={notchOutline(layout, grow)} transform={transform} />;
  return <rect x={-grow * .6} y={-grow * .3} width={layout.width + grow * 1.2} height={layout.height + grow * 1.3}
    rx={layout.radius + grow * .8} transform={transform} />;
}

export function PreviewContent({ config, layout, media, sampledAt }: {
  config: IslandConfig; layout: PreviewLayout; media?: MediaStatus; sampledAt: number;
}) {
  const { appearance: a, animation } = config;
  const { width, height, expanded } = layout;
  const motion = animation.enabled && !animation.reducedMotion && animation.performance !== "performance";
  const automatic = a.theme === "system" && isDefaultDarkSurface(a.background);
  const background = automatic ? `light-dark(${lightSurface}, ${a.background})` : a.background;
  const surface = isLightSurface(a.background) ? "light" : "dark";
  const tone = (key: keyof typeof islandPalette.tones) => {
    const pair = islandPalette.tones[key];
    return automatic ? `light-dark(${pair.light}, ${pair.dark})` : pair[surface];
  };
  const border = automatic && a.background === islandPalette.darkSurface && a.border === islandPalette.darkBorder ? background : a.border;
  const id = useId();
  const [now, setNow] = useState(() => new Date());
  const interval = expanded && media?.playing ? 1000 : 60_000;
  useEffect(() => {
    const timer = setTimeout(() => setNow(new Date()), interval - Date.now() % interval);
    return () => clearTimeout(timer);
  }, [now, interval]);
  const clock = now.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false });
  const battery = a.showBattery ? media?.battery : null;
  return <div {...stylex.props(s.shell, s.colors(a.theme, tone("foreground"), tone("battery"), tone("date"), tone("dim"),
    tone("control"), tone("elapsed"), tone("track"), tone("lyricsButton"), tone("lyricsBorder")))}>
    {a.blur && animation.performance !== "performance" && <div {...stylex.props(s.blur(`url(#${id}-blur)`))} />}
    <svg width="100%" height="100%" viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" fill="currentColor" {...stylex.props(s.svg)}>
      <defs>
        <clipPath id={id}><Outline layout={layout} /></clipPath>
        {a.blur && <clipPath id={`${id}-blur`} clipPathUnits="objectBoundingBox">
          <Outline layout={layout} transform={`scale(${1 / width} ${1 / height})`} />
        </clipPath>}
      </defs>
      {a.shadow && expanded && [[10, .035], [6.5, .05], [3.5, .07], [1.5, .09]].map(([grow, opacity]) =>
        <g key={grow} fill={islandPalette.shadow} opacity={opacity}><Outline layout={layout} grow={grow} /></g>)}
      <g fill={background} fillOpacity={a.opacity}><Outline layout={layout} /></g>
      <g clipPath={`url(#${id})`}>
        {!layout.minimal && <g transform={`translate(${layout.bodyX} ${layout.bodyY})`}>
          <g key={expanded ? "music" : "bar"} {...stylex.props(motion && s.reveal(animation.contentMs, animation.amplitude))}>
            {expanded ? <PreviewMusic media={media} battery={battery} sampledAt={sampledAt} now={now} clock={clock} motion={motion}
              lyricsEnabled={a.lyricsEnabled} top={!layout.side} />
              : <Compact layout={layout} battery={battery} pet={a.pet} music={Boolean(media?.title)} playing={Boolean(media?.playing)} motion={motion} clock={clock} />}
          </g>
        </g>}
        {(border !== background || a.opacity < 1) && <g fill="none" stroke={border} strokeWidth="2"><Outline layout={layout} /></g>}
      </g>
    </svg>
  </div>;
}

const blinking = stylex.keyframes({ from: { transform: "scaleY(1)" }, "50%": { transform: "scaleY(.45)" }, to: { transform: "scaleY(1)" } });
const entering = stylex.keyframes({ from: { opacity: 0, transform: "translateY(var(--island-shift))" }, to: { opacity: 1, transform: "translateY(0)" } });
const s = stylex.create({
  shell: { position: "relative", width: "100%", height: "100%", fontFamily: '"Segoe UI", "Microsoft YaHei UI", sans-serif', userSelect: "none" },
  svg: { overflow: "visible", display: "block" },
  colors: (theme: string, color: string, battery: string, date: string, dim: string, control: string, elapsed: string, track: string, button: string, border: string) => ({
    colorScheme: theme === "system" ? "light dark" : theme, color, "--island-battery": battery, "--island-date": date,
    "--island-dim": dim, "--island-control": control, "--island-elapsed": elapsed, "--island-track": track,
    "--island-lyrics-button": button, "--island-lyrics-border": border,
  }),
  blur: (clipPath: string) => ({ position: "absolute", inset: 0, backdropFilter: "blur(12px)", clipPath }),
  blink: (delay: number) => ({ transformBox: "fill-box", transformOrigin: "center", animationName: { default: blinking, "@media (prefers-reduced-motion: reduce)": "none" },
    animationDuration: "180ms", animationDelay: `${delay}s`, animationTimingFunction: "ease-in-out" }),
  reveal: (ms: number, amplitude: number) => ({ "--island-shift": `${4 * amplitude}px`,
    animationName: { default: entering, "@media (prefers-reduced-motion: reduce)": "none" }, animationDuration: `${ms}ms`, animationTimingFunction: "ease-out" }),
});
