import * as stylex from "@stylexjs/stylex";
import { Plus, Terminal } from "lucide-react";
import { useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import { Panel, ui } from "../../app/workbenchUi";
import { Badge } from "../../components/ui/badge";
import { ChoiceSelect } from "../../app/ChoiceSelect";
import { Input } from "../../components/ui/input";
import { Button } from "../../components/ui/button";
import { Field, FieldDescription, FieldLabel } from "../../components/ui/field";
import type { RootState } from "../../store";
import { newSession, sendCommand } from "./remoteSlice";
import type { Server } from "./remoteSamples";

const styles = stylex.create({
  transcript: {
    padding: 16, marginBlock: 0, minHeight: 144, borderRadius: 6, backgroundColor: "var(--surface-secondary)",
    overflowWrap: "anywhere", whiteSpace: "pre-wrap", fontFamily: "var(--mono)", fontSize: 13,
  },
});

export function RemoteTerminal({ host, connected }: { host: Server; connected: boolean }) {
  const sessions = useSelector((state: RootState) => state.remote.sessions).filter(item => item.hostId === host.id);
  const nextId = useSelector((state: RootState) => state.remote.nextSession);
  const dispatch = useDispatch();
  const [id, setId] = useState(sessions[0].id);
  const [command, setCommand] = useState("");
  const active = sessions.find(item => item.id === id) ?? sessions[0];
  return <Panel title="模拟终端" action={<Badge variant="secondary">{connected ? "已模拟连接" : "连接已断开"}</Badge>}>
    <div {...stylex.props(ui.stack)}>
      <div {...stylex.props(ui.toolbar)}>
        <Field className="w-auto max-w-full"><FieldLabel htmlFor="remote-terminal-session">会话</FieldLabel>
          <ChoiceSelect id="remote-terminal-session" value={String(active.id)} onValueChange={value => { setId(Number(value)); setCommand(""); }}
            options={sessions.map(item => ({ value: String(item.id), label: host.name + " · 会话 " + item.id }))} />
        </Field>
        <Button variant="outline" disabled={!connected} onClick={() => { dispatch(newSession(host.id)); setId(nextId); setCommand(""); }}>
          <Plus data-icon="inline-start" aria-hidden="true" />新建演示会话</Button>
      </div>
      <div {...stylex.props(ui.row)}><Terminal size={16} aria-hidden="true" /><code>{host.name} · {host.cwd}</code></div>
      <div role="log" aria-label="终端演示记录" {...stylex.props(styles.transcript)}>
        {active.lines.length ? active.lines.map((line, index) => <div key={index}>
          <strong>$ {line.command}</strong><div>{line.reply}</div>
        </div>) : "输入 help、ls、pwd 或 hostname 查看样本回显。"}
      </div>
      <form {...stylex.props(ui.toolbar)} onSubmit={event => {
        event.preventDefault(); if (!connected || !command.trim()) return;
        dispatch(sendCommand({ sessionId: active.id, command })); setCommand("");
      }}><Field className="w-auto max-w-full" data-disabled={!connected || undefined}>
          <FieldLabel htmlFor="remote-command">演示输入</FieldLabel>
          <Input id="remote-command" value={command} disabled={!connected} autoComplete="off" spellCheck={false}
            aria-describedby={connected ? undefined : "remote-command-hint"} onChange={event => setCommand(event.target.value)} />
        </Field>
        <Button type="submit" disabled={!connected || !command.trim()}>发送</Button>
      </form>
      {!connected && <FieldDescription id="remote-command-hint">在连接页模拟连接此主机后即可发送输入。</FieldDescription>}
    </div>
  </Panel>;
}
