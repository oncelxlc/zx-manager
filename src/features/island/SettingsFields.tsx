import * as stylex from "@stylexjs/stylex";
import type { ReactNode } from "react";

function Row({ label, children }: { label: string; children: ReactNode }) {
  return <label {...stylex.props(s.row)}><span>{label}</span>{children}</label>;
}

export function Check({ label, value, change }: { label: string; value: boolean; change: (value: boolean) => void }) {
  return <Row label={label}><input type="checkbox" checked={value} {...stylex.props(s.check)}
    onChange={event => change(event.currentTarget.checked)} /></Row>;
}

export function Amount({ label, value, change, min, max, step = 1 }: {
  label: string; value: number; change: (value: number) => void; min: number; max: number; step?: number | "any";
}) {
  return <Row label={label}><input type="number" required value={Number.isFinite(value) ? value : ""}
    min={min} max={max} step={step} {...stylex.props(s.input)}
    onChange={event => change(event.currentTarget.valueAsNumber)} /></Row>;
}

export function ColorField({ label, value, change }: {
  label: string; value: string; change: (value: string) => void;
}) {
  return <Row label={label}><input type="color" value={value} {...stylex.props(s.input)}
    onChange={event => change(event.currentTarget.value)} /></Row>;
}

export function IslandSlider({ position, value, change }: {
  position?: "horizontal" | "vertical"; value: number; change: (value: number) => void;
}) {
  const min = position ? -100 : 80;
  const max = position ? 100 : 140;
  const reset = position ? 0 : 100;
  const label = position === "vertical" ? "上下位置（%）" : position === "horizontal" ? "左右位置（%）" : "岛体大小（%）";
  return <div {...stylex.props(s.row, s.flexible)}>
    <span>{label}</span>
    <div {...stylex.props(s.controls)}>
      {position && <button type="button" aria-label={position === "vertical" ? "向上微调" : "向左微调"} disabled={value === min}
        onClick={() => change(Math.max(min, value - 5))}>{position === "vertical" ? "↑" : "←"}</button>}
      <input type="range" aria-label={label} aria-valuetext={`${value}%`} value={value}
        {...stylex.props(s.slider, position && s.positionSlider)}
        min={min} max={max} step={position ? 1 : 5} onChange={event => change(event.currentTarget.valueAsNumber)} />
      {position && <button type="button" aria-label={position === "vertical" ? "向下微调" : "向右微调"} disabled={value === max}
        onClick={() => change(Math.min(max, value + 5))}>{position === "vertical" ? "↓" : "→"}</button>}
      <span>{value}%</span>
      <button type="button" disabled={value === reset} onClick={() => change(reset)}>{position ? "恢复居中" : "恢复默认"}</button>
    </div>
  </div>;
}

export function Choice<T extends string>({ label, value, options, change }: {
  label: string; value: T; options: ReadonlyArray<readonly [T, string]>; change: (value: T) => void;
}) {
  return <Row label={label}><select value={value} {...stylex.props(s.input)} onChange={event => {
    const selected = options.find(([key]) => key === event.currentTarget.value);
    if (selected) change(selected[0]);
  }}>{options.map(([key, title]) => <option key={key} value={key}>{title}</option>)}</select></Row>;
}

const s = stylex.create({
  flexible: { flexWrap: "wrap" },
  controls: { display: "flex", alignItems: "center", flexWrap: "wrap", columnGap: 6, rowGap: 4 },
  slider: { width: 180, maxWidth: "100%", accentColor: "#176957" },
  positionSlider: { width: 150 },
  row: {
    minHeight: 54, display: "flex", alignItems: "center", justifyContent: "space-between", gap: 12,
    paddingBlock: 7, borderBottomWidth: 1, borderBottomStyle: "solid", borderBottomColor: "#e7e5e4",
    fontSize: "0.94rem", color: "#292524",
  },
  input: { minWidth: 0, width: "min(50%, 14rem)", height: 38, paddingInline: 9, borderRadius: 7,
    fontFamily: "inherit", fontSize: "0.94rem", color: "#1c1917", borderWidth: 1, borderStyle: "solid", borderColor: "#a8a29e", backgroundColor: "#fff" },
  check: { width: 22, height: 22, marginInlineEnd: 1, accentColor: "#176957", cursor: "pointer" },
});
