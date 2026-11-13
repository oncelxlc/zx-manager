import * as stylex from "@stylexjs/stylex";
import { Unplug, Cable } from "lucide-react";
import { useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import { useSearchParams } from "react-router";
import { addActivity } from "../../app/activitySlice";
import { DemoBoundary, Dialog, PageHeader, Panel, ui } from "../../app/workbenchUi";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "../../components/ui/table";
import { Badge } from "../../components/ui/badge";
import { ChoiceSelect } from "../../app/ChoiceSelect";
import { Button } from "../../components/ui/button";
import { Field, FieldLabel } from "../../components/ui/field";
import { ResourceTabs } from "../../app/ResourceTabs";
import type { RootState } from "../../store";
import { RemoteFiles } from "./RemoteFiles";
import { RemoteTerminal } from "./RemoteTerminal";
import type { Server } from "./remoteSamples";
import { setConnection } from "./remoteSlice";

const tabs = [["connections", "SSH 连接"], ["servers", "服务器"], ["files", "文件"], ["terminal", "终端"],
  ["docker", "Docker"], ["deployments", "部署"]] as const;

export function RemotePage() {
  const dispatch = useDispatch();
  const [params, setParams] = useSearchParams();
  const { servers, connections } = useSelector((state: RootState) => state.remote);
  const host = servers.find(item => item.id === params.get("host")) ?? servers[0];
  const connection = connections.find(item => item.serverId === host.id);
  const tab = tabs.find(([value]) => value === params.get("tab"))?.[0] ?? "connections";
  const connected = connection?.connected ?? false;

  function toggle() {
    if (!connection) return;
    const at = new Date().toISOString().slice(0, 16).replace("T", " ") + " UTC";
    dispatch(setConnection({ id: connection.id, connected: !connected, at }));
    dispatch(addActivity({ text: (connected ? "模拟断开 " : "模拟连接 ") + host.name, to: "/remote?host=" + host.id }));
  }

  return <>
    <PageHeader title="远程中心" description="按主机查看 SSH 连接、服务器资源与部署记录。" />
    <DemoBoundary permission><div {...stylex.props(ui.stack)}>
      <div {...stylex.props(ui.toolbar)}><Field className="w-auto max-w-full"><FieldLabel htmlFor="remote-host">当前主机</FieldLabel>
        <ChoiceSelect id="remote-host" value={host.id} onValueChange={id => setParams({ host: id, tab })}
          options={servers.map(item => ({ value: item.id, label: item.name }))} />
      </Field><Badge variant="secondary" role="status">{connected ? "已模拟连接" : "已断开"}</Badge>
        <span {...stylex.props(ui.muted)}>{host.os}</span></div>
      <ResourceTabs label="远程主机功能" tabs={tabs} current={tab}>
        {tab === "connections" && connection && <Panel title={connection.alias}
          action={<Button variant={connected ? "outline" : "default"} onClick={toggle}>
            {connected ? <Unplug data-icon="inline-start" aria-hidden="true" /> : <Cable data-icon="inline-start" aria-hidden="true" />}
            {connected ? "模拟断开" : "模拟连接"}</Button>}>
          <Metrics values={[
            ["主机地址", connection.host], ["端口", String(connection.port)], ["用户", connection.user],
            ["认证方式", connection.authType], ["上次连接", connection.lastConnected], ["关联服务器", host.name],
          ]} />
        </Panel>}
        {tab === "servers" && <Panel title={host.name} action={<Badge variant="outline">样本指标</Badge>}>
          <Metrics values={[
            ["CPU", host.cpu], ["内存", host.memory], ["磁盘", host.disk], ["运行时间", host.uptime],
            ["操作系统", host.os], ["采样时间", host.sampledAt],
          ]} />
        </Panel>}
        {tab === "files" && <RemoteFiles key={host.id} host={host} connected={connected} />}
        {tab === "terminal" && <RemoteTerminal key={host.id} host={host} connected={connected} />}
        {tab === "docker" && <Docker key={host.id} host={host} />}
        {tab === "deployments" && <Deployments key={host.id} host={host} />}
      </ResourceTabs>
    </div></DemoBoundary>
  </>;
}

const styles = stylex.create({
  metrics: { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(min(180px, 100%), 1fr))", gap: 16, margin: 0 },
  value: { margin: 0, marginBlockStart: 4, fontWeight: 600, overflowWrap: "anywhere" },
});

function Metrics({ values }: { values: [string, string][] }) {
  return <dl {...stylex.props(styles.metrics)}>{values.map(([label, value]) =>
    <div key={label}><dt {...stylex.props(ui.muted)}>{label}</dt><dd {...stylex.props(styles.value, ui.mono)}>{value}</dd></div>)}</dl>;
}

function Docker({ host }: { host: Server }) {
  const [filter, setFilter] = useState("all");
  const resources = host.docker.filter(item => filter === "all" || item.type === filter);
  return <Panel title="Docker 资源" action={<Field className="w-auto"><FieldLabel className="sr-only" htmlFor="docker-resource-type">Docker 资源类型</FieldLabel>
    <ChoiceSelect id="docker-resource-type" value={filter} onValueChange={setFilter} options={[
      { value: "all", label: "全部类型" }, { value: "Containers", label: "容器" }, { value: "Images", label: "镜像" },
      { value: "Volumes", label: "存储卷" }, { value: "Networks", label: "网络" },
    ]} /></Field>}>
    <Table><TableHeader><TableRow>
      <TableHead>名称</TableHead><TableHead>类型</TableHead><TableHead>镜像</TableHead><TableHead>状态</TableHead><TableHead>端口</TableHead><TableHead>创建日期</TableHead>
    </TableRow></TableHeader><TableBody>{resources.map(item => <TableRow key={item.type + "/" + item.name}>
      <TableCell><code>{item.name}</code></TableCell><TableCell>{item.type}</TableCell><TableCell>{item.image}</TableCell>
      <TableCell><Badge variant="secondary">{item.status}</Badge></TableCell><TableCell>{item.ports}</TableCell><TableCell>{item.created}</TableCell>
    </TableRow>)}</TableBody></Table>
  </Panel>;
}

function Deployments({ host }: { host: Server }) {
  const [version, setVersion] = useState("");
  const detail = host.deployments.find(item => item.version === version);
  return <Panel title="部署历史" action={<span {...stylex.props(ui.muted)}>{host.name}</span>}>
    <Table><TableHeader><TableRow>
      <TableHead>版本</TableHead><TableHead>提交</TableHead><TableHead>状态</TableHead><TableHead>部署时间</TableHead><TableHead>日志</TableHead>
    </TableRow></TableHeader><TableBody>{host.deployments.map(item => <TableRow key={item.version}>
      <TableCell>{item.version}</TableCell><TableCell><code>{item.commit}</code></TableCell>
      <TableCell><Badge variant={item.status === "失败" ? "destructive" : "secondary"}>{item.status}</Badge></TableCell><TableCell>{item.at}</TableCell>
      <TableCell><Button variant="outline" size="sm" aria-label={"查看 " + item.version + " 日志"} onClick={() => setVersion(item.version)}>查看日志</Button></TableCell>
    </TableRow>)}</TableBody></Table>
    {detail && <Dialog title={"部署日志 · " + detail.version} onClose={() => setVersion("")} side>
      <div {...stylex.props(ui.stack)}><strong>{host.name} · {detail.status}</strong>
        <span {...stylex.props(ui.muted)}>{detail.at} · {detail.commit}</span>
        <ol>{detail.log.map(line => <li key={line}>{line}</li>)}</ol>
      </div>
    </Dialog>}
  </Panel>;
}
