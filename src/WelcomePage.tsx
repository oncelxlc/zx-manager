import * as stylex from "@stylexjs/stylex";
import { Cable, ChevronDown, ChevronRight, FolderPlus, GitFork, PanelsTopLeft, Terminal, Wrench, Zap } from "lucide-react";
import { useEffect, useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import { Link, useLocation } from "react-router";
import { writePreference } from "./app/localPreferences";
import { DemoBoundary, PageHeader, Panel, ui } from "./app/workbenchUi";
import { Badge } from "./components/ui/badge";
import { Button } from "./components/ui/button";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "./components/ui/collapsible";
import { Empty, EmptyContent, EmptyHeader, EmptyTitle } from "./components/ui/empty";
import { OverviewPreferences } from "./features/overview/OverviewPreferences";
import { defaultLayout, layoutKey, loadLayout, type Widget, type WidgetId } from "./features/overview/overviewLayout";
import { ActivityFeed, EnvironmentSummary, GitSummary, GithubSummary, RecentProjects, RemoteSummary } from "./features/overview/OverviewResources";
import { toggleDependencies, type RootState } from "./store";

const widgets = {
  shortcuts: QuickActions, recent: RecentProjects, environment: EnvironmentSummary, git: GitSummary,
  github: GithubSummary, remote: RemoteSummary, activity: ActivityFeed,
};
const actions = [
  { to: "/projects?new=1", label: "添加项目", icon: FolderPlus },
  { to: "/git?action=clone", label: "克隆仓库", icon: GitFork },
  { to: "/remote?host=staging&tab=terminal", label: "打开终端", icon: Terminal },
  { to: "/remote?host=build&tab=connections", label: "连接主机", icon: Cable },
  { to: "/toolbox", label: "工具箱", icon: Wrench },
];
const dependencies = [
  "Tauri", "React", "Vite", "React Router", "TypeScript",
  "StyleX", "lucide-react", "shadcn/ui", "Redux Toolkit",
];

export function WelcomePage() {
  const location = useLocation();
  const section = new URLSearchParams(location.search).get("section");
  const target: WidgetId | null = section === "recent" || section === "activity" ? section : null;
  const [layout, setLayout] = useState(loadLayout);
  const [editing, setEditing] = useState(false);
  const visible = layout.filter(item => item.visible || item.id === target);
  function update(value: Widget[]) { setLayout(value); writePreference(layoutKey, value); }
  useEffect(() => {
    if (target) {
      const panel = document.getElementById("widget-" + target);
      panel?.focus({ preventScroll: true });
      panel?.scrollIntoView?.({ block: "start" });
    }
  }, [target, location.key]);
  return <>
    <PageHeader title="工作台总览" description={new Date().toLocaleDateString("zh-CN", { year: "numeric", month: "long", day: "numeric", weekday: "long" })}
      actions={<Button variant="outline" onClick={() => setEditing(true)}><PanelsTopLeft data-icon="inline-start" aria-hidden="true" />自定义总览</Button>} />
    <DemoBoundary key={location.key}>
      <div {...stylex.props(s.grid)}>
        {visible.map(({ id }) => {
          const Component = widgets[id];
          return <div key={id} id={"widget-" + id} tabIndex={-1} {...stylex.props(s.widget, id === "shortcuts" && s.full)}>
            <Component />
          </div>;
        })}
        {!visible.length && <Empty><EmptyHeader><EmptyTitle>未显示任何模块。</EmptyTitle></EmptyHeader>
          <EmptyContent><Button variant="outline" onClick={() => update(defaultLayout)}>恢复默认布局</Button></EmptyContent></Empty>}
      </div>
    </DemoBoundary>
    <Dependencies />
    {editing && <OverviewPreferences layout={layout} update={update} close={() => setEditing(false)} />}
  </>;
}

function QuickActions() {
  return <Panel title={<><Zap className="size-4 shrink-0" aria-hidden="true" />快捷操作</>}>
    <div className="flex flex-nowrap items-center gap-2 overflow-x-auto p-1 -m-1">
      {actions.map(({ to, icon: Icon, label }) => <Button key={to} variant="outline" render={<Link to={to} />} nativeButton={false}>
        <Icon data-icon="inline-start" aria-hidden="true" />{label}
      </Button>)}
    </div>
  </Panel>;
}

function Dependencies() {
  const expanded = useSelector((state: RootState) => state.welcome.dependenciesExpanded);
  const dispatch = useDispatch();
  return <Collapsible open={expanded} onOpenChange={() => dispatch(toggleDependencies())}>
    <section aria-labelledby="dependencies" {...stylex.props(s.dependencies)}>
      <div {...stylex.props(s.depHeader)}>
        <h2 id="dependencies" {...stylex.props(s.depTitle)}>
          <CollapsibleTrigger render={<Button variant="ghost" size="sm" />} aria-controls="dependency-list">
            {expanded ? <ChevronDown data-icon="inline-start" aria-hidden="true" /> : <ChevronRight data-icon="inline-start" aria-hidden="true" />}主要依赖
          </CollapsibleTrigger>
        </h2>
        <Link to="/settings/island" {...stylex.props(ui.link)}>灵动岛设置</Link>
      </div>
      <CollapsibleContent id="dependency-list" keepMounted>
        <ul {...stylex.props(s.list)}>{dependencies.map(name => <li key={name}><Badge variant="outline">{name}</Badge></li>)}</ul>
      </CollapsibleContent>
    </section>
  </Collapsible>;
}

const s = stylex.create({
  grid: { display: "grid", gridTemplateColumns: { default: "minmax(0, 1.15fr) minmax(0, 1fr)", "@media (max-width: 900px)": "minmax(0, 1fr)" },
    alignItems: "start", gap: 16 },
  widget: { minWidth: 0, scrollMarginBlockStart: 12 },
  full: { gridColumn: "1 / -1" },
  dependencies: { borderBlockStart: "1px solid var(--border)", paddingBlock: 10 },
  depHeader: { display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8, fontSize: 13 },
  depTitle: { margin: 0 },
  list: { display: "flex", flexWrap: "wrap", gap: 6, padding: 0, listStyle: "none", margin: "6px 0 0" },
});
