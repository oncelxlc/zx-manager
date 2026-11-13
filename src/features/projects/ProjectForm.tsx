import * as stylex from "@stylexjs/stylex";
import { useState, type FormEvent } from "react";
import { Dialog, ui } from "../../app/workbenchUi";
import { Button } from "../../components/ui/button";
import { Field, FieldError, FieldGroup, FieldLabel } from "../../components/ui/field";
import { Input } from "../../components/ui/input";
import type { Project } from "./projectsSlice";

export type ProjectFields = Pick<Project, "name" | "path" | "language" | "tags" | "ide">;

export function ProjectForm({ project, onClose, onSave }: {
  project?: Project; onClose: () => void; onSave: (fields: ProjectFields) => void;
}) {
  const [name, setName] = useState(project?.name ?? "");
  const [path, setPath] = useState(project?.path ?? "");
  const [language, setLanguage] = useState(project?.language ?? "");
  const [tags, setTags] = useState(project?.tags.join("，") ?? "");
  const [ide, setIde] = useState(project?.ide ?? "");
  const [error, setError] = useState("");

  const invalidName = Boolean(error && !name.trim());
  const invalidPath = Boolean(error && !path.trim());

  function save(event: FormEvent) {
    event.preventDefault();
    if (!name.trim() || !path.trim()) {
      setError("请填写项目名称和本地路径。");
      return;
    }
    onSave({
      name: name.trim(), path: path.trim(), language: language.trim(), ide: ide.trim(),
      tags: [...new Set(tags.split(/[,，]/).map(tag => tag.trim()).filter(Boolean))],
    });
  }

  return <Dialog title={project ? "编辑项目" : "新增演示项目"} onClose={onClose}>
    <form onSubmit={save}><FieldGroup className="gap-3">
      <Field data-invalid={invalidName || undefined}>
        <FieldLabel htmlFor="project-name">项目名称</FieldLabel>
        <Input id="project-name" required value={name} aria-invalid={invalidName} aria-describedby={invalidName ? "project-error" : undefined}
          onChange={event => setName(event.target.value)} />
      </Field>
      <Field data-invalid={invalidPath || undefined}>
        <FieldLabel htmlFor="project-path">本地路径</FieldLabel>
        <Input id="project-path" required value={path} aria-invalid={invalidPath} aria-describedby={invalidPath ? "project-error" : undefined}
          placeholder="D:/Demo/my-project" onChange={event => setPath(event.target.value)} className="font-mono" />
      </Field>
      <Field>
        <FieldLabel htmlFor="project-language">主要语言</FieldLabel>
        <Input id="project-language" value={language} onChange={event => setLanguage(event.target.value)} />
      </Field>
      <Field>
        <FieldLabel htmlFor="project-tags">标签（用逗号分隔）</FieldLabel>
        <Input id="project-tags" value={tags} onChange={event => setTags(event.target.value)} />
      </Field>
      <Field>
        <FieldLabel htmlFor="project-ide">默认编辑器</FieldLabel>
        <Input id="project-ide" value={ide} onChange={event => setIde(event.target.value)} />
      </Field>
      {(invalidName || invalidPath) && <FieldError id="project-error">{error}</FieldError>}
      <div {...stylex.props(ui.row)}>
        <Button type="submit" variant="default">{project ? "保存修改" : "添加项目"}</Button>
        <Button variant="outline" type="button" onClick={onClose}>取消</Button>
      </div>
    </FieldGroup></form>
  </Dialog>;
}
