import * as stylex from "@stylexjs/stylex";
import { useEffect, useId, useMemo, useRef } from "react";
import type { MediaStatus } from "./islandApi";
import { BatteryRing, MusicNote, MusicWave, Playback, Skip, Speaker } from "./PreviewSymbols";

function Cover({ cover }: { cover: NonNullable<MediaStatus["cover"]> }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const ctx = ref.current?.getContext("2d");
    if (!ctx || cover.bgra.length !== cover.width * cover.height * 4) return;
    const image = ctx.createImageData(cover.width, cover.height);
    for (let i = 0; i < cover.bgra.length; i += 4) {
      // Windows sends premultiplied BGRA; ImageData takes straight RGBA.
      const alpha = cover.bgra[i + 3];
      const undo = alpha ? 255 / alpha : 0;
      image.data[i] = cover.bgra[i + 2] * undo;
      image.data[i + 1] = cover.bgra[i + 1] * undo;
      image.data[i + 2] = cover.bgra[i] * undo;
      image.data[i + 3] = alpha;
    }
    ctx.putImageData(image, 0, 0);
  }, [cover]);
  return <canvas ref={ref} width={cover.width} height={cover.height} {...stylex.props(s.art)} />;
}

function mmss(ms: number) {
  const sec = Math.floor(Math.max(0, ms) / 1000);
  return `${String(Math.floor(sec / 60)).padStart(2, "0")}:${String(sec % 60).padStart(2, "0")}`;
}

function coverColors(title: string) {
  let hash = 1469598103934665603n;
  for (const byte of new TextEncoder().encode(title)) hash = BigInt.asUintN(64, (hash ^ BigInt(byte)) * 1099511628211n);
  return [["#ff5e62", "#ff9966"], ["#56ccf2", "#2f80ed"], ["#a855f7", "#ec4899"],
    ["#34d399", "#1078c8"], ["#fbbf24", "#ef4444"]][Number(hash % 5n)];
}

export function PreviewMusic({ media, battery, sampledAt, now, clock, motion, lyricsEnabled, top }: {
  media?: MediaStatus; battery?: [number, boolean] | null; sampledAt: number; now: Date; clock: string; motion: boolean; lyricsEnabled: boolean; top: boolean;
}) {
  const has = Boolean(media?.title);
  const lyrics = has && lyricsEnabled;
  const art = lyrics ? 44 : 62, artY = lyrics ? 23 : 13;
  const textX = lyrics ? 99 : 121, textWidth = lyrics ? 123 : 220;
  const trackX = lyrics ? 44 : 96, trackWidth = lyrics ? 176 : 221, trackY = lyrics ? 80.25 : 90.85;
  const duration = media?.durationMs ?? 0;
  const elapsed = media?.playing ? Math.max(0, now.getTime() - sampledAt) : 0;
  const position = Math.min(duration || Infinity, (media?.positionMs ?? 0) + elapsed);
  const date = `${now.getMonth() + 1}月${now.getDate()}日 周${"日一二三四五六"[now.getDay()]}`;
  const gradient = useId();
  const title = media?.title;
  const colors = useMemo(() => title ? coverColors(title) : undefined, [title]);
  const dim = "var(--island-control)";
  return <>
    <foreignObject x={battery ? 6 : 0} y="0" width="419" height="30">
      <div {...stylex.props(s.header)}>
        <i {...stylex.props(s.headerBaseline)} />
        {battery && <span {...stylex.props(s.batterySlot)}><svg viewBox="0 0 24 24" width="24" height="24" {...stylex.props(s.batteryIcon)}>
          <BatteryRing x={8.5} y={12} battery={battery} scale={0.8} />
        </svg></span>}
        <span {...stylex.props(s.date)}>{date}</span><time {...stylex.props(s.clock)}>{clock}</time>
      </div>
    </foreignObject>
    {has && <MusicWave x={367} y={18} playing={Boolean(media?.playing)} motion={motion} />}
    {colors && !media?.cover && <defs><linearGradient id={gradient} x2="0" y2="1">
      <stop stopColor={colors[0]} /><stop offset="1" stopColor={colors[1]} />
    </linearGradient></defs>}
    {has && media?.cover ? <foreignObject x="44" y={artY} width={art} height={art}><Cover cover={media.cover} /></foreignObject> : <>
      <rect x="44" y={artY} width={art} height={art} rx={has ? 10 : 13} fill={has ? `url(#${gradient})` : "#19191b"} />
      <MusicNote x={44 + art / 2} y={artY + art / 2} size={has ? art * 0.39 : 26} color={has ? "#ffffffeb" : "#6e6e72"} />
    </>}
    <foreignObject x={textX} y={lyrics ? 23 : has ? 35 : 29} width={textWidth} height="24">
      <div {...stylex.props(s.line)}><i {...stylex.props(s.textBaseline)} /><strong {...stylex.props(s.title, lyrics && s.smallTitle)}>{title || "暂无音乐"}</strong></div>
    </foreignObject>
    {has && media?.artist && <foreignObject x={textX} y={lyrics ? 39 : 49} width={textWidth} height="24">
      <div {...stylex.props(s.line)}><i {...stylex.props(s.textBaseline)} /><span {...stylex.props(s.artist)}>{media.artist}</span></div>
    </foreignObject>}
    {lyrics && <g><path d="M233 32v66" fill="none" stroke="var(--island-track)" />
      <text x="246" y="61" dominantBaseline="central" fontSize="10" fill="var(--island-dim)">暂无歌词</text></g>}
    <g fill={has ? "currentColor" : "var(--island-elapsed)"} fontSize={lyrics ? 10 : 15}>
      <text x="44" y={lyrics ? 100 : 97}>{mmss(position)}</text>
      <text x={lyrics ? 220 : 375} y={lyrics ? 100 : 97} textAnchor="end">{has && duration === 0 ? "--:--" : `-${mmss(duration - position)}`}</text>
    </g>
    <rect x={trackX} y={trackY} width={trackWidth} height="3.5" rx="1.75" fill={has ? "#5a5a5a" : "var(--island-track)"} />
    {has && duration > 0 && position > 0 && <rect x={trackX} y={trackY} width={Math.max(3.5, trackWidth * position / duration)} height="3.5" rx="1.75" />}
    <text x="74" y="123" fontSize="14" textAnchor="middle" fontWeight="600" fill={has ? lyrics ? "currentColor" : "var(--island-dim)" : dim}>词</text>
    <Skip x={162} direction={-1} color={has && media?.canPrevious ? "currentColor" : dim} />
    <Playback playing={Boolean(media?.playing)} color={has && media?.canToggle ? "currentColor" : dim} />
    <Skip x={258} direction={1} color={has && media?.canNext ? "currentColor" : dim} />
    {has && top && <g><rect x="279" y="108" width="68" height="20" rx="10" fill="var(--island-lyrics-button)" stroke="var(--island-lyrics-border)" />
      <text x="313" y="122" textAnchor="middle" fontSize="10" fontWeight="600" fill="var(--island-dim)">岛上歌词</text></g>}
    <Speaker />
  </>;
}

const s = stylex.create({
  art: { width: "100%", height: "100%", objectFit: "cover", borderRadius: 10 },
  header: { display: "flex", alignItems: "baseline", justifyContent: "center", whiteSpace: "nowrap", lineHeight: 1 },
  headerBaseline: { width: 0, height: 22.5 },
  batterySlot: { width: 24, height: 0, position: "relative" },
  batteryIcon: { position: "absolute", top: -15.9, left: 0 },
  date: { fontSize: 11, color: "var(--island-date)" },
  clock: { marginInlineStart: 8, fontWeight: 700, fontSize: 12.5 },
  line: { display: "flex", alignItems: "baseline", lineHeight: 1 },
  textBaseline: { width: 0, height: 18 },
  title: { overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontWeight: 700, fontSize: 14 },
  smallTitle: { fontSize: 12 },
  artist: { overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: 11.5, color: "var(--island-dim)" },
});
