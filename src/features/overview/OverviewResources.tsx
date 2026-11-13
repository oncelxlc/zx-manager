import * as stylex from "@stylexjs/stylex";
import { Activity, ArrowRight, Blocks, Cable, ChevronRight, CircleDot, FolderGit2, GitBranch, GitPullRequest, Pin } from "lucide-react";
import type { ReactNode } from "react";
import { useSelector } from "react-redux";
import { Link, useSearchParams } from "react-router";
import { Panel, ui } from "../../app/workbenchUi";
import { Badge } from "../../components/ui/badge";
import { Empty, EmptyContent, EmptyHeader, EmptyTitle } from "../../components/ui/empty";
import type { RootState } from "../../store";
import { projectTime } from "../projects/projectsSlice";

function ViewAll({ to, name }: { to: string; name: string }) {
  return <Link to={to} aria-label={`查看全部${name}`} {...stylex.props(ui.link, s.more)}>
    查看全部<ChevronRight className="size-4 shrink-0" aria-hidden="true" />
  </Link>;
}

function ResourceLine({ to, title, detail, status, variant = "secondary", icon }: {
  to: string; title: string; detail?: string; status?: string; variant?: "secondary" | "outline" | "default"; icon?: ReactNode;
}) {
  return <li><Link to={to} {...stylex.props(s.line)}>
    <div {...stylex.props(s.lead)}>
      <span {...stylex.props(s.name)}>{icon}{title}</span>
      {detail && <small {...stylex.props(s.detail)}>{detail}</small>}
    </div>
    {status && <Badge variant={variant}>{status}</Badge>}
  </Link></li>;
}

function ResourceEmpty({ title, to, action }: { title: string; to: string; action: string }) {
  return <Empty>
    <EmptyHeader><EmptyTitle>{title}</EmptyTitle></EmptyHeader>
    <EmptyContent><Link to={to} {...stylex.props(ui.link)}>{action}</Link></EmptyContent>
  </Empty>;
}

export function RecentProjects() {
  const projects = useSelector((state: RootState) => state.projects.items);
  const repos = useSelector((state: RootState) => state.git.repositories);
  const recent = projects.filter(project => project.pinned || project.lastOpened > 0)
    .sort((a, b) => (b.pinnedAt ?? 0) - (a.pinnedAt ?? 0) || b.lastOpened - a.lastOpened).slice(0, 5);
  return <Panel id="recent" title={<><FolderGit2 className="size-4 shrink-0" aria-hidden="true" />最近项目</>} action={<ViewAll name="项目" to="/projects" />}>
    {recent.length ? <ul {...stylex.props(s.list)}>
      {recent.map(project => {
        const repo = repos.find(item => item.id === project.gitId);
        const branch = repo?.branches.find(item => item.name === repo.currentBranch);
        return <li key={project.id} {...stylex.props(s.line)}>
          <Link to={`/projects?id=${project.id}`} {...stylex.props(s.lead, s.projectName)}>
            <span {...stylex.props(s.name)}>{project.pinned && <Pin className="size-4 shrink-0" aria-label="已固定" />}{project.name}</span>
            <small {...stylex.props(s.detail, ui.mono)}>{project.path}</small>
          </Link>
          {repo && branch && <Link to={`/git?repo=${repo.id}`} {...stylex.props(ui.link, s.gitState)}>
            <span {...stylex.props(ui.mono, s.more)}><GitBranch className="size-4 shrink-0" aria-hidden="true" />{branch.name}</span>
            <Badge variant={branch.changes.length ? "default" : "secondary"}>{branch.changes.length ? "有变更" : "干净"}</Badge>
          </Link>}
        </li>;
      })}
    </ul> : <ResourceEmpty title="暂无最近打开或固定的项目。" to="/projects" action="管理项目" />}
  </Panel>;
}

export function EnvironmentSummary() {
  const resources = useSelector((state: RootState) => state.environment.items);
  const runtimes = resources.filter(item => item.category === "runtime");
  return <Panel title={<><Blocks className="size-4 shrink-0" aria-hidden="true" />开发环境</>} action={<ViewAll name="环境" to="/environment" />}>
    {runtimes.length ? <ul {...stylex.props(s.list)}>{runtimes.map(item =>
      <ResourceLine key={item.id} to={`/environment?id=${item.id}`} title={item.name}
        status={item.defaultVersion ? `v${item.defaultVersion}` : "未安装"} variant={item.defaultVersion ? "secondary" : "outline"} />)}</ul> :
      <ResourceEmpty title="暂无运行时。" to="/environment" action="查看环境资源" />}
  </Panel>;
}

export function GitSummary() {
  const repos = useSelector((state: RootState) => state.git.repositories);
  return <Panel title={<><GitBranch className="size-4 shrink-0" aria-hidden="true" />Git 动态</>} action={<ViewAll name="Git 工作区" to="/git" />}>
    {repos.length ? <ul {...stylex.props(s.list)}>{repos.slice(0, 5).map(repo => {
      const branch = repo.branches.find(item => item.name === repo.currentBranch);
      const status = [];
      if (branch?.changes.length) status.push(`${branch.changes.length} 个变更`);
      if (branch?.ahead) status.push(`领先 ${branch.ahead}`);
      if (branch?.behind) status.push(`落后 ${branch.behind}`);
      return <ResourceLine key={repo.id} to={`/git?repo=${repo.id}&tab=${branch?.changes.length ? "changes" : "history"}`}
        title={repo.name} detail={`${repo.currentBranch} · ${branch?.history[0]?.message ?? "无提交记录"}`}
        status={status.join(" · ") || "已同步"} variant={status.length ? "default" : "secondary"} />;
    })}</ul> : <ResourceEmpty title="暂无本地仓库。" to="/git?action=clone" action="克隆演示仓库" />}
  </Panel>;
}

export function GithubSummary() {
  const { accountId, repositories, items } = useSelector((state: RootState) => state.github);
  const open = items.filter(item => item.state === "open" && repositories.some(repo => repo.id === item.repoId && repo.accountId === accountId));
  return <Panel title={<><GitPullRequest className="size-4 shrink-0" aria-hidden="true" />GitHub 待办</>} action={<ViewAll name="GitHub 活动" to="/github?tab=activity" />}>
    {open.length ? <ul {...stylex.props(s.list)}>{open.slice(0, 5).map(item =>
      <ResourceLine key={item.repoId + "/" + item.number} to={`/github?tab=${item.kind === "pr" ? "prs" : "issues"}&q=%23${item.number}&account=${accountId}`}
        title={item.title} detail={`${item.repoId} #${item.number}`} status={item.kind === "pr" ? "PR" : "Issue"}
        icon={item.kind === "pr" ? <GitPullRequest className="size-4 shrink-0" aria-hidden="true" /> : <CircleDot className="size-4 shrink-0" aria-hidden="true" />} />)}</ul> :
      <ResourceEmpty title="当前账户没有待办。" to="/github?tab=activity" action="查看协作活动" />}
  </Panel>;
}

export function RemoteSummary() {
  const { connections, servers } = useSelector((state: RootState) => state.remote);
  return <Panel title={<><Cable className="size-4 shrink-0" aria-hidden="true" />远程连接</>} action={<ViewAll name="远程连接" to="/remote?tab=connections" />}>
    {connections.length ? <ul {...stylex.props(s.list)}>{connections.map(connection => {
      const server = servers.find(item => item.id === connection.serverId);
      return <ResourceLine key={connection.id} to={`/remote?host=${connection.serverId}&tab=connections`} title={connection.alias}
        detail={`${server?.name ?? connection.host} · ${server?.os ?? connection.authType}`} status={connection.connected ? "已连接" : "未连接"}
        variant={connection.connected ? "secondary" : "outline"} />;
    })}</ul> : <ResourceEmpty title="暂无连接。" to="/remote" action="打开远程中心" />}
  </Panel>;
}

export function ActivityFeed() {
  const [params] = useSearchParams();
  const items = useSelector((state: RootState) => state.activity.items);
  const projects = useSelector((state: RootState) => state.projects.items);
  const all = params.get("section") === "activity";
  return <Panel id="activity" title={<><Activity className="size-4 shrink-0" aria-hidden="true" />活动记录</>}
    action={all ? <span {...stylex.props(ui.muted)}>{items.length} 条记录</span> : <ViewAll name="活动记录" to="/?section=activity" />}>
    {items.length ? <ul {...stylex.props(s.list)}>{(all ? items : items.slice(0, 5)).map(item => {
      const removed = item.to.startsWith("/projects?id=") && !projects.some(project => item.to === `/projects?id=${project.id}`);
      return <ResourceLine key={item.id} to={removed ? "/projects" : item.to} title={item.text} status={projectTime(item.at)}
        icon={<ArrowRight className="size-4 shrink-0" aria-hidden="true" />} />;
    })}</ul> : <ResourceEmpty title="本次会话暂无活动。" to="/projects" action="打开项目" />}
  </Panel>;
}

const s = stylex.create({
  list: { listStyle: "none", padding: 0, margin: 0 },
  more: { display: "inline-flex", alignItems: "center", gap: 2, fontSize: 12 },
  line: {
    display: "grid", gridTemplateColumns: "minmax(0, 1fr) auto", alignItems: "center", gap: 12, paddingBlock: 8,
    textDecoration: "none", color: { default: "var(--foreground)", ":hover": "var(--primary)" },
  },
  lead: { display: "grid", gap: 3, minWidth: 0, overflowWrap: "anywhere" },
  projectName: { color: { default: "var(--foreground)", ":hover": "var(--primary)" }, textDecoration: "none" },
  gitState: { display: "grid", justifyItems: "end", gap: 3, minWidth: 0 },
  name: { display: "flex", alignItems: "center", gap: 6, fontWeight: 580, fontSize: 14 },
  detail: { color: "var(--muted-foreground)", fontSize: 12 },
});
