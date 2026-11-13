import { readPreference } from "../../app/localPreferences";

export const widgetNames = {
  shortcuts: "快捷操作", recent: "最近项目", environment: "开发环境", git: "Git 动态",
  github: "GitHub 待办", remote: "远程连接", activity: "活动记录",
};
export type WidgetId = keyof typeof widgetNames;
export interface Widget { id: WidgetId; visible: boolean }
export const layoutKey = "zx-once.overview.layout.v1";
export const defaultLayout: Widget[] = (Object.keys(widgetNames) as WidgetId[]).map(id => ({ id, visible: true }));

function isWidget(value: unknown): value is Widget {
  return value !== null && typeof value === "object" && "id" in value && typeof value.id === "string"
    && Object.prototype.hasOwnProperty.call(widgetNames, value.id) && "visible" in value && typeof value.visible === "boolean";
}

export function loadLayout(): Widget[] {
  const saved = readPreference(layoutKey);
  if (!Array.isArray(saved) || !saved.every(isWidget) || new Set(saved.map(item => item.id)).size !== saved.length) return defaultLayout;
  return [...saved, ...defaultLayout.filter(widget => !saved.some(item => item.id === widget.id))];
}
