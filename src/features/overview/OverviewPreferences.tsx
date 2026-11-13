import * as stylex from "@stylexjs/stylex";
import { ArrowDown, ArrowUp, RotateCcw } from "lucide-react";
import { Dialog } from "../../app/workbenchUi";
import { Button } from "../../components/ui/button";
import { Checkbox } from "../../components/ui/checkbox";
import { Field, FieldGroup, FieldLabel } from "../../components/ui/field";
import { defaultLayout, widgetNames, type Widget } from "./overviewLayout";

export function OverviewPreferences({ layout, update, close }: { layout: Widget[]; update: (value: Widget[]) => void; close: () => void }) {
  function move(index: number, offset: number) {
    const next = layout.slice();
    const [widget] = next.splice(index, 1);
    next.splice(index + offset, 0, widget);
    update(next);
  }
  return <Dialog title="自定义总览" onClose={close}>
    <FieldGroup className="gap-3">
      {layout.map((item, index) => <div key={item.id} {...stylex.props(s.line)}>
        <Field orientation="horizontal" className="w-auto min-w-0">
          <Checkbox id={"widget-visible-" + item.id} checked={item.visible} onCheckedChange={visible => update(layout.map(widget =>
            widget.id === item.id ? { ...widget, visible } : widget))} />
          <FieldLabel htmlFor={"widget-visible-" + item.id}>{widgetNames[item.id]}</FieldLabel>
        </Field>
        <div className="flex shrink-0 items-center gap-2">
          <Button variant="outline" size="icon-sm" aria-label={"上移" + widgetNames[item.id]} disabled={index === 0} onClick={() => move(index, -1)}><ArrowUp aria-hidden="true" /></Button>
          <Button variant="outline" size="icon-sm" aria-label={"下移" + widgetNames[item.id]} disabled={index === layout.length - 1} onClick={() => move(index, 1)}><ArrowDown aria-hidden="true" /></Button>
        </div>
      </div>)}
      <div {...stylex.props(s.line)}>
        <Button variant="outline" onClick={() => update(defaultLayout)}><RotateCcw data-icon="inline-start" aria-hidden="true" />恢复默认布局</Button>
        <Button onClick={close}>完成</Button>
      </div>
    </FieldGroup>
  </Dialog>;
}

const s = stylex.create({
  line: { display: "flex", alignItems: "center", justifyContent: "space-between", gap: 10 },
});
