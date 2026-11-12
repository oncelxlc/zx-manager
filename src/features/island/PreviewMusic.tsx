import * as stylex from "@stylexjs/stylex";
import { FastForward, Music2, Pause, Play, Rewind, Settings, Volume2 } from "lucide-react";
import { useEffect, useId, useMemo, useRef } from "react";
import type { MediaStatus } from "./islandApi";

export function MusicWave({ playing, motion }: { playing: boolean; motion: boolean }) {
  return <span {...stylex.props(s.wave)}>
    {[4, 8, 12, 7, 5].map((bar, i) => <i key={bar} {...stylex.props(s.waveBar(playing ? bar : 3.5, i), playing && motion && s.playing)} />)}
  </span>;
}

export function BatteryRing({ battery }: { battery: [number, boolean] }) {
  const [pct, charging] = battery;
  const color = charging ? "#21eb54" : pct < 20 ? "#f23046" : "currentColor";
  return <svg viewBox="0 0 20 20" width="20" height="20" fill="none">
    <circle cx="10" cy="10" r="8.2" stroke={color} strokeOpacity=".22" strokeWidth="2.6" />
    <circle cx="10" cy="10" r="8.2" stroke={color} strokeWidth="2.6" pathLength="100"
      strokeDasharray={`${Math.max(2, pct)} 100`} transform="rotate(-90 10 10)" />
    {charging ? <path d="m11.6 5-4.8 5.9h3.5l-2 4.1 4.9-5.9H9.7Z" fill={color} />
      : <text x="10" y="12.5" textAnchor="middle" fontWeight="700" fontSize="6.6" fill={color}>{pct}</text>}
  </svg>;
}

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

export function PreviewMusic({ media, sampledAt, now, clock, narrow, motion }: {
  media?: MediaStatus; sampledAt: number; now: Date; clock: string; narrow: boolean; motion: boolean;
}) {
  const has = Boolean(media?.title);
  const duration = media?.durationMs ?? 0;
  const elapsed = media?.playing ? Math.max(0, now.getTime() - sampledAt) : 0;
  const position = Math.min(duration || Infinity, (media?.positionMs ?? 0) + elapsed);
  const date = `${now.getMonth() + 1}月${now.getDate()}日 周${"日一二三四五六"[now.getDay()]}`;
  const cx = narrow ? 70 : 210;
  const coverX = narrow ? 39 : 44, coverY = narrow ? 43 : 13;
  const trackX = narrow ? 12 : 96, trackY = narrow ? 162 : 91, trackWidth = narrow ? 116 : 221;
  const buttonY = narrow ? 204 : 118;
  const gradient = useId();
  const title = media?.title;
  const colors = useMemo(() => title ? coverColors(title) : undefined, [title]);
  const Playback = media?.playing ? Pause : Play;
  return <svg viewBox={narrow ? "0 0 140 226" : "0 0 419 140"} width="100%" height="100%" fill="currentColor">
    <foreignObject x={cx - (narrow ? 59 : 94)} y="5" width={narrow ? 107 : 188} height="26">
      <div {...stylex.props(s.header)}>
        {!narrow && media?.battery && <span {...stylex.props(s.headerBattery)}><BatteryRing battery={media.battery} /></span>}
        <span {...stylex.props(s.date)}>{date}</span><time {...stylex.props(s.time)}>{clock}</time>
      </div>
    </foreignObject>
    {!narrow && has && <foreignObject x="359" y="10" width="16" height="16"><MusicWave playing={Boolean(media?.playing)} motion={motion} /></foreignObject>}
    <Settings x={narrow ? 119 : 386} y="10" width="16" height="16" opacity=".7" />
    {colors && !media?.cover && <defs><linearGradient id={gradient} x2="0" y2="1">
      <stop stopColor={colors[0]} /><stop offset="1" stopColor={colors[1]} />
    </linearGradient></defs>}
    {!media?.cover && <rect x={coverX} y={coverY} width="62" height="62" rx={has ? 10 : 13} fill={has ? `url(#${gradient})` : "#19191b"} />}
    {has && media?.cover ? <foreignObject x={coverX} y={coverY} width="62" height="62"><Cover cover={media.cover} /></foreignObject>
      : <Music2 x={coverX + 18} y={coverY + 18} width="26" height="26" strokeWidth={2.6} color={has ? "#fff" : "#6e6e72"} />}
    <foreignObject x={narrow ? 12 : 121} y={narrow ? 110 : has ? 32 : 27} width={narrow ? 116 : 239} height={has ? 42 : 28}>
      <div {...stylex.props(s.metadata, narrow && s.center)}>
        <strong {...stylex.props(s.title)}>{media?.title || "暂无音乐"}</strong>
        {has && media?.artist && <span {...stylex.props(s.artist)}>{media.artist}</span>}
      </div>
    </foreignObject>
    <g fill="currentColor" opacity={has ? 1 : 0.32} fontSize={narrow ? 11 : 15}>
      <text x={narrow ? 12 : 44} y={narrow ? 183 : 97}>{mmss(position)}</text>
      <text x={narrow ? 128 : 375} y={narrow ? 183 : 97} textAnchor="end">{has && duration === 0 ? "--:--" : `-${mmss(duration - position)}`}</text>
    </g>
    <rect x={trackX} y={trackY} width={trackWidth} height="3.5" rx="1.75" opacity={has && media?.canSeek ? 0.35 : 0.15} />
    {has && duration > 0 && <rect x={trackX} y={trackY} width={trackWidth * position / duration} height="3.5" rx="1.75" opacity={media?.canSeek ? 1 : 0.5} />}
    <text x={narrow ? 15 : 74} y={buttonY + 5} textAnchor="middle" fontSize="14" fontWeight="600" opacity=".37">词</text>
    <Rewind x={(narrow ? 42 : 162) - 8} y={buttonY - 8} width="16" height="16" fill="currentColor" opacity={has && media?.canPrevious ? 1 : 0.37} />
    <Playback x={(narrow ? 70 : 211) - 9} y={buttonY - 9} width="18" height="18" fill="currentColor" opacity={has && media?.canToggle ? 1 : 0.37} />
    <FastForward x={(narrow ? 98 : 258) - 8} y={buttonY - 8} width="16" height="16" fill="currentColor" opacity={has && media?.canNext ? 1 : 0.37} />
    <Volume2 x={(narrow ? 125 : 367) - 8} y={buttonY - 8} width="16" height="16" opacity=".37" />
  </svg>;
}

const pulse = stylex.keyframes({ from: { transform: "scaleY(.4)" }, to: { transform: "scaleY(1)" } });
const s = stylex.create({
  wave: { display: "flex", height: 16, alignItems: "center", gap: 1.5 },
  waveBar: (height: number, i: number) => ({ width: 2, height, backgroundColor: "currentColor", borderRadius: 2, animationDelay: `${i * -81}ms` }),
  playing: { animationName: pulse, animationDuration: "520ms", animationDirection: "alternate", animationIterationCount: "infinite",
    animationPlayState: { default: "running", "@media (prefers-reduced-motion: reduce)": "paused" } },
  art: { width: "100%", height: "100%", objectFit: "cover", borderRadius: 10 },
  header: { display: "flex", width: "100%", height: "100%", alignItems: "center", justifyContent: "center", gap: 8, whiteSpace: "nowrap" },
  headerBattery: { display: "flex", width: 16, height: 16, alignItems: "center", justifyContent: "center" },
  date: { fontSize: 11, opacity: 0.82 },
  time: { fontSize: 12.5, fontVariantNumeric: "tabular-nums", fontWeight: 700 },
  metadata: { height: "100%", display: "flex", flexDirection: "column", justifyContent: "center", gap: 1 },
  center: { textAlign: "center" },
  title: { overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontWeight: 700, fontSize: 14 },
  artist: { overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", opacity: 0.7, fontSize: 11 },
});
