import { createSlice, type PayloadAction } from "@reduxjs/toolkit";

export interface Project {
  id: string;
  name: string;
  path: string;
  language: string;
  tags: string[];
  ide: string;
  lastOpened: number;
  pinned: boolean;
  pinnedAt?: number;
  gitId?: string;
  githubId?: string;
  environmentId?: string;
  serverId?: string;
}

const items: Project[] = [
  {
    id: "zx-once", name: "zx-once", path: "D:/Demo/zx-once", language: "TypeScript",
    tags: ["桌面", "Tauri"], ide: "VS Code", lastOpened: Date.UTC(2026, 10, 13, 2, 12),
    pinned: false, gitId: "zx-once", githubId: "zx-once", environmentId: "node", serverId: "build",
  },
  {
    id: "api-service", name: "api-service", path: "D:/Demo/api-service", language: "Rust",
    tags: ["服务端"], ide: "RustRover", lastOpened: Date.UTC(2026, 10, 12, 5, 18),
    pinned: false, gitId: "api-service", githubId: "api-service", environmentId: "rust", serverId: "staging",
  },
  {
    id: "web-client", name: "web-client", path: "D:/Demo/web-client", language: "TypeScript",
    tags: ["前端"], ide: "VS Code", lastOpened: Date.UTC(2026, 10, 11, 9, 46),
    pinned: false, gitId: "web-client", githubId: "web-client", environmentId: "node", serverId: "staging",
  },
];

const projects = createSlice({
  name: "projects",
  initialState: { items },
  reducers: {
    projectAdded(state, { payload }: PayloadAction<Project>) {
      state.items.push(payload);
    },
    projectEdited(state, { payload }: PayloadAction<Pick<Project, "id" | "name" | "path" | "language" | "tags" | "ide">>) {
      const project = state.items.find(item => item.id === payload.id);
      if (project) Object.assign(project, payload);
    },
    projectRemoved(state, { payload }: PayloadAction<string>) {
      state.items = state.items.filter(item => item.id !== payload);
    },
    projectPinToggled(state, { payload }: PayloadAction<{ id: string; at: number }>) {
      const project = state.items.find(item => item.id === payload.id);
      if (project) {
        project.pinned = !project.pinned;
        project.pinnedAt = project.pinned ? payload.at : undefined;
      }
    },
    projectOpened(state, { payload }: PayloadAction<{ id: string; at: number }>) {
      const project = state.items.find(item => item.id === payload.id);
      if (project) project.lastOpened = payload.at;
    },
  },
});

export const { projectAdded, projectEdited, projectRemoved, projectPinToggled, projectOpened } = projects.actions;
export const projectsReducer = projects.reducer;

export function projectTime(time: number) {
  return time ? new Date(time).toLocaleString("zh-CN", {
    month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit", hour12: false,
  }) : "尚未打开";
}
