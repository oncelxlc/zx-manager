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

export function TextField({ label, value, change, color = false }: {
  label: string; value: string; change: (value: string) => void; color?: boolean;
}) {
  return <Row label={label}><input type={color ? "color" : "text"} value={value} {...stylex.props(s.input)}
    spellCheck={false} onChange={event => change(event.currentTarget.value)} /></Row>;
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
  row: {
    minHeight: 54, display: "flex", alignItems: "center", justifyContent: "space-between", gap: 12,
    paddingBlock: 7, borderBottom: "1px solid #e7e5e4", fontSize: "0.94rem", color: "#292524",
  },
  input: { minWidth: 0, width: "min(50%, 14rem)", height: 38, paddingInline: 9, borderRadius: 7,
    fontFamily: "inherit", fontSize: "0.94rem", color: "#1c1917", border: "1px solid #a8a29e", backgroundColor: "#fff" },
  check: { width: 22, height: 22, marginInlineEnd: 1, accentColor: "#176957", cursor: "pointer" },
});
