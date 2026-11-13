import {
  Activity, BookMarked, Boxes, Cable, CircleDot, Clock3, Container, FolderKanban,
  FolderTree, GitBranch, GitPullRequest, HardDrive, LayoutDashboard, Network,
  Hammer, Rocket, Server, Terminal, Wrench,
} from "lucide-react";

export const destinations = {
  overview: { label: "总览", to: "/", icon: LayoutDashboard },
  recent: { label: "最近使用", to: "/?section=recent", icon: Clock3 },
  activity: { label: "活动记录", to: "/?section=activity", icon: Activity },
  projects: { label: "项目", to: "/projects", icon: FolderKanban },
  git: { label: "Git 工作区", to: "/git", icon: GitBranch },
  environment: { label: "开发环境", to: "/environment", icon: Boxes },
  toolchain: { label: "工具链", to: "/environment?tab=toolchain", icon: Hammer },
  github: { label: "GitHub", to: "/github", icon: BookMarked },
  repositories: { label: "仓库", to: "/github?tab=repositories", icon: HardDrive },
  prs: { label: "Pull Requests", to: "/github?tab=prs", icon: GitPullRequest },
  issues: { label: "Issues", to: "/github?tab=issues", icon: CircleDot },
  remote: { label: "远程中心", to: "/remote", icon: Network },
  connections: { label: "SSH 连接", to: "/remote?tab=connections", icon: Cable },
  servers: { label: "服务器", to: "/remote?tab=servers", icon: Server },
  files: { label: "远程文件", to: "/remote?tab=files", icon: FolderTree },
  terminal: { label: "终端", to: "/remote?tab=terminal", icon: Terminal },
  docker: { label: "Docker", to: "/remote?tab=docker", icon: Container },
  deployments: { label: "部署", to: "/remote?tab=deployments", icon: Rocket },
  toolbox: { label: "工具箱", to: "/toolbox", icon: Wrench },
};

export type NavId = keyof typeof destinations;
export interface NavGroup {
  id: string;
  title: string;
  items: NavId[];
  hidden: boolean;
}
export type Theme = "system" | "light" | "dark";
export interface Preferences {
  theme: Theme;
  sidebar: "auto" | "expanded" | "collapsed";
  groups: NavGroup[];
  hidden: NavId[];
  pins: NavId[];
}

export const defaults: Preferences = {
  theme: "system",
  sidebar: "auto",
  groups: [
    { id: "workbench", title: "工作台", items: ["overview", "recent", "activity"], hidden: false },
    { id: "local", title: "本地开发", items: ["projects", "git", "environment", "toolchain"], hidden: false },
    { id: "collaboration", title: "代码协作", items: ["github", "repositories", "prs", "issues"], hidden: false },
    { id: "remote", title: "远程基础设施", items: ["remote", "connections", "servers", "files", "terminal", "docker", "deployments"], hidden: false },
    { id: "utilities", title: "开发辅助", items: ["toolbox"], hidden: false },
  ],
  hidden: ["toolchain", "repositories", "prs", "issues", "connections", "servers", "files", "terminal", "docker", "deployments"],
  pins: [],
};

export function isNavId(value: unknown): value is NavId {
  return typeof value === "string" && Object.prototype.hasOwnProperty.call(destinations, value);
}

export function activeDestination(path: string, search: string, visible: NavId[]) {
  const params = new URLSearchParams(search);
  const matching = visible.filter(id => destinations[id].to.split("?")[0] === path);
  return matching.find(id => {
    const query = destinations[id].to.split("?")[1];
    return query && [...new URLSearchParams(query)].every(([key, value]) => params.get(key) === value);
  }) ?? matching.find(id => destinations[id].to === path);
}
