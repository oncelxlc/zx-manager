import * as stylex from "@stylexjs/stylex";
import { createContext, useContext, useId } from "react";
import { ChoiceSelect } from "../../app/ChoiceSelect";
import { Button } from "../../components/ui/button";
import { Checkbox } from "../../components/ui/checkbox";
import { Field, FieldLabel } from "../../components/ui/field";
import { Input } from "../../components/ui/input";
import { Slider } from "../../components/ui/slider";

export const IslandFieldsDisabled = createContext(false);

export function Check({ label, value, change }: { label: string; value: boolean; change: (value: boolean) => void }) {
  const id = useId();
  const disabled = useContext(IslandFieldsDisabled);
  return <Field orientation="horizontal" data-disabled={disabled || undefined} className="min-h-10 justify-between">
    <FieldLabel htmlFor={id}>{label}</FieldLabel>
    <Checkbox id={id} aria-label={label} checked={value} disabled={disabled} onCheckedChange={change} />
  </Field>;
}

export function Amount({ label, value, change, min, max, step = 1 }: {
  label: string; value: number; change: (value: number) => void; min: number; max: number; step?: number | "any";
}) {
  const id = useId();
  return <Field orientation="horizontal" className="min-h-10 justify-between">
    <FieldLabel htmlFor={id}>{label}</FieldLabel>
    <Input id={id} type="number" required value={Number.isFinite(value) ? value : ""}
      min={min} max={max} step={step} className="w-1/2 max-w-56" onChange={event => change(event.currentTarget.valueAsNumber)} />
  </Field>;
}

export function ColorField({ label, value, change }: {
  label: string; value: string; change: (value: string) => void;
}) {
  const id = useId();
  return <Field orientation="horizontal" className="min-h-10 justify-between">
    <FieldLabel htmlFor={id}>{label}</FieldLabel>
    <Input id={id} type="color" value={value} className="w-1/2 max-w-56" onChange={event => change(event.currentTarget.value)} />
  </Field>;
}

export function IslandSlider({ position, value, change }: {
  position?: "horizontal" | "vertical"; value: number; change: (value: number) => void;
}) {
  const min = position ? -100 : 80;
  const max = position ? 100 : 140;
  const reset = position ? 0 : 100;
  const label = position === "vertical" ? "上下位置（%）" : position === "horizontal" ? "左右位置（%）" : "岛体大小（%）";
  const disabled = useContext(IslandFieldsDisabled);
  return <Field orientation="horizontal" data-disabled={disabled || undefined} className="min-h-10 flex-wrap justify-between">
    <span>{label}</span>
    <div {...stylex.props(s.controls)}>
      {position && <Button variant="outline" size="icon-sm" aria-label={position === "vertical" ? "向上微调" : "向左微调"}
        disabled={value === min} onClick={() => change(Math.max(min, value - 5))}>{position === "vertical" ? "↑" : "←"}</Button>}
      <div {...stylex.props(s.slider, position && s.positionSlider)}>
        <Slider aria-label={label} aria-valuetext={value + "%"} value={[value]} disabled={disabled}
          min={min} max={max} step={position ? 1 : 5} onValueChange={([next]) => change(next)} />
      </div>
      {position && <Button variant="outline" size="icon-sm" aria-label={position === "vertical" ? "向下微调" : "向右微调"}
        disabled={value === max} onClick={() => change(Math.min(max, value + 5))}>{position === "vertical" ? "↓" : "→"}</Button>}
      <span>{value}%</span>
      <Button variant="outline" size="sm" disabled={value === reset} onClick={() => change(reset)}>{position ? "恢复居中" : "恢复默认"}</Button>
    </div>
  </Field>;
}

export function Choice<T extends string>({ label, value, options, change }: {
  label: string; value: T; options: ReadonlyArray<readonly [T, string]>; change: (value: T) => void;
}) {
  const id = useId();
  const disabled = useContext(IslandFieldsDisabled);
  return <Field orientation="horizontal" data-disabled={disabled || undefined} className="min-h-10 justify-between">
    <FieldLabel htmlFor={id}>{label}</FieldLabel>
    <ChoiceSelect id={id} aria-label={label} disabled={disabled} value={value} onValueChange={change}
      options={options.map(([value, label]) => ({ value, label }))} />
  </Field>;
}

const s = stylex.create({
  controls: { display: "flex", alignItems: "center", flexWrap: "wrap", gap: 6, maxWidth: "100%" },
  slider: { width: 180, maxWidth: "100%" },
  positionSlider: { width: 150 },
});
