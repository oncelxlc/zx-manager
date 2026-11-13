import { createSlice, type PayloadAction } from "@reduxjs/toolkit";

export type ResourceCategory = "runtime" | "toolchain" | "package";
export interface EnvironmentResource {
  id: string;
  name: string;
  category: ResourceCategory;
  versions: string[];
  defaultVersion: string | null;
  path: string;
  description: string;
}

function resource(id: string, name: string, category: ResourceCategory, versions: string[], description: string): EnvironmentResource {
  return {
    id, name, category, versions, description, defaultVersion: versions[0] ?? null,
    path: versions.length ? `D:/Demo/Tools/${id}` : "",
  };
}

const items = [
  resource("node", "Node.js", "runtime", ["24.15.0", "22.16.0"], "运行 JavaScript 服务及前端构建脚本。可从 Node.js 官网选择所需版本的 Windows 安装包。"),
  resource("rust", "Rust", "runtime", ["1.89.0", "1.88.0"], "编译 Rust 项目。通过 rustup 管理编译器与不同的工具链版本。"),
  resource("python", "Python", "runtime", ["3.13.5", "3.12.9"], "运行 Python 脚本与开发工具。可从 Python 官网获取 Windows 安装包。"),
  resource("java", "Java", "runtime", ["21.0.7"], "使用 JDK 运行和编译 Java 项目。安装时选择项目要求的发行版及主版本。"),
  resource("go", "Go", "runtime", ["1.24.4"], "使用 Go 工具链编译服务与命令行工具。可从 Go 官网获取 Windows 安装包。"),
  resource("dotnet", ".NET", "runtime", [], "开发 .NET 应用需要对应版本的 SDK，可从 Microsoft .NET 下载页获取。"),
  resource("git", "Git", "toolchain", ["2.50.0"], "管理本地版本历史。Windows 用户可通过 Git for Windows 安装命令行客户端。"),
  resource("vscode", "Visual Studio Code", "toolchain", ["1.101.0"], "编辑本地工作区。可从 Visual Studio Code 官网下载桌面编辑器。"),
  resource("docker", "Docker CLI", "toolchain", [], "操作容器与镜像。可按 Docker 官方指南配置桌面客户端及引擎。"),
  resource("pnpm", "pnpm", "package", ["12.16.1", "10.12.1"], "管理 Node.js 项目依赖。可按 pnpm 官方指南安装与项目锁文件兼容的版本。"),
  resource("npm", "npm", "package", ["11.3.0"], "管理 JavaScript 软件包，通常随 Node.js 一同安装。"),
  resource("cargo", "Cargo", "package", ["1.89.0"], "管理 Rust 依赖与构建，通常随 Rust 工具链一同安装。"),
];

const environment = createSlice({
  name: "environment",
  initialState: { items, detectedAt: Date.UTC(2026, 10, 12, 8, 30) },
  reducers: {
    sampleDetected(state, { payload }: PayloadAction<number>) {
      state.detectedAt = payload;
    },
    defaultVersionChanged(state, { payload }: PayloadAction<{ id: string; version: string }>) {
      const item = state.items.find(resource => resource.id === payload.id);
      if (item?.versions.includes(payload.version)) item.defaultVersion = payload.version;
    },
  },
});

export const { sampleDetected, defaultVersionChanged } = environment.actions;
export const environmentReducer = environment.reducer;
