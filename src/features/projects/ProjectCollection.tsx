import * as stylex from "@stylexjs/stylex";
import { FolderGit2, Pin } from "lucide-react";
import { useSelector } from "react-redux";
import { Link } from "react-router";
import { ui } from "../../app/workbenchUi";
import { Badge } from "../../components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "../../components/ui/table";
import type { RootState } from "../../store";
import { projectTime, type Project } from "./projectsSlice";

export function ProjectCollection({ projects, grid }: { projects: Project[]; grid: boolean }) {
  const repositories = useSelector((state: RootState) => state.git.repositories);
  const entries = projects.map(project => {
    const repo = repositories.find(item => item.id === project.gitId);
    const branch = repo?.branches.find(item => item.name === repo.currentBranch);
    const name = <Link to={`/projects?id=${project.id}`} {...stylex.props(ui.link, styles.name)}>
      <FolderGit2 className="size-4 shrink-0" aria-hidden="true" />{project.name}
      {project.pinned && <Pin className="size-4 shrink-0" aria-label="已固定到首页" />}
    </Link>;
    const git = branch && repo ? <Link to={`/git?repo=${repo.id}`} {...stylex.props(ui.link, ui.mono)}>
      {branch.name} · {branch.changes.length ? "有变更" : "干净"}</Link> : "未关联";
    return { project, name, git };
  });

  if (grid) return <div {...stylex.props(ui.grid)}>
    {entries.map(({ project, name, git }) => <Card key={project.id} role="article">
      <CardHeader><CardTitle role="heading" aria-level={3}>{name}</CardTitle></CardHeader>
      <CardContent><div {...stylex.props(ui.stack)}>
        <span {...stylex.props(ui.mono, ui.muted, styles.path)}>{project.path}</span>
        <div {...stylex.props(ui.row)}>
          <span>{project.language || "未设置语言"}</span>
          {project.tags.map(tag => <Badge variant="secondary" key={tag}>{tag}</Badge>)}
        </div>
        <div>{git}</div>
        <small {...stylex.props(ui.muted)}>最近打开 · {projectTime(project.lastOpened)}</small>
      </div></CardContent>
    </Card>)}
  </div>;

  return <Table>
    <TableHeader><TableRow>{["项目 / 路径", "语言", "Git 分支 / 状态", "标签", "最近打开"].map(label => <TableHead key={label} scope="col">{label}</TableHead>)}</TableRow></TableHeader>
    <TableBody>{entries.map(({ project, name, git }) => <TableRow key={project.id}>
      <TableCell>{name}<div {...stylex.props(ui.mono, ui.muted, styles.path)}>{project.path}</div></TableCell>
      <TableCell>{project.language || "未设置"}</TableCell><TableCell>{git}</TableCell>
      <TableCell><div {...stylex.props(ui.row)}>{project.tags.map(tag => <Badge variant="secondary" key={tag}>{tag}</Badge>)}</div></TableCell>
      <TableCell className="font-mono">{projectTime(project.lastOpened)}</TableCell>
    </TableRow>)}</TableBody>
  </Table>;
}

const styles = stylex.create({
  name: { display: "inline-flex", alignItems: "center", gap: 7, fontWeight: 600, overflowWrap: "anywhere" },
  path: { fontSize: 12, overflowWrap: "anywhere" },
});
