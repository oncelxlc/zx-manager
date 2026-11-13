import * as stylex from "@stylexjs/stylex";
import { useState } from "react";
import { useSelector } from "react-redux";
import { Link } from "react-router";
import { Dialog, ui } from "../../app/workbenchUi";
import { Button } from "../../components/ui/button";
import type { RootState } from "../../store";
import { ProjectForm, type ProjectFields } from "./ProjectForm";
import { projectTime, type Project } from "./projectsSlice";

export function ProjectDetails({ project, onClose, onSave, onRemove, onPin, onOpen, onCopy }: {
  project: Project; onClose: () => void; onSave: (fields: ProjectFields) => void; onRemove: () => void; onPin: () => void;
  onOpen: () => void; onCopy: () => void;
}) {
  const environment = useSelector((state: RootState) => state.environment.items.find(item => item.id === project.environmentId));
  const git = useSelector((state: RootState) => state.git.repositories.find(item => item.id === project.gitId));
  const github = useSelector((state: RootState) => state.github.repositories.find(item => item.id === project.githubId));
  const server = useSelector((state: RootState) => state.remote.servers.find(item => item.id === project.serverId));
  const activity = useSelector((state: RootState) => state.activity.items);
  const recent = activity.filter(item => item.to === `/projects?id=${project.id}`).slice(0, 5);
  const [action, setAction] = useState<"edit" | "remove" | null>(null);
  if (action === "edit") return <ProjectForm project={project} onClose={() => setAction(null)} onSave={fields => {
    onSave(fields);
    setAction(null);
  }} />;
  if (action === "remove") return <Dialog title="从工作台移除项目？" onClose={() => setAction(null)}>
    <p>将移除“{project.name}”的演示记录，本地文件将保留。</p>
    <div {...stylex.props(ui.row)}>
      <Button variant="outline" onClick={() => setAction(null)}>取消</Button>
      <Button variant="destructive" onClick={onRemove}>确认移除</Button>
    </div>
  </Dialog>;
  return <Dialog title={project.name} onClose={onClose} side>
    <div {...stylex.props(ui.stack)}>
      <div {...stylex.props(ui.row)}>
        <Button variant="default" onClick={onOpen}>模拟打开</Button>
        <Button variant="outline" onClick={onPin} aria-pressed={project.pinned}>{project.pinned ? "取消首页固定" : "固定到首页"}</Button>
      </div>
      <dl {...stylex.props(styles.details)}>
        <dt>本地路径</dt><dd><div {...stylex.props(ui.mono)}>{project.path}</div><Button variant="outline" onClick={onCopy}>复制路径</Button></dd>
        <dt>主要语言</dt><dd>{project.language || "未设置"}</dd>
        <dt>标签</dt><dd>{project.tags.join(" · ") || "无"}</dd>
        <dt>默认编辑器</dt><dd>{project.ide || "未设置"}</dd>
        <dt>最近打开</dt><dd>{projectTime(project.lastOpened)}</dd>
        <dt>Git 工作区</dt><dd>{git ? <Link {...stylex.props(ui.link)} to={`/git?repo=${git.id}`}>{git.name} · {git.currentBranch}</Link> : "未关联"}</dd>
        <dt>GitHub 仓库</dt><dd>{github ? <Link {...stylex.props(ui.link)} to={`/github?repo=${github.id}`}>{github.owner}/{github.name}</Link> : "未关联"}</dd>
        <dt>开发环境</dt><dd>{environment ? <Link {...stylex.props(ui.link)} to={`/environment?id=${environment.id}`}>{environment.name} {environment.defaultVersion}</Link> : "未关联"}</dd>
        <dt>远程主机</dt><dd>{server ? <Link {...stylex.props(ui.link)} to={`/remote?host=${server.id}`}>{server.name}</Link> : "未关联"}</dd>
      </dl>
      <h3 {...stylex.props(styles.heading)}>最近活动</h3>
      {recent.length ? <ul {...stylex.props(styles.activity)}>{recent.map(item =>
        <li key={item.id}>{item.text} <small {...stylex.props(ui.muted)}>{projectTime(item.at)}</small></li>)}</ul> :
        <span {...stylex.props(ui.muted)}>本次会话暂无项目活动。</span>}
      <div {...stylex.props(ui.row)}>
        <Button variant="outline" onClick={() => setAction("edit")}>编辑项目</Button>
        <Button variant="destructive" onClick={() => setAction("remove")}>从工作台移除</Button>
      </div>
    </div>
  </Dialog>;
}

const styles = stylex.create({
  details: {
    display: "grid", gridTemplateColumns: "max-content minmax(0, 1fr)", gap: "12px 16px",
    alignItems: "baseline", margin: 0, overflowWrap: "anywhere",
  },
  heading: { margin: 0, fontSize: 15 },
  activity: { margin: 0, paddingInlineStart: 18 },
});
