import * as stylex from "@stylexjs/stylex";
import { ScanLine } from "lucide-react";
import { useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import { Link, useSearchParams } from "react-router";
import { addActivity } from "../../app/activitySlice";
import { DemoBoundary, Dialog, PageHeader, Panel, ui } from "../../app/workbenchUi";
import { Badge } from "../../components/ui/badge";
import { ChoiceSelect } from "../../app/ChoiceSelect";
import { Button } from "../../components/ui/button";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "../../components/ui/collapsible";
import { Empty, EmptyContent, EmptyHeader, EmptyTitle } from "../../components/ui/empty";
import { Input } from "../../components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "../../components/ui/table";
import { ResourceTabs } from "../../app/ResourceTabs";
import { notify } from "../../app/toasts";
import type { RootState } from "../../store";
import { defaultVersionChanged, sampleDetected, type EnvironmentResource, type ResourceCategory } from "./environmentSlice";

const categories: ReadonlyArray<readonly [ResourceCategory, string]> = [
  ["runtime", "运行时"], ["toolchain", "工具链"], ["package", "包管理器"],
];

export function EnvironmentPage() {
  const { items, detectedAt } = useSelector((state: RootState) => state.environment);
  const dispatch = useDispatch();
  const [params, setParams] = useSearchParams();
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState("");
  const id = params.get("id");
  const resource = items.find(item => item.id === id);
  const category = resource?.category ?? categories.find(([id]) => id === params.get("tab"))?.[0] ?? "runtime";
  const shown = items.filter(item => item.category === category &&
    item.name.toLowerCase().includes(query.trim().toLowerCase()) &&
    (!status || (status === "installed") === Boolean(item.defaultVersion)));
  function detect() {
    dispatch(sampleDetected(Date.now()));
    dispatch(addActivity({ text: "已模拟检测开发环境", to: "/environment" }));
    notify(`模拟检测完成，已读取 ${items.length} 个资源的样本。`);
  }
  const close = () => setParams({ tab: category });
  return <>
    <PageHeader title="开发环境" eyebrow="本地开发" description="查看本机运行时、编译工具与包管理器的演示配置。"
      actions={<Button variant="default" onClick={detect}><ScanLine data-icon="inline-start" aria-hidden="true" />模拟检测</Button>} />
    <DemoBoundary>
      <Panel title="环境资源" action={<span {...stylex.props(ui.muted)}>样本检测 · {new Date(detectedAt).toLocaleString("zh-CN", { hour12: false })}</span>}>
        <ResourceTabs label="环境类别" tabs={categories} current={category}>
          <div {...stylex.props(ui.stack)}>
            <div {...stylex.props(ui.toolbar)}>
              <Input type="search" className="min-w-40 max-w-xs flex-1" aria-label="搜索环境资源" placeholder="搜索资源名称" value={query}
                onChange={event => setQuery(event.target.value)} />
              <ChoiceSelect value={status} aria-label="筛选安装状态" onValueChange={setStatus} options={[
                { value: "", label: "全部状态" }, { value: "installed", label: "已安装" }, { value: "missing", label: "未安装" },
              ]} />
            </div>
            {shown.length ? <Table>
              <TableHeader><TableRow>{["资源名称", "默认版本", "安装状态", "样本路径"].map(label => <TableHead scope="col" key={label}>{label}</TableHead>)}</TableRow></TableHeader>
              <TableBody>{shown.map(item => <TableRow key={item.id}>
                <TableCell><Link to={`/environment?id=${item.id}`} {...stylex.props(ui.link)}>{item.name}</Link></TableCell>
                <TableCell className="font-mono">{item.defaultVersion ?? "—"}</TableCell>
                <TableCell><Badge variant="secondary">{item.versions.length ? "已安装" : "未安装"}</Badge></TableCell>
                <TableCell className="font-mono text-muted-foreground">{item.path || "—"}</TableCell>
              </TableRow>)}</TableBody>
            </Table> : <Empty>
              <EmptyHeader><EmptyTitle>此类别下没有匹配的资源。</EmptyTitle></EmptyHeader>
              <EmptyContent><Button variant="outline" onClick={() => { setQuery(""); setStatus(""); }}>清空筛选</Button></EmptyContent>
            </Empty>}
          </div>
        </ResourceTabs>
      </Panel>
    </DemoBoundary>
    {resource ? <ResourceDetails key={resource.id} resource={resource} onClose={close} /> :
      id && <Dialog title="找不到环境资源" onClose={close}><p>演示样本中没有这个资源。</p><Button variant="outline" onClick={close}>返回列表</Button></Dialog>}
  </>;
}

function ResourceDetails({ resource, onClose }: { resource: EnvironmentResource; onClose: () => void }) {
  const dispatch = useDispatch();
  const [version, setVersion] = useState<string | null>(null);
  function switchVersion() {
    if (!version) return;
    dispatch(defaultVersionChanged({ id: resource.id, version }));
    const text = `已将 ${resource.name} 的演示默认版本切换到 ${version}。`;
    dispatch(addActivity({ text, to: `/environment?id=${resource.id}` }));
    notify(text);
    setVersion(null);
  }
  if (version) return <Dialog title="切换默认版本？" onClose={() => setVersion(null)}>
    <p>将在演示环境中使用 {resource.name} {version}。当前默认版本为 {resource.defaultVersion}。</p>
    <div {...stylex.props(ui.row)}>
      <Button variant="outline" onClick={() => setVersion(null)}>取消</Button>
      <Button variant="default" onClick={switchVersion}>确认切换</Button>
    </div>
  </Dialog>;

  return <Dialog title={resource.name} onClose={onClose} side>
    <div {...stylex.props(ui.stack)}>
      <div {...stylex.props(ui.row)}>
        <Badge variant="secondary">{resource.versions.length ? "已安装" : "未安装"}</Badge>
        <span {...stylex.props(ui.muted)}>{categories.find(([id]) => id === resource.category)?.[1]}</span>
      </div>
      {resource.path && <div><strong>样本安装路径</strong><p {...stylex.props(ui.mono)}>{resource.path}</p></div>}
      <div><strong>已安装版本</strong>
        {resource.versions.length ? <ul {...stylex.props(styles.versions)}>
          {resource.versions.map(value => <li key={value} {...stylex.props(styles.version)}>
            <span {...stylex.props(ui.mono)}>{value}</span>
            {value === resource.defaultVersion ? <Badge variant="secondary">当前默认</Badge> :
              <Button variant="outline" aria-label={`将 ${value} 设为默认版本`} onClick={() => setVersion(value)}>设为默认</Button>}
          </li>)}
        </ul> : <p {...stylex.props(ui.muted)}>此样本未安装任何版本。</p>}
      </div>
      <Collapsible>
        <CollapsibleTrigger render={<Button variant="outline" />}>安装说明</CollapsibleTrigger>
        <CollapsibleContent><p>{resource.description}</p></CollapsibleContent>
      </Collapsible>
    </div>
  </Dialog>;
}

const styles = stylex.create({
  versions: { listStyle: "none", padding: 0, margin: 0 },
  version: {
    minHeight: 46, borderBottom: "1px solid var(--border)", display: "flex", alignItems: "center",
    justifyContent: "space-between", gap: 12,
  },
});
