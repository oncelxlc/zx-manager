# ZxManager

ZxManager 是基于 Tauri 2、React、Vite 和 React Router 的单窗口桌面项目。浏览器和桌面共用一个中文欢迎页，主要依赖清单默认展开，点击标题可以收起或重新展开。

## 主要依赖

| 用途 | 技术 |
| --- | --- |
| 桌面窗口与打包 | Tauri 2（Rust） |
| 界面与类型检查 | React、React DOM、TypeScript |
| 开发服务器与前端构建 | Vite |
| 路由 | React Router 的 `HashRouter`，首页为 `#/` |
| 样式与组件 | StyleX、shadcn/ui（Base UI） |
| 图标 | lucide-react |
| 状态管理 | Redux Toolkit、React Redux |

依赖标签使用项目内的 shadcn/ui `Badge` 组件。组件源码放在 `src/components/ui/`，样式使用 StyleX 编写。添加组件时，复制所需组件的源码，将其样式迁移到 StyleX。[shadcn/ui 源码分发说明](https://ui.shadcn.com/docs)

## 本地运行

开发环境建议使用 Node.js 24（24.15.0 以上）或 26，以及 pnpm 12。在仓库根目录安装锁定版本的依赖并启动开发服务器。

```sh
pnpm install --frozen-lockfile
pnpm dev
```

要运行桌面应用，请先安装 Rust 工具链，并按操作系统准备 [Tauri 开发依赖](https://v2.tauri.app/start/prerequisites/)。

运行 `pnpm tauri:dev` 启动桌面窗口，或运行 `pnpm tauri:build` 生成桌面安装包。

## 测试与构建

运行 `pnpm check` 一次完成以下三项检查，也可以单独运行其中一项定位问题。

| 命令 | 检查内容 |
| --- | --- |
| `pnpm typecheck` | TypeScript 类型和生产 React 组件的有效行数 |
| `pnpm test` | Vitest 在 jsdom 中检查欢迎页、未知路由回退和 Redux 展开/收起交互，再运行组件行数检查器的单元测试 |
| `pnpm build` | 编译前端，将生产资源输出到 `dist/` |

开发时可以用 `pnpm test:watch` 持续运行前端测试。修改视觉样式后，在 `pnpm tauri:dev` 打开的窗口检查标签布局，并用键盘展开和收起依赖清单。

推送代码和创建或更新拉取请求时，GitHub Actions 会在 Windows 执行 `pnpm check`，并检查 Rust 格式、Clippy 和 Cargo 测试。需要本地复现 Rust 检查时，在 `src-tauri/` 运行以下命令。

```sh
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
```
