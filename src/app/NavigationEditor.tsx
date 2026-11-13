import { nanoid } from "@reduxjs/toolkit";
import { ArrowDown, ArrowUp, Eye, EyeOff, GripVertical, Plus, RotateCcw, Star } from "lucide-react";
import { useRef, useState, type DragEvent } from "react";
import { useDispatch, useSelector } from "react-redux";
import { Button } from "../components/ui/button";
import { Card, CardContent, CardHeader } from "../components/ui/card";
import { Checkbox } from "../components/ui/checkbox";
import { Field, FieldDescription, FieldLabel } from "../components/ui/field";
import { Input } from "../components/ui/input";
import { ChoiceSelect } from "./ChoiceSelect";
import type { RootState } from "../store";
import { destinations, type NavGroup, type NavId } from "./navigation";
import { addGroup, moveGroup, moveItem, renameGroup, resetNavigation, toggleGroupVisibility, toggleHidden, togglePin } from "./preferencesSlice";
import { Dialog, Panel } from "./workbenchUi";

type Dragged = { type: "group"; id: string } | { type: "item"; id: NavId };

function GroupName({ group }: { group: NavGroup }) {
  const dispatch = useDispatch();
  const [editing, setEditing] = useState<string | null>(null);
  return <Input aria-label={`分组名称 · ${group.title}`} value={editing ?? group.title} maxLength={40}
    onChange={event => setEditing(event.target.value)} onBlur={() => {
      if (editing?.trim()) dispatch(renameGroup({ id: group.id, title: editing }));
      setEditing(null);
    }} onKeyDown={event => {
      if (event.key === "Enter") event.currentTarget.blur();
      if (event.key === "Escape") setEditing(null);
    }} className="min-w-0 max-w-[min(200px,80%)]" />;
}

export function NavigationEditor() {
  const { groups, pins, hidden } = useSelector((state: RootState) => state.preferences);
  const dispatch = useDispatch();
  const [name, setName] = useState("");
  const [reset, setReset] = useState(false);
  const dragged = useRef<Dragged | null>(null);
  const groupOptions = groups.map(group => ({ value: group.id, label: group.title }));
  function drag(event: DragEvent, value: Dragged) {
    dragged.current = value;
    event.dataTransfer.setData("text/plain", value.id);
    event.dataTransfer.effectAllowed = "move";
  }
  function drop(event: DragEvent, group: NavGroup, groupIndex: number, itemIndex?: number) {
    if (!dragged.current) return;
    event.preventDefault();
    event.stopPropagation();
    const source = dragged.current;
    if (source.type === "group") dispatch(moveGroup({ id: source.id, index: groupIndex }));
    else dispatch(moveItem({ id: source.id, groupId: group.id, index: itemIndex }));
    dragged.current = null;
  }
  return <div className="flex min-w-0 flex-col gap-3" onDragEnd={() => { dragged.current = null; }}>
    <Panel title="导航分组" action={<Button variant="outline" onClick={() => setReset(true)}><RotateCcw size={14} /> 恢复默认</Button>}>
      <form className="flex flex-wrap items-center gap-2" onSubmit={event => {
        event.preventDefault();
        if (name.trim()) { dispatch(addGroup({ id: nanoid(), title: name })); setName(""); }
      }}>
        <Field className="w-auto max-w-full"><FieldLabel className="sr-only" htmlFor="new-nav-group">新分组名称</FieldLabel>
          <Input id="new-nav-group" placeholder="新分组名称" value={name} onChange={event => setName(event.target.value)} maxLength={40} /></Field>
        <Button type="submit" disabled={!name.trim()}><Plus size={14} /> 新建分组</Button>
      </form>
    </Panel>
    {groups.map((group, groupIndex) =>
      <Card key={group.id} aria-label={group.title} className="gap-0 py-0"
        onDragOver={event => { if (dragged.current) event.preventDefault(); }}
        onDrop={event => drop(event, group, groupIndex)}>
        <CardHeader className="px-3 py-2"><div className="flex flex-wrap items-center gap-2">
          <span draggable onDragStart={event => drag(event, { type: "group", id: group.id })}
            title="拖动分组" className="cursor-grab text-muted-foreground"><GripVertical size={16} /></span>
          <GroupName group={group} />
          <div className="ml-auto flex flex-wrap items-center gap-1">
            <Button variant="ghost" size="icon-sm" aria-label={`上移分组 · ${group.title}`} title="上移分组" disabled={groupIndex === 0}
              onClick={() => dispatch(moveGroup({ id: group.id, index: groupIndex - 1 }))}><ArrowUp size={15} /></Button>
            <Button variant="ghost" size="icon-sm" aria-label={`下移分组 · ${group.title}`} title="下移分组" disabled={groupIndex === groups.length - 1}
              onClick={() => dispatch(moveGroup({ id: group.id, index: groupIndex + 1 }))}><ArrowDown size={15} /></Button>
            <Field orientation="horizontal" className="w-auto"><Checkbox id={group.id + "-visible"} aria-label="显示分组" checked={!group.hidden}
              onCheckedChange={() => dispatch(toggleGroupVisibility(group.id))} /><FieldLabel htmlFor={group.id + "-visible"}>显示分组</FieldLabel></Field>
          </div>
        </div></CardHeader><CardContent className="px-0">
        {group.items.map((id, itemIndex) => {
          const { label, icon: Icon } = destinations[id];
          return <div key={id} className="flex flex-wrap items-center gap-2 border-t px-3 py-[7px]" onDrop={event => drop(event, group, groupIndex, itemIndex)}>
            <span draggable onDragStart={event => drag(event, { type: "item", id })}
              title="拖动导航项" className="cursor-grab text-muted-foreground"><GripVertical size={14} /></span>
            <span className="inline-flex items-center gap-2"><Icon size={16} aria-hidden="true" />{label}</span>
            <div className="ml-auto flex flex-wrap items-center gap-1">
              <div className="min-w-0 max-w-[130px] [&_[data-slot=select-trigger]]:max-w-full">
                <ChoiceSelect size="sm" aria-label={`${label} 所属分组`} value={group.id} options={groupOptions}
                  onValueChange={groupId => dispatch(moveItem({ id, groupId }))} />
              </div>
              <Button variant="ghost" size="icon-sm" aria-label={`上移 · ${label}`} title="上移" disabled={itemIndex === 0}
                onClick={() => dispatch(moveItem({ id, groupId: group.id, index: itemIndex - 1 }))}><ArrowUp size={15} /></Button>
              <Button variant="ghost" size="icon-sm" aria-label={`下移 · ${label}`} title="下移" disabled={itemIndex === group.items.length - 1}
                onClick={() => dispatch(moveItem({ id, groupId: group.id, index: itemIndex + 1 }))}><ArrowDown size={15} /></Button>
              <Button variant={pins.includes(id) ? "secondary" : "ghost"} size="icon-sm" aria-label={`收藏 · ${label}`} title="固定到收藏"
                aria-pressed={pins.includes(id)} onClick={() => dispatch(togglePin(id))}>
                <Star size={15} fill={pins.includes(id) ? "currentColor" : "none"} /></Button>
              <Button variant="ghost" size="icon-sm" aria-label={`显示 · ${label}`} title={hidden.includes(id) ? "显示" : "隐藏"}
                aria-pressed={!hidden.includes(id)} onClick={() => dispatch(toggleHidden(id))}>
                {hidden.includes(id) ? <EyeOff size={15} /> : <Eye size={15} />}</Button>
            </div>
          </div>;
        })}
        {!group.items.length && <FieldDescription className="px-3 py-1.5">将导航项拖到这里，或在其分组菜单中选择“{group.title}”。</FieldDescription>}
      </CardContent></Card>)}
    {reset && <Dialog title="恢复默认导航？" onClose={() => setReset(false)}>
      <p>分组名称、顺序、收藏和显示状态将恢复为初始设置。</p>
      <div className="flex flex-wrap gap-2"><Button variant="outline" onClick={() => setReset(false)}>取消</Button>
        <Button onClick={() => { dispatch(resetNavigation()); setReset(false); }}>恢复默认</Button></div>
    </Dialog>}
  </div>;
}
