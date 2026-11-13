import { createSlice, type PayloadAction } from "@reduxjs/toolkit";

export interface GitChange {
  path: string;
  status: "M" | "A" | "D";
}

interface Commit {
  id: string;
  message: string;
  at: string;
  files: GitChange[];
}

export interface GitBranch {
  name: string;
  changes: GitChange[];
  history: Commit[];
  ahead: number;
  behind: number;
}

export interface GitRepository {
  id: string;
  name: string;
  githubId: string;
  currentBranch: string;
  branches: GitBranch[];
}

function branch(name: string, message: string, changes: GitChange[] = [], ahead = 0, behind = 0): GitBranch {
  return {
    name, changes, ahead, behind,
    history: [{ id: "d000001", message, at: "2026-11-12T08:30:00Z", files: [{ path: "README.md", status: "M" }] }],
  };
}

const repositories: GitRepository[] = [
  { id: "zx-once", name: "zx-once", githubId: "zx-once", currentBranch: "main", branches: [
    branch("main", "更新工作台说明", [{ path: "src/App.tsx", status: "M" }, { path: "docs/navigation.md", status: "A" }]),
    branch("feature/navigation", "整理导航分组", [], 1),
  ] },
  { id: "api-service", name: "api-service", githubId: "api-service", currentBranch: "develop", branches: [
    branch("develop", "补充接口示例", [], 0, 2), branch("main", "归档演示配置"),
  ] },
  { id: "web-client", name: "web-client", githubId: "web-client", currentBranch: "main", branches: [
    branch("main", "发布界面示例", [], 1), branch("docs", "更新组件目录"),
  ] },
];

interface BranchTarget { repoId: string; branchName: string }

function findBranch(repos: GitRepository[], target: BranchTarget) {
  return repos.find(repo => repo.id === target.repoId)?.branches.find(item => item.name === target.branchName);
}

const slice = createSlice({
  name: "git",
  initialState: { repositories, sequence: 1 },
  reducers: {
    selectBranch(state, { payload }: PayloadAction<BranchTarget>) {
      const repo = state.repositories.find(item => item.id === payload.repoId);
      if (repo?.branches.some(item => item.name === payload.branchName)) repo.currentBranch = payload.branchName;
    },
    stageSample(state, { payload }: PayloadAction<BranchTarget>) {
      const current = findBranch(state.repositories, payload);
      if (current && !current.changes.some(file => file.path === "README.md")) {
        current.changes.push({ path: "README.md", status: "M" });
      }
    },
    commitSample(state, { payload }: PayloadAction<BranchTarget & { message: string; at: string }>) {
      const current = findBranch(state.repositories, payload);
      const message = payload.message.trim();
      if (!current?.changes.length || !message) return;
      const id = "d" + (++state.sequence).toString(16).padStart(6, "0");
      current.history.unshift({ id, message, at: payload.at, files: current.changes.splice(0) });
      current.ahead += 1;
    },
    syncSample(state, { payload }: PayloadAction<BranchTarget & { direction: "pull" | "push"; at: string }>) {
      const current = findBranch(state.repositories, payload);
      if (!current) return;
      if (payload.direction === "push") {
        current.ahead = 0;
      } else if (current.behind) {
        for (let count = 0; count < current.behind; count++) {
          const id = "d" + (++state.sequence).toString(16).padStart(6, "0");
          current.history.unshift({ id, message: "同步远端样本更新 " + (count + 1), at: payload.at,
            files: [{ path: "docs/notes.md", status: "M" }] });
        }
        current.behind = 0;
      }
    },
    cloneSample(state, { payload }: PayloadAction<{ id: string; name: string; defaultBranch: string }>) {
      if (state.repositories.some(repo => repo.githubId === payload.id)) return;
      state.repositories.push({
        id: payload.id, name: payload.name, githubId: payload.id, currentBranch: payload.defaultBranch,
        branches: [branch(payload.defaultBranch, "建立演示工作副本")],
      });
    },
  },
});

export const { selectBranch, stageSample, commitSample, syncSample, cloneSample } = slice.actions;
export const gitReducer = slice.reducer;
