# ZxManager

ZxManager 是一个基于 Tauri 的本地基础设施管理控制台。目前提供一个可交互的服务与系统资源 Dashboard，适合作为接入真实本机服务管理能力前的产品界面基础。

![ZxManager application icon](public/icon.png)

## 当前功能

- 深色优先的桌面管理界面，支持亮色、深色和跟随系统主题。
- 中文与 English 界面切换，偏好设置会保存在本地浏览器存储中。
- 系统资源趋势图、服务概览与可配置列的服务表格。
- 添加、启动、停止、重启、移除服务以及通知反馈等完整前端交互。
- 基于 Tauri 2、React 19、TypeScript、Vite 7、Tailwind CSS 4、shadcn/Base UI 和 Recharts 构建。

> 当前服务数据和操作均为前端 Mock。`src-tauri` 仍只保留 Tauri 模板的 `greet` 命令，尚未接入系统服务、进程或配置文件；请不要将界面操作视为对本机服务的真实控制。

## 快速开始

### 环境要求

- Node.js（建议使用项目当前支持的 LTS 版本）
- pnpm
- Rust stable 工具链，以及 [Tauri 开发环境要求](https://v2.tauri.app/start/prerequisites/)

### 安装与运行

```bash
pnpm install
pnpm tauri:dev
```

仅运行 Web 前端时：

```bash
pnpm dev
```

Vite 开发服务器固定使用 `http://localhost:1420`；Tauri 开发模式会自动启动它。

## 常用命令

| 命令 | 用途 |
| --- | --- |
| `pnpm dev` | 启动 Vite 前端开发服务器。 |
| `pnpm tauri:dev` | 启动带热更新的完整 Tauri 桌面应用。 |
| `pnpm build` | 执行 TypeScript 类型检查并构建前端生产包。 |
| `pnpm tauri build` | 构建平台相关的桌面应用安装包。 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 运行 Rust 单元测试。 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 检查 Rust 代码格式。 |

## 项目结构

```text
@/                         shadcn/Base UI 基础层、主题和通用工具
src/
  app/                     路由定义
  components/              项目业务组件
  data/                    稳定 Mock 数据
  i18n/                    国际化初始化、翻译资源与格式化工具
  layouts/                 共享页面布局
  pages/                   路由页面
  services/                Mock 服务接口和本地偏好存储
  styles/                  全局样式
  types/                   领域类型
src-tauri/
  src/                     Rust 应用入口与 Tauri 命令
  capabilities/            Tauri 权限声明
  icons/                   应用图标
```

路径别名将 `@/*` 映射到仓库根目录的 `@/`（shadcn 基础层），将 `src/*` 映射到 `src/`（项目业务代码）。新增组件时请保持这个边界。

## 开发说明

- 应用在渲染前读取偏好、应用主题并初始化 i18n，以减少首次渲染时的主题和语言闪烁。
- 用户可见文案使用 `react-i18next` 翻译资源；新增文案时同步更新 `src/i18n/locales/zh-CN.ts` 和 `src/i18n/locales/en-US.ts`。
- 若要接入真实服务管理，请先在 `src-tauri/src/` 实现受权限约束的 Tauri 命令，再替换 `src/services/tauri/service-manager.ts` 中的 Mock 实现；不要直接在前端执行系统命令。
- UI 基础组件由 shadcn 的 `base-nova`/Base UI 配置管理，相关文件位于 `@/components/ui/`。

## 推荐编辑器

[VS Code](https://code.visualstudio.com/) 配合 [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) 与 [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) 扩展可获得较好的开发体验。
