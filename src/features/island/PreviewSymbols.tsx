import * as stylex from "@stylexjs/stylex";

export function BatteryRing({ battery: [percent, charging], x, y, scale = 1 }: {
  battery: [number, boolean]; x: number; y: number; scale?: number;
}) {
  const color = charging ? "#21eb54" : percent < 20 ? "#f23046" : "var(--island-battery)";
  return <g transform={`translate(${x} ${y}) scale(${scale})`} fill={color}>
    <circle r="8.2" fill="none" stroke={color} strokeWidth="2.6" opacity=".22" />
    <circle r="8.2" fill="none" stroke={color} strokeWidth="2.6" pathLength="100"
      strokeDasharray={percent === 100 ? undefined : `${Math.max(2, percent)} 100`} transform="rotate(-90)" />
    {charging ? <path d="M1.6 -5 -3.2 .9 .3 .9Z M-.3 -.9 3.2 -.9 -1.6 5Z" />
      : <text x="0" y="2.4" textAnchor="middle" fontWeight="700" fontSize="6.6">{percent}</text>}
  </g>;
}

export function MusicWave({ x, y, playing, motion }: { x: number; y: number; playing: boolean; motion: boolean }) {
  return <g transform={`translate(${x} ${y})`}>{[0, 1, 2, 3, 4].map(i => {
    const speed = 5 + i * 0.7;
    const height = playing ? 7.5 + 4 * Math.sin(i * 1.6) : 3.5;
    return <rect key={i} x={-7.8 + i * 3.9} y={-height / 2} width="1.9" height={height} rx=".95"
      {...stylex.props(playing && motion && s.wave(speed, i, height))} />;
  })}</g>;
}

export function MusicNote({ x, y, size, color }: { x: number; y: number; size: number; color: string }) {
  return <g transform={`translate(${x} ${y})`} fill={color}>
    <circle cx={-size * 0.24} cy={size * 0.28} r={size * 0.14} />
    <circle cx={size * 0.26} cy={size * 0.18} r={size * 0.14} />
    <path d={`M${-size * .11} ${size * .28} V${-size * .30} M${size * .39} ${size * .18} V${-size * .42}`}
      fill="none" stroke={color} strokeWidth="1.6" strokeLinecap="round" />
    <path d={`M${-size * .11 - .8} ${-size * .30} L${size * .39 + .8} ${-size * .42} V${-size * .30} L${-size * .11 - .8} ${-size * .18}Z`} />
  </g>;
}

export function Skip({ x, direction, color }: { x: number; direction: number; color: string }) {
  return <g transform={`translate(${x} 118) scale(${direction} 1)`} fill={color}>
    <path d="M-8.9 -6 .3 0 -8.9 6Z M-.3 -6 8.9 0 -.3 6Z" /></g>;
}

export function Playback({ playing, color }: { playing: boolean; color: string }) {
  return <path transform="translate(211 118)" fill={color}
    d={playing ? "M-6.45 -7.5h4.386v15h-4.386Z M2.064 -7.5H6.45v15H2.064Z" : "M-5.418 -7.5 7.482 0 -5.418 7.5Z"} />;
}

export function Speaker() {
  return <g transform="translate(367 118)" fill="var(--island-dim)">
    <rect x="-7" y="-2.8" width="4" height="5.6" rx=".8" /><path d="M-4 -2.8 .5 -6V6L-4 2.8Z" />
    <g stroke="var(--island-dim)" strokeWidth="1.4" fill="none" strokeLinecap="round">
      <path d="M3.33 -3.48a5.4 5.4 0 0 1 0 6.96" /><path d="M5.7 -5.48a8.5 8.5 0 0 1 0 10.96" opacity=".6" />
    </g>
  </g>;
}

const pulse = stylex.keyframes({
  "0%": { transform: "scaleY(var(--wave-middle))" }, "25%": { transform: "scaleY(var(--wave-high))" },
  "50%": { transform: "scaleY(var(--wave-middle))" }, "75%": { transform: "scaleY(var(--wave-low))" },
  "100%": { transform: "scaleY(var(--wave-middle))" },
});
const s = stylex.create({
  wave: (speed: number, i: number, height: number) => ({
    "--wave-middle": String(7.5 / height), "--wave-high": String(11.5 / height), "--wave-low": String(3.5 / height),
    animationName: { default: pulse, "@media (prefers-reduced-motion: reduce)": "none" },
    animationDuration: `${2 * Math.PI / speed}s`, animationDelay: `${-i * 1.6 / speed}s`, animationIterationCount: "infinite",
    animationTimingFunction: "linear", transformBox: "fill-box", transformOrigin: "center",
  }),
});
