import { createSlice, type PayloadAction } from "@reduxjs/toolkit";

export interface HostedRepository {
  id: string;
  name: string;
  accountId: string;
  owner: string;
  visibility: "Public" | "Private";
  defaultBranch: string;
  description: string;
  syncedAt: string;
}

interface WorkItem {
  repoId: string;
  number: number;
  title: string;
  kind: "pr" | "issue";
  state: "open" | "closed";
  author: string;
  date: string;
}

const repositories: HostedRepository[] = [
  { id: "zx-once", name: "zx-once", accountId: "studio", owner: "demo-studio", visibility: "Public",
    defaultBranch: "main", description: "桌面开发工作台与本地项目管理。", syncedAt: "2026-11-13 09:40 UTC" },
  { id: "api-service", name: "api-service", accountId: "studio", owner: "demo-studio", visibility: "Private",
    defaultBranch: "main", description: "服务接口、路由与共享类型。", syncedAt: "2026-11-13 09:38 UTC" },
  { id: "workbench-docs", name: "workbench-docs", accountId: "studio", owner: "demo-studio", visibility: "Public",
    defaultBranch: "main", description: "开发工作台使用手册与资源索引。", syncedAt: "2026-11-13 09:35 UTC" },
  { id: "web-client", name: "web-client", accountId: "lab", owner: "demo-lab", visibility: "Private",
    defaultBranch: "main", description: "前端组件与客户端演示。", syncedAt: "2026-11-12 16:20 UTC" },
];

const items: WorkItem[] = [
  { repoId: "zx-once", number: 27, title: "支持键盘切换导航分组", kind: "pr", state: "open", author: "demo-editor", date: "2026-11-13 09:32" },
  { repoId: "zx-once", number: 25, title: "窄窗口下保留详情入口", kind: "issue", state: "open", author: "demo-reviewer", date: "2026-11-12 16:10" },
  { repoId: "api-service", number: 18, title: "更新接口响应示例", kind: "pr", state: "closed", author: "demo-editor", date: "2026-11-12 11:20" },
  { repoId: "workbench-docs", number: 8, title: "校对快速入门文档", kind: "issue", state: "closed", author: "demo-reviewer", date: "2026-11-11 10:00" },
  { repoId: "web-client", number: 11, title: "调整列表筛选器", kind: "pr", state: "open", author: "demo-lab", date: "2026-11-12 14:20" },
  { repoId: "web-client", number: 7, title: "补充空列表反馈", kind: "issue", state: "open", author: "demo-editor", date: "2026-11-11 09:04" },
];

const slice = createSlice({
  name: "github",
  initialState: {
    accounts: [{ id: "studio", name: "Demo Studio" }, { id: "lab", name: "Demo Lab" }],
    accountId: "studio", repositories, items,
  },
  reducers: {
    chooseAccount(state, { payload }: PayloadAction<string>) {
      if (state.accounts.some(account => account.id === payload)) state.accountId = payload;
    },
  },
});

export const { chooseAccount } = slice.actions;
export const githubReducer = slice.reducer;
