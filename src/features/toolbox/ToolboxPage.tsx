import * as stylex from "@stylexjs/stylex";
import { Binary, Braces, Clock3, Copy, Eraser, History } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useSearchParams } from "react-router";
import { readPreference, writePreference } from "../../app/localPreferences";
import { PageHeader, Panel, ui } from "../../app/workbenchUi";
import { ChoiceSelect } from "../../app/ChoiceSelect";
import { Button } from "../../components/ui/button";
import { Empty, EmptyHeader, EmptyTitle } from "../../components/ui/empty";
import { Field, FieldError, FieldGroup, FieldLabel } from "../../components/ui/field";
import { Input } from "../../components/ui/input";
import { Textarea } from "../../components/ui/textarea";
import { ToggleGroup, ToggleGroupItem } from "../../components/ui/toggle-group";
import { notify } from "../../app/toasts";
import { convertBase64, convertTimestamp, formatJson, type TimeUnit, type TimeZone } from "./conversions";

const tools = [
  { id: "json", title: "JSON 格式化", category: "数据", icon: Braces },
  { id: "base64", title: "Base64 编解码", category: "文本", icon: Binary },
  { id: "timestamp", title: "时间戳转换", category: "数据", icon: Clock3 },
] as const;
type Tool = (typeof tools)[number];
const recentKey = "zx-once.toolbox.recent.v1";

export function ToolboxPage() {
  const [params, setParams] = useSearchParams();
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState("");
  const [recent, setRecent] = useState(() => {
    const value = readPreference(recentKey);
    return Array.isArray(value) ? tools.filter(tool => value.includes(tool.id)).sort((a, b) => value.indexOf(a.id) - value.indexOf(b.id)) : [];
  });
  const selected = tools.find(tool => tool.id === params.get("tool"));
  const visible = tools.filter(tool => (!category || tool.category === category)
    && (tool.title + tool.id).toLowerCase().includes(query.trim().toLowerCase()));

  function remember(tool: Tool) {
    const next = [tool, ...recent.filter(item => item.id !== tool.id)];
    setRecent(next);
    writePreference(recentKey, next.map(item => item.id));
  }
  function open(tool: Tool) {
    remember(tool);
    setParams({ tool: tool.id });
  }
  return <>
    <PageHeader eyebrow="开发辅助" title="工具箱" description="在本地格式化文本、转换编码和时间戳。" />
    <div {...stylex.props(ui.toolbar)}>
      <Input type="search" className="min-w-40 max-w-xs flex-1" aria-label="搜索工具" placeholder="搜索工具" value={query} onChange={event => setQuery(event.target.value)} />
      <ChoiceSelect value={category} aria-label="工具分类" onValueChange={setCategory}
        options={[{ value: "", label: "全部分类" }, { value: "文本", label: "文本" }, { value: "数据", label: "数据" }]} />
    </div>
    {visible.length ? <ToggleGroup aria-label="选择工具" variant="outline" spacing={3}
      className="grid w-full grid-cols-[repeat(auto-fit,minmax(min(100%,15rem),1fr))]" value={selected ? [selected.id] : []}
      onValueChange={([id]) => {
        const tool = visible.find(item => item.id === id);
        if (tool) open(tool);
      }}>
      {visible.map(tool => <ToggleGroupItem key={tool.id} value={tool.id} className="h-auto min-h-14 justify-start gap-2.5 px-4 py-2.5">
        <tool.icon data-icon="inline-start" aria-hidden="true" /><span>{tool.title}</span>
        <small className="text-muted-foreground group-hover/toggle:text-foreground group-aria-pressed/toggle:text-accent-foreground">{tool.category}</small>
      </ToggleGroupItem>)}
    </ToggleGroup> : <Empty role="status"><EmptyHeader><EmptyTitle>没有匹配的工具。</EmptyTitle></EmptyHeader></Empty>}
    {!!recent.length && <div {...stylex.props(ui.row)}>
      <History className="size-4 shrink-0" aria-hidden="true" /><span {...stylex.props(ui.muted)}>最近使用</span>
      {recent.map(tool => <Button variant="outline" key={tool.id} onClick={() => open(tool)}>{tool.title}</Button>)}
    </div>}
    {selected && <ToolEditor key={selected.id} tool={selected} remember={() => remember(selected)} />}
  </>;
}

function ToolEditor({ tool, remember }: { tool: Tool; remember: () => void }) {
  const [input, setInput] = useState("");
  const [output, setOutput] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [reverse, setReverse] = useState(false);
  const [unit, setUnit] = useState<TimeUnit>("seconds");
  const [zone, setZone] = useState<TimeZone>("utc");
  const timestamp = tool.id === "timestamp";
  function invalidate() { setOutput(null); setError(""); }
  function run(event: FormEvent) {
    event.preventDefault();
    try {
      const value = tool.id === "json" ? formatJson(input)
        : tool.id === "base64" ? convertBase64(input, reverse) : convertTimestamp(input, reverse, unit, zone);
      setOutput(value);
      setError("");
      remember();
    } catch (reason) {
      setOutput(null);
      setError(timestamp && reason instanceof Error ? reason.message
        : tool.id === "json" ? "JSON 解析失败，请检查引号、逗号和括号。" : "无法转换，请输入有效的 UTF-8 Base64 文本。");
    }
  }
  const zoneLabel = zone === "utc" ? "UTC" : Intl.DateTimeFormat().resolvedOptions().timeZone;
  return <Panel title={tool.title}>
    <form onSubmit={run}><FieldGroup className="gap-3">
      {tool.id !== "json" && <div {...stylex.props(ui.toolbar)}>
        <ChoiceSelect aria-label="转换方向" value={reverse ? "reverse" : "forward"}
          onValueChange={value => { setReverse(value === "reverse"); invalidate(); }}
          options={[
            { value: "forward", label: timestamp ? "时间戳 → 日期时间" : "UTF-8 → Base64" },
            { value: "reverse", label: timestamp ? "日期时间 → 时间戳" : "Base64 → UTF-8" },
          ]} />
        {timestamp && <>
          <ChoiceSelect<TimeUnit> aria-label="时间戳单位" value={unit} onValueChange={value => { setUnit(value); invalidate(); }}
            options={[{ value: "seconds", label: "秒" }, { value: "milliseconds", label: "毫秒" }]} />
          <ChoiceSelect<TimeZone> aria-label="时区" value={zone} onValueChange={value => { setZone(value); invalidate(); }}
            options={[{ value: "utc", label: "UTC" }, { value: "local", label: "本地时区" }]} />
        </>}
      </div>}
      <div {...stylex.props(ui.grid)}>
        <Field data-invalid={!!error || undefined}>
          <FieldLabel htmlFor="tool-input">输入</FieldLabel>
          <Textarea id="tool-input" value={input} spellCheck={false} rows={7} aria-invalid={!!error} aria-describedby={error ? "tool-error" : undefined}
            placeholder={timestamp ? reverse ? "1970-01-01 00:00:00" : "0" : tool.id === "json" ? '{"a":1}' : "输入要转换的文本"}
            onChange={event => { setInput(event.target.value); invalidate(); }} className="font-mono" />
          {error && <FieldError id="tool-error">{error}</FieldError>}
        </Field>
        <Field>
          <FieldLabel htmlFor="tool-output">输出{timestamp && " · " + zoneLabel}</FieldLabel>
          <Textarea id="tool-output" readOnly value={output ?? ""} rows={7} className="font-mono" />
        </Field>
      </div>
      <div {...stylex.props(ui.toolbar)}>
        <Button variant="default" type="submit">{tool.id === "json" ? "格式化" : "转换"}</Button>
        <Button type="button" variant="outline" disabled={output === null} onClick={async () => {
          if (output === null) return;
          try { await navigator.clipboard.writeText(output); notify("已复制结果"); }
          catch { notify("复制失败，可在输出框中手动复制。", true); }
        }}><Copy data-icon="inline-start" aria-hidden="true" />复制结果</Button>
        <Button type="button" variant="outline" onClick={() => { setInput(""); invalidate(); }}><Eraser data-icon="inline-start" aria-hidden="true" />清空</Button>
      </div>
    </FieldGroup></form>
  </Panel>;
}
