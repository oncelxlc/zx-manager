import * as stylex from "@stylexjs/stylex";
import { ArrowDownToLine, ArrowUpFromLine, ChevronsUpDown, FolderGit2, GitBranch as BranchIcon, GitFork, Plus } from "lucide-react";
import { useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import { Link, useSearchParams } from "react-router";
import { addActivity } from "../../app/activitySlice";
import { DemoBoundary, Dialog, PageHeader, Panel, ui } from "../../app/workbenchUi";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "../../components/ui/table";
import { Badge } from "../../components/ui/badge";
import { ChoiceSelect } from "../../app/ChoiceSelect";
import { Input } from "../../components/ui/input";
import { Button } from "../../components/ui/button";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "../../components/ui/collapsible";
import { Empty, EmptyHeader, EmptyTitle } from "../../components/ui/empty";
import { Field, FieldDescription, FieldGroup, FieldLabel } from "../../components/ui/field";
import { ResourceTabs } from "../../app/ResourceTabs";
import { notify } from "../../app/toasts";
import type { RootState } from "../../store";
import { cloneSample, commitSample, selectBranch, stageSample, syncSample, type GitBranch, type GitRepository } from "./gitSlice";

const tabs = [["changes", "工作区变更"], ["history", "提交历史"], ["branches", "分支"], ["remotes", "远端"]] as const;

const styles = stylex.create({
  workspace: { display: "grid", alignItems: "start", gap: 16,
    gridTemplateColumns: { default: "minmax(164px, 205px) minmax(0, 1fr)", "@media (max-width: 1099px)": "minmax(0, 1fr)" } },
  repoNav: { display: { default: "block", "@media (max-width: 1099px)": "none" } },
  repoSelect: { display: { default: "none", "@media (max-width: 1099px)": "grid" } },
});

export function GitPage() {
  const dispatch = useDispatch();
  const [params, setParams] = useSearchParams();
  const repositories = useSelector((state: RootState) => state.git.repositories);
  const hosted = useSelector((state: RootState) => state.github.repositories);
  const repo = repositories.find(item => item.id === params.get("repo")) ?? repositories[0];
  const available = hosted.filter(item => !repositories.some(local => local.githubId === item.id));
  const [cloneId, setCloneId] = useState(params.get("source") ?? "");
  const selected = available.find(item => item.id === cloneId) ?? available[0];
  const current = repo.branches.find(item => item.name === repo.currentBranch) ?? repo.branches[0];
  const tab = tabs.find(([key]) => key === params.get("tab"))?.[0] ?? "changes";

  function clone() {
    if (!selected) return;
    dispatch(cloneSample(selected));
    setParams({ repo: selected.id });
    const text = "已建立 " + selected.name + " 的演示工作副本";
    notify(text);
    dispatch(addActivity({ text, to: "/git?repo=" + selected.id }));
  }

  return <>
    <PageHeader title="Git 工作区" description="查看本地仓库的分支、工作区变更与提交记录。"
      actions={<Button variant="outline" onClick={() => { const next = new URLSearchParams(params); next.set("action", "clone"); setParams(next); }}>
        <GitFork data-icon="inline-start" aria-hidden="true" /> 克隆仓库</Button>} />
    <DemoBoundary><div {...stylex.props(styles.workspace)}>
      <aside aria-label="本地仓库" {...stylex.props(styles.repoNav)}><Panel title="本地仓库" action={<Badge variant="secondary">{repositories.length}</Badge>}>
        <nav aria-label="选择本地仓库">{repositories.map(item =>
          <Button key={item.id} nativeButton={false} variant={item.id === repo.id ? "secondary" : "ghost"}
            className="h-auto w-full items-start justify-start gap-2 px-2.5 py-2 text-left whitespace-normal"
            render={<Link to={"/git?repo=" + item.id + "&tab=" + tab} aria-current={item.id === repo.id ? "page" : undefined} />}>
            <FolderGit2 data-icon="inline-start" aria-hidden="true" /><span className="min-w-0 break-words">
              <strong>{item.name}</strong><small className="block font-mono">{item.currentBranch}</small>
            </span>
          </Button>)}</nav>
      </Panel></aside>
      <div {...stylex.props(ui.stack)}><div {...stylex.props(ui.toolbar)}>
        <div {...stylex.props(styles.repoSelect)}><Field><FieldLabel htmlFor="git-local-repo">本地仓库</FieldLabel>
          <ChoiceSelect id="git-local-repo" value={repo.id} onValueChange={id => setParams({ repo: id, tab })}
            options={repositories.map(item => ({ value: item.id, label: item.name }))} />
        </Field></div>
        <Field className="w-auto max-w-full"><FieldLabel htmlFor="git-branch">当前分支</FieldLabel>
          <ChoiceSelect id="git-branch" value={current.name} onValueChange={branchName => dispatch(selectBranch({ repoId: repo.id, branchName }))}
            options={repo.branches.map(item => ({ value: item.name, label: item.name }))} />
        </Field>
      </div>
      <ResourceTabs label="Git 仓库视图" tabs={tabs} current={tab}>
        <GitWorkspace key={repo.id + "/" + current.name} repo={repo} branch={current} tab={tab} />
      </ResourceTabs>
      {params.get("action") === "clone" && <Dialog title="克隆演示仓库" onClose={() => {
        const next = new URLSearchParams(params); next.delete("action"); setParams(next, { replace: true });
      }}>
        {selected ? <FieldGroup className="gap-3">
          <Field><FieldLabel htmlFor="git-clone-source">选择尚未克隆的样本仓库</FieldLabel>
            <ChoiceSelect id="git-clone-source" value={selected.id} aria-describedby="git-clone-note" onValueChange={setCloneId}
              options={available.map(item => ({ value: item.id, label: item.owner + "/" + item.name }))} />
            <FieldDescription id="git-clone-note">工作副本仅保存在本次演示会话中。</FieldDescription>
          </Field>
          <Button onClick={clone}>建立演示工作副本</Button>
        </FieldGroup> : <Empty><EmptyHeader><EmptyTitle>所有演示仓库都已有本地工作副本。</EmptyTitle></EmptyHeader></Empty>}
      </Dialog>}
    </div></div></DemoBoundary>
  </>;
}

function GitWorkspace({ repo, branch, tab }: {
  repo: GitRepository; branch: GitBranch; tab: typeof tabs[number][0];
}) {
  const dispatch = useDispatch();
  const [message, setMessage] = useState("");
  const [confirm, setConfirm] = useState(false);
  const target = { repoId: repo.id, branchName: branch.name };
  const hosted = useSelector((state: RootState) => state.github.repositories.find(item => item.id === repo.githubId));

  function report(text: string) {
    notify(text);
    dispatch(addActivity({ text: repo.name + " · " + text, to: "/git?repo=" + repo.id + "&tab=" + tab }));
  }
  function sync(direction: "pull" | "push") {
    dispatch(syncSample({ ...target, direction, at: new Date().toISOString() }));
    report(direction === "pull" ? "已模拟拉取远端更新" : "已模拟推送本地提交");
  }
  function commit() {
    if (!message.trim() || !branch.changes.length) return;
    dispatch(commitSample({ ...target, message, at: new Date().toISOString() }));
    report("已模拟提交「" + message.trim() + "」");
    setMessage("");
    setConfirm(false);
  }

  return <Panel title={repo.name} action={<Badge variant="secondary">
    <BranchIcon size={14} aria-hidden="true" /> {branch.name}</Badge>}>
    <div {...stylex.props(ui.stack)}>
      <div {...stylex.props(ui.row)}>
        <Badge variant={branch.changes.length ? "warning" : "success"}>{branch.changes.length ? "Dirty · 有变更" : "Clean · 干净"}</Badge>
        <span {...stylex.props(ui.mono)}>Ahead {branch.ahead} · Behind {branch.behind}</span>
        <Button variant="outline" disabled={!branch.behind} onClick={() => sync("pull")}><ArrowDownToLine data-icon="inline-start" aria-hidden="true" /> 模拟 Pull</Button>
        <Button variant="outline" disabled={!branch.ahead} onClick={() => sync("push")}><ArrowUpFromLine data-icon="inline-start" aria-hidden="true" /> 模拟 Push</Button>
      </div>
      {tab === "changes" && <>
        {branch.changes.length ? <Table>
          <TableHeader><TableRow><TableHead>状态</TableHead><TableHead>文件路径</TableHead></TableRow></TableHeader><TableBody>{branch.changes.map(file =>
            <TableRow key={file.path}><TableCell>{{ M: "已修改", A: "新增", D: "已删除" }[file.status]}</TableCell>
              <TableCell><code>{file.path}</code></TableCell></TableRow>)}</TableBody></Table>
          : <Empty><EmptyHeader><EmptyTitle>当前分支没有未提交的文件。</EmptyTitle></EmptyHeader></Empty>}
        <div><Button variant="outline" disabled={branch.changes.some(file => file.path === "README.md")}
          onClick={() => { dispatch(stageSample(target)); notify("已模拟修改 README.md"); }}>
          <Plus data-icon="inline-start" aria-hidden="true" /> 添加示例变更</Button></div>
        <form {...stylex.props(ui.toolbar)} onSubmit={event => { event.preventDefault(); if (message.trim() && branch.changes.length) setConfirm(true); }}>
          <Field className="w-auto max-w-full"><FieldLabel htmlFor="git-commit-message">提交说明</FieldLabel>
            <Input id="git-commit-message" value={message} placeholder="简述这次变更" onChange={event => setMessage(event.target.value)} />
          </Field><Button type="submit" disabled={!message.trim() || !branch.changes.length}>模拟提交</Button>
        </form>
        {confirm && <Dialog title="确认模拟提交" onClose={() => setConfirm(false)}>
          <p>将 {branch.changes.length} 个文件变更提交到 {branch.name}。</p><p>{message.trim()}</p>
          <div {...stylex.props(ui.row)}><Button variant="outline" onClick={() => setConfirm(false)}>取消</Button><Button onClick={commit}>确认提交</Button></div>
        </Dialog>}
      </>}
      {tab === "history" && <div {...stylex.props(ui.stack)}>{branch.history.map(change => <Collapsible key={change.id}>
        <CollapsibleTrigger render={<Button variant="ghost" />}>
          <ChevronsUpDown data-icon="inline-start" aria-hidden="true" /><code>{change.id}</code> · {change.message}
        </CollapsibleTrigger>
        <CollapsibleContent>
          <p {...stylex.props(ui.muted)}>{new Date(change.at).toLocaleString()} · {change.files.length} 个文件</p>
          <ul>{change.files.map(file => <li key={file.path}><code>{file.status}　{file.path}</code></li>)}</ul>
        </CollapsibleContent>
      </Collapsible>)}</div>}
      {tab === "branches" && <Table>
        <TableHeader><TableRow><TableHead>分支</TableHead><TableHead>上游</TableHead><TableHead>状态</TableHead></TableRow></TableHeader>
        <TableBody>{repo.branches.map(item => <TableRow key={item.name}>
          <TableCell><code>{item.name}</code></TableCell><TableCell>origin/{item.name}</TableCell>
          <TableCell>{item.name === branch.name ? "当前分支" :
            <Button variant="outline" size="sm" onClick={() => dispatch(selectBranch({ repoId: repo.id, branchName: item.name }))}>
              切换分支
            </Button>}
          </TableCell>
        </TableRow>)}</TableBody>
      </Table>}
      {tab === "remotes" && hosted && <div {...stylex.props(ui.stack)}>
        <strong>origin</strong><code>https://git.example.invalid/{hosted.owner}/{hosted.name}.git</code>
        <div><Button variant="link" nativeButton={false} render={<Link to={"/github?repo=" + hosted.id} />}>查看关联 GitHub 仓库</Button></div>
      </div>}
    </div>
  </Panel>;
}
