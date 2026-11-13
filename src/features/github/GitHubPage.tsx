import * as stylex from "@stylexjs/stylex";
import { ExternalLink, GitPullRequest, CircleDot } from "lucide-react";
import { useEffect, useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import { Link, useSearchParams } from "react-router";
import { DemoBoundary, Dialog, PageHeader, Panel, ui } from "../../app/workbenchUi";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "../../components/ui/table";
import { ChoiceSelect } from "../../app/ChoiceSelect";
import { Input } from "../../components/ui/input";
import { Button } from "../../components/ui/button";
import { Badge } from "../../components/ui/badge";
import { Empty, EmptyHeader, EmptyTitle } from "../../components/ui/empty";
import { Field, FieldLabel } from "../../components/ui/field";
import { ResourceTabs } from "../../app/ResourceTabs";
import type { RootState } from "../../store";
import { chooseAccount } from "./githubSlice";

const tabs = [["repositories", "仓库"], ["prs", "Pull Requests"], ["issues", "Issues"], ["activity", "活动"]] as const;
type HubTab = typeof tabs[number][0];

export function GitHubPage() {
  const dispatch = useDispatch();
  const [params, setParams] = useSearchParams();
  const { accounts, accountId, repositories, items } = useSelector((state: RootState) => state.github);
  const detail = repositories.find(repo => repo.id === params.get("repo"));
  const activeAccount = detail?.accountId ?? accounts.find(account => account.id === params.get("account"))?.id ?? accountId;
  const tab = tabs.find(([key]) => key === params.get("tab"))?.[0] ?? "repositories";
  const owned = repositories.filter(repo => repo.accountId === activeAccount);
  const repoIds = new Set(owned.map(repo => repo.id));
  const accountItems = items.filter(item => repoIds.has(item.repoId));
  const localRepos = useSelector((state: RootState) => state.git.repositories);
  const local = localRepos.find(repo => repo.githubId === detail?.id);

  useEffect(() => {
    if (accountId !== activeAccount) dispatch(chooseAccount(activeAccount));
  }, [accountId, activeAccount, dispatch]);

  function viewRepo(id: string) {
    const next = new URLSearchParams(params);
    next.set("repo", id);
    setParams(next);
  }

  function closeDetail() {
    const next = new URLSearchParams(params);
    next.delete("repo");
    setParams(next, { replace: true });
  }

  return <>
    <PageHeader title="GitHub" description="按账户查看演示仓库、协作事项与最近活动。" />
    <DemoBoundary permission><div {...stylex.props(ui.stack)}>
      <div {...stylex.props(ui.toolbar)}><Field className="w-auto max-w-full"><FieldLabel htmlFor="github-account">演示账户</FieldLabel>
        <ChoiceSelect id="github-account" value={activeAccount} options={accounts.map(account => ({ value: account.id, label: account.name }))}
          onValueChange={account => { dispatch(chooseAccount(account)); setParams({ account, tab }); }} />
      </Field><span {...stylex.props(ui.muted)}>{owned.length} 个仓库 · {accountItems.filter(item => item.state === "open").length} 个待处理事项</span></div>
      <ResourceTabs label="GitHub 资源" tabs={tabs} current={tab}>
        <HubList key={activeAccount + "/" + tab} tab={tab} owned={owned} items={accountItems} openRepo={viewRepo} />
      </ResourceTabs>
      {detail && <Dialog title={detail.name} onClose={closeDetail} side>
        <div {...stylex.props(ui.stack)}><strong {...stylex.props(ui.mono)}>{detail.owner}/{detail.name}</strong>
          <p>{detail.description}</p><span {...stylex.props(ui.muted)}>{detail.visibility === "Public" ? "公开" : "私有"} · 默认分支 {detail.defaultBranch}</span>
          <span>最近同步（样本） · {detail.syncedAt}</span>
          <span>打开的 PR · {items.filter(item => item.repoId === detail.id && item.kind === "pr" && item.state === "open").length}</span>
          <span>打开的 Issues · {items.filter(item => item.repoId === detail.id && item.kind === "issue" && item.state === "open").length}</span>
          <div className="flex flex-col items-start gap-3">
            <Button variant="outline" nativeButton={false} render={<Link to={local ? "/git?repo=" + local.id : "/git?action=clone&source=" + detail.id} />}>
              {local ? "查看本地 Git 工作区" : "模拟克隆此仓库"}</Button>
            <Button variant="link" nativeButton={false} render={<a href="https://github.com/octocat/Hello-World" target="_blank" rel="noreferrer" />}>
              打开 GitHub 公共示例 <ExternalLink data-icon="inline-end" aria-hidden="true" /></Button>
          </div>
        </div>
      </Dialog>}
    </div></DemoBoundary>
  </>;
}

function HubList({ tab, owned, items, openRepo }: {
  tab: HubTab; owned: RootState["github"]["repositories"]; items: RootState["github"]["items"]; openRepo: (id: string) => void;
}) {
  const [params, setParams] = useSearchParams();
  const search = params.get("q") ?? "";
  function setSearch(value: string) {
    const next = new URLSearchParams(params);
    if (value) next.set("q", value);
    else next.delete("q");
    setParams(next, { replace: true });
  }
  const [filter, setFilter] = useState("all");
  const isRepos = tab === "repositories";
  const matches = (text: string) => text.toLowerCase().includes(search.trim().toLowerCase());
  const repositories = owned.filter(repo => matches(repo.name + " " + repo.owner) && (filter === "all" || repo.visibility === filter));
  const shown = items.filter(item => (tab === "activity" || item.kind === (tab === "prs" ? "pr" : "issue"))
    && (filter === "all" || item.state === filter)
    && matches(item.title + " " + item.repoId + " #" + item.number + " " + item.author))
    .sort((a, b) => b.date.localeCompare(a.date));
  const title = tabs.find(([id]) => id === tab)?.[1] ?? "仓库";
  return <Panel title={title}><div {...stylex.props(ui.stack)}>
    <div {...stylex.props(ui.toolbar)}>
      <Field className="w-auto max-w-full"><FieldLabel className="sr-only" htmlFor="github-search">搜索{title}</FieldLabel>
        <Input id="github-search" type="search" placeholder={"搜索" + title} value={search} onChange={event => setSearch(event.target.value)} />
      </Field>
      <Field className="w-auto max-w-full"><FieldLabel className="sr-only" htmlFor="github-filter">{isRepos ? "仓库可见性" : "事项状态"}</FieldLabel>
        <ChoiceSelect id="github-filter" value={filter} onValueChange={setFilter} options={isRepos
          ? [{ value: "all", label: "全部可见性" }, { value: "Public", label: "公开" }, { value: "Private", label: "私有" }]
          : [{ value: "all", label: "全部状态" }, { value: "open", label: "未关闭" }, { value: "closed", label: "已关闭" }]} />
      </Field>
      {(filter !== "all" || search) && <Button variant="outline" onClick={() => { setSearch(""); setFilter("all"); }}>清空筛选</Button>}
    </div>
    {(isRepos ? repositories.length : shown.length) === 0 ? <Empty><EmptyHeader><EmptyTitle>没有匹配的{title}。</EmptyTitle></EmptyHeader></Empty>
      : isRepos ? <Table>
        <TableHeader><TableRow>
          <TableHead>仓库</TableHead><TableHead>可见性</TableHead><TableHead>默认分支</TableHead><TableHead>待办</TableHead><TableHead>上次同步（样本）</TableHead>
        </TableRow></TableHeader>
        <TableBody>{repositories.map(repo => <TableRow key={repo.id}>
          <TableCell><Button variant="link" size="sm" onClick={() => openRepo(repo.id)}>{repo.owner}/{repo.name}</Button></TableCell>
          <TableCell><Badge variant="outline">{repo.visibility === "Public" ? "公开" : "私有"}</Badge></TableCell><TableCell><code>{repo.defaultBranch}</code></TableCell>
          <TableCell>{items.filter(item => item.repoId === repo.id && item.state === "open").length}</TableCell><TableCell>{repo.syncedAt}</TableCell>
        </TableRow>)}</TableBody>
      </Table>
      : tab === "activity" ? <ol {...stylex.props(ui.stack)}>{shown.map(item =>
        <li key={item.repoId + "/" + item.number}><strong>{item.author}</strong> {item.state === "closed" ? "关闭" : "更新"}
          {item.kind === "pr" ? "合并请求" : "问题"}「{item.title}」
          <div {...stylex.props(ui.row)}><Button variant="link" size="sm" onClick={() => openRepo(item.repoId)}>{item.repoId} #{item.number}</Button>
            <small {...stylex.props(ui.muted)}>{item.date} UTC</small></div>
        </li>)}</ol>
      : <Table><TableHeader><TableRow><TableHead>事项</TableHead><TableHead>仓库</TableHead><TableHead>状态</TableHead><TableHead>更新（UTC）</TableHead></TableRow></TableHeader>
        <TableBody>{shown.map(item => <TableRow key={item.repoId + "/" + item.number}>
          <TableCell><div {...stylex.props(ui.row)}>{item.kind === "pr" ? <GitPullRequest size={15} aria-label="Pull Request" /> : <CircleDot size={15} aria-label="Issue" />}
            <strong>{item.title}</strong></div><small {...stylex.props(ui.muted)}>#{item.number} · {item.author}</small></TableCell>
          <TableCell><Button variant="link" size="sm" onClick={() => openRepo(item.repoId)}>{item.repoId}</Button></TableCell>
          <TableCell><Badge variant="secondary">{item.state === "open" ? "未关闭" : "已关闭"}</Badge></TableCell><TableCell>{item.date}</TableCell>
        </TableRow>)}</TableBody></Table>}
  </div></Panel>;
}
