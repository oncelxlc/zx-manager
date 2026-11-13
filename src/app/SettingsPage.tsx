import { Check, ChevronRight, ListTree, Monitor, Moon, Palette, PanelsTopLeft, Sun } from "lucide-react";
import { useDispatch, useSelector } from "react-redux";
import { Link, useLocation } from "react-router";
import { Button } from "../components/ui/button";
import { Item, ItemActions, ItemContent, ItemDescription, ItemGroup, ItemMedia, ItemTitle } from "../components/ui/item";
import { ToggleGroup, ToggleGroupItem } from "../components/ui/toggle-group";
import type { RootState } from "../store";
import { NavigationEditor } from "./NavigationEditor";
import type { Theme } from "./navigation";
import { setTheme } from "./preferencesSlice";
import { PageHeader, Panel } from "./workbenchUi";

const sections = [
  { to: "/settings/appearance", title: "外观", summary: "选择工作台的浅色、深色或系统主题", icon: Palette },
  { to: "/settings/navigation", title: "导航自定义", summary: "排列分组和页面，管理侧栏收藏", icon: ListTree },
  { to: "/settings/island", title: "灵动岛设置", summary: "配置岛屿位置、通知、外观和交互", icon: PanelsTopLeft },
];

const themes = [
  { id: "system", name: "跟随系统", icon: Monitor },
  { id: "light", name: "浅色", icon: Sun },
  { id: "dark", name: "深色", icon: Moon },
] satisfies { id: Theme; name: string; icon: typeof Monitor }[];

export function SettingsPage() {
  const { pathname } = useLocation();
  const theme = useSelector((state: RootState) => state.preferences.theme);
  const dispatch = useDispatch();
  const section = sections.find(item => item.to === pathname);
  return <>
    {section && <div><Button variant="link" size="sm" nativeButton={false} render={<Link to="/settings" />}>返回设置</Button></div>}
    <PageHeader title={section?.title ?? "设置"} description={section?.summary ?? "管理工作台偏好与桌面功能。"} />
    {!section && <Panel title="工作台设置">
      <ItemGroup className="has-data-[size=sm]:gap-[3px]">{sections.map(({ to, title, summary, icon: Icon }) =>
        <div key={to} role="listitem"><Item render={<Link to={to} />} variant="outline" size="sm">
          <ItemMedia variant="icon" className="size-9"><Icon aria-hidden="true" /></ItemMedia>
          <ItemContent><ItemTitle>{title}</ItemTitle><ItemDescription>{summary}</ItemDescription></ItemContent>
          <ItemActions><ChevronRight size={16} aria-hidden="true" /></ItemActions>
        </Item></div>)}</ItemGroup>
    </Panel>}
    {pathname === "/settings/appearance" && <Panel title="工作台主题">
      <ToggleGroup aria-label="工作台主题" className="grid w-full grid-cols-[repeat(auto-fit,minmax(min(140px,100%),1fr))] gap-2.5"
        value={[theme]} onValueChange={values => {
        const next = themes.find(item => item.id === values[0]);
        if (next) dispatch(setTheme(next.id));
      }}>
        {themes.map(({ id, name, icon: Icon }) =>
          <ToggleGroupItem key={id} value={id} className="h-auto justify-start gap-2.5 px-3 py-3.5">
            <Icon aria-hidden="true" />{name}{theme === id && <Check className="ml-auto" aria-hidden="true" />}
          </ToggleGroupItem>)}
      </ToggleGroup>
    </Panel>}
    {pathname === "/settings/navigation" && <NavigationEditor />}
  </>;
}
