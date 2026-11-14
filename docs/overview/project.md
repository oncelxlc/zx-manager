# 项目结构、运行与验证

zx-once 在浏览器和桌面主窗口中加载同一套 React 工作台。项目、环境、Git、GitHub 和远程中心提供可交互的演示数据。工具箱在本地转换文本。Windows 10 / 11 桌面通过 Tauri 和 Rust 接入托盘、置顶灵动岛、系统媒体会话及通知。[工作台页面与数据范围](../modules/workbench.md) · [灵动岛功能](../modules/island.md)

## 代码入口

| 位置 | 职责 |
| --- | --- |
| [前端启动](../../src/main.tsx)、[路由](../../src/App.tsx) | 启动时尝试恢复桌面窗口快照，创建 Redux store，再挂载 HashRouter。首页为 `#/`。 |
| [工作台外壳](../../src/app/)、[业务模块](../../src/features/)、[共享状态](../../src/store.ts) | 外壳管理导航、设置和页面布局。业务模块实现各自的操作和视图，通过 Redux 共享会话数据。 |
| [基础组件](../../src/components/ui/)、[组件配置](../../components.json)、[全局样式](../../src/styles.css) | shadcn/ui 使用 Base Nova 和 Neutral 基础配置，组件样式由 Tailwind CSS 编译。页面布局与岛体预览使用 StyleX，图标来自 lucide-react。 |
| [桌面入口](../../src-tauri/src/lib.rs)、[灵动岛](../../src-tauri/src/island/) | Windows 主窗口使用 WebView2。Rust 管理托盘、配置和独立的原生岛窗。关闭主窗口会释放 WebView，托盘可重开工作台。 |
| [Tauri 配置](../../src-tauri/tauri.conf.json)、[主窗口权限](../../src-tauri/capabilities/main.json) | 配置开发服务器地址、生产资源、窗口尺寸和安装包。Windows 本地 `main` 窗口可调用获准的灵动岛管理命令。 |

## 运行与构建

在仓库根目录使用 Node.js 24.15+（24.x）或 26.x。pnpm 版本由 `packageManager` 指定，前端依赖按锁文件安装。桌面端锁定的 Tauri 依赖要求 Rust 1.90 或更高版本。在 Windows 上还需 MSVC 工具链、C++ 构建工具和 WebView2。[前端命令与包管理器版本](../../package.json) · [桌面端依赖锁文件](../../Cargo.lock) · [Tauri 环境准备](https://v2.tauri.app/start/prerequisites/)

| 在仓库根目录执行 | 用途 |
| --- | --- |
| `pnpm install --frozen-lockfile` | 安装锁定的前端依赖。 |
| `pnpm dev` | 启动 Vite，浏览器访问 `http://localhost:1420`。 |
| `pnpm tauri:dev` | 启动 Vite 和桌面窗口。在 Windows 上同时启动托盘并按已保存的配置加载灵动岛。 |
| `pnpm build` | 执行 TypeScript 和 Vite 构建，输出到 `dist/`。 |
| `pnpm tauri:build` | 构建前端和桌面安装包。需要 Windows 系统通知权限的安装方式见 [MSIX 打包](../special/windows-notifications.md)。 |

## 验证

- **前端**　在仓库根目录运行 `pnpm check`。它依次运行 `pnpm typecheck`（TypeScript 和生产组件 200 有效行限制）、`pnpm test`（Vitest 和组件行数检查器的单元测试）、`pnpm build`。界面改动还需用 `pnpm tauri:dev` 打开桌面窗口，验证受影响的操作。
- **Rust**　在 `src-tauri/` 运行下列命令。Windows CI 对推送和拉取请求执行相同的 Rust 检查及前端检查。[CI 工作流](../../.github/workflows/check.yml)

```sh
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
```
