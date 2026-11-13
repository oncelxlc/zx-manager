import { nanoid } from "@reduxjs/toolkit";
import * as stylex from "@stylexjs/stylex";
import { LayoutGrid, List, Plus } from "lucide-react";
import { useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import { useSearchParams } from "react-router";
import { addActivity } from "../../app/activitySlice";
import { DemoBoundary, Dialog, PageHeader, Panel, ui } from "../../app/workbenchUi";
import { ChoiceSelect } from "../../app/ChoiceSelect";
import { Button } from "../../components/ui/button";
import { Empty, EmptyContent, EmptyHeader, EmptyTitle } from "../../components/ui/empty";
import { Input } from "../../components/ui/input";
import { ToggleGroup, ToggleGroupItem } from "../../components/ui/toggle-group";
import { notify } from "../../app/toasts";
import type { RootState } from "../../store";
import { ProjectCollection } from "./ProjectCollection";
import { ProjectDetails } from "./ProjectDetails";
import { ProjectForm, type ProjectFields } from "./ProjectForm";
import { projectAdded, projectEdited, projectOpened, projectPinToggled, projectRemoved } from "./projectsSlice";

export function ProjectsPage() {
  const items = useSelector((state: RootState) => state.projects.items);
  const dispatch = useDispatch();
  const [params, setParams] = useSearchParams();
  const [query, setQuery] = useState("");
  const [tag, setTag] = useState("");
  const [language, setLanguage] = useState("");
  const [sort, setSort] = useState("recent");
  const [grid, setGrid] = useState(false);
  const id = params.get("id");
  const project = items.find(item => item.id === id);
  const adding = params.get("new") === "1";
  const tags = [...new Set([...items.flatMap(item => item.tags), tag].filter(Boolean))].sort();
  const languages = [...new Set([...items.map(item => item.language), language].filter(Boolean))].sort();
  const needle = query.trim().toLowerCase();
  const projects = items.filter(item =>
    (!tag || item.tags.includes(tag)) && (!language || item.language === language) &&
    [item.name, item.path, item.language, ...item.tags].some(field => field.toLowerCase().includes(needle)),
  ).sort((a, b) => sort === "name" ? a.name.localeCompare(b.name, "zh-CN") : b.lastOpened - a.lastOpened);

  function record(text: string, resourceId?: string) {
    dispatch(addActivity({ text, to: resourceId ? `/projects?id=${resourceId}` : "/projects" }));
    notify(text);
  }
  function save(fields: ProjectFields) {
    const target = adding ? nanoid() : project?.id;
    if (!target) return;
    if (adding) dispatch(projectAdded({ ...fields, id: target, pinned: false, lastOpened: 0 }));
    else dispatch(projectEdited({ ...fields, id: target }));
    record(`已${adding ? "添加" : "更新"}演示项目 ${fields.name}`, target);
    if (adding) setParams({ id: target }, { replace: true });
  }
  async function copyPath() {
    if (!project) return;
    try {
      await navigator.clipboard.writeText(project.path);
      notify("已复制项目路径。");
    } catch {
      notify("复制失败，可选中路径手动复制。", true);
    }
  }
  function resetFilters() {
    setQuery(""); setTag(""); setLanguage("");
  }
  function openAdd() {
    setParams({ new: "1" });
  }
  const close = () => setParams({});
  return <>
    <PageHeader title="项目" eyebrow="本地开发" description="管理本地工作区、开发环境与关联仓库。"
      actions={<Button variant="default" onClick={openAdd}><Plus data-icon="inline-start" aria-hidden="true" />新增项目</Button>} />
    <DemoBoundary>
      <Panel title="本地项目" action={<span {...stylex.props(ui.muted)}>{projects.length} / {items.length} 个项目</span>}>
        <div {...stylex.props(ui.stack)}>
          <div {...stylex.props(ui.toolbar)}>
            <Input type="search" className="min-w-40 max-w-xs flex-1" value={query} aria-label="搜索项目" placeholder="搜索名称或路径" onChange={event => setQuery(event.target.value)} />
            <ChoiceSelect value={tag} aria-label="筛选标签" onValueChange={setTag}
              options={[{ value: "", label: "全部标签" }, ...tags.map(value => ({ value, label: value }))]} />
            <ChoiceSelect value={language} aria-label="筛选语言" onValueChange={setLanguage}
              options={[{ value: "", label: "全部语言" }, ...languages.map(value => ({ value, label: value }))]} />
            <ChoiceSelect value={sort} aria-label="项目排序" onValueChange={setSort}
              options={[{ value: "recent", label: "最近打开" }, { value: "name", label: "项目名称" }]} />
            <ToggleGroup aria-label="展示方式" value={[grid ? "grid" : "list"]} onValueChange={([view]) => {
              if (view) setGrid(view === "grid");
            }}>
              <ToggleGroupItem value="list" aria-label="列表视图" title="列表视图"><List aria-hidden="true" /></ToggleGroupItem>
              <ToggleGroupItem value="grid" aria-label="网格视图" title="网格视图"><LayoutGrid aria-hidden="true" /></ToggleGroupItem>
            </ToggleGroup>
          </div>
          {projects.length ? <ProjectCollection projects={projects} grid={grid} /> :
            <Empty><EmptyHeader><EmptyTitle>{items.length ? "没有匹配的项目。" : "还没有演示项目。"}</EmptyTitle></EmptyHeader>
              <EmptyContent><Button variant="outline" onClick={items.length ? resetFilters : openAdd}>{items.length ? "清空筛选" : "添加项目"}</Button></EmptyContent></Empty>}
        </div>
      </Panel>
    </DemoBoundary>
    {adding && <ProjectForm onClose={close} onSave={save} />}
    {!adding && project && <ProjectDetails key={project.id} project={project} onClose={close} onSave={save} onCopy={copyPath}
      onRemove={() => {
        dispatch(projectRemoved(project.id));
        record(`已移除演示项目 ${project.name}`);
        setParams({}, { replace: true });
      }}
      onPin={() => {
        dispatch(projectPinToggled({ id: project.id, at: Date.now() }));
        record(`${project.pinned ? "已取消固定" : "已固定到首页"} · ${project.name}`, project.id);
      }}
      onOpen={() => {
        dispatch(projectOpened({ id: project.id, at: Date.now() }));
        record(`已模拟打开 ${project.name}`, project.id);
      }} />}
    {!adding && id && !project && <Dialog title="找不到项目" onClose={close}>
      <p>此演示项目已移除或未包含在本次会话。</p><Button variant="outline" onClick={close}>返回项目列表</Button>
    </Dialog>}
  </>;
}
