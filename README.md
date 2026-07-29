# ZxManager

ZxManager 是一个基于 Tauri 2 与 React 19 的本地基础设施管理控制台。项目目前同时包含可用的本机系统信息能力，以及用于验证产品交互的服务管理 Dashboard 原型。

![ZxManager application icon](public/icon.png)

## 项目状态

ZxManager 仍处于早期开发阶段。请注意区分以下两类能力：

| 能力 | 当前实现 |
| --- | --- |
| 系统信息 | 通过权限受控的 Tauri 命令读取真实的系统、CPU、内存、Swap、GPU、磁盘和运行环境信息。 |
| 诊断复制 | 通过 Tauri 剪贴板插件写入经过白名单筛选的诊断信息。 |
| 偏好设置 | 优先写入 Tauri Store，并保留旧版 `localStorage` 数据迁移和不可用时的降级路径。 |
| 应用重启 | 通过 Tauri Process 插件执行真实的应用重启，并在界面中提供确认和失败反馈。 |
| 网络监控 | 仅支持 Windows 10/11。用户明确启用并允许 UAC 后，通过独立辅助进程的 ETW 事件按应用统计 TCP/UDP 流量，并保存最多 7 天的本地 SQLite 历史。 |
| 代理归类 | 将应用流量归为静态代理、非代理或未知；PAC、自动代理、透明代理和无法可靠判断的连接只计入“全部”。 |
| 服务管理 | 服务数据、资源趋势以及添加、启动、停止、重启和移除操作均为前端 Mock，不会控制本机服务。 |

## 功能概览

- Dashboard：资源指标、趋势图、服务概览、列显示配置和服务操作反馈。
- 系统信息：查看操作系统、CPU、内存、Swap、GPU、磁盘、应用运行环境和数据可用性。
- 网络监控：在 Windows 10/11 按应用查看实时速率、会话累计和历史用量，并可分别按全部、代理、非代理切换；采样间隔可选 1/3/5/10 秒并默认持久化为 5 秒，采样缺口不会补零或跨缺口连线。
- 桌面体验：启动等待界面、应用重启、响应式侧栏和局部滚动的数据表格。
- 个性化：中文与 English 界面，以及亮色、深色和跟随系统三种主题。
- 状态处理：系统信息支持加载、刷新、错误、部分数据不可用和保留最近成功数据。
- 安全边界：真实桌面能力均通过显式 Tauri 命令或插件权限开放。

## 技术栈

- React 19、TypeScript、React Router、Zustand
- Tauri 2、Rust、sysinfo、wgpu、rusqlite（bundled SQLite）
- Vite 7、Tailwind CSS 4
- shadcn `base-nova`、Base UI、Lucide、Recharts
- i18next、react-i18next
- Vitest、jsdom、Testing Library

## 快速开始

### 环境要求

- Node.js LTS
- pnpm
- Rust stable 工具链
- 当前操作系统对应的 [Tauri 2 开发依赖](https://v2.tauri.app/start/prerequisites/)

安装依赖并启动完整桌面应用：

```bash
pnpm install
pnpm tauri:dev
```

仅启动 Web 前端：

```bash
pnpm dev
```

Vite 开发服务器使用 `http://localhost:1420`。浏览器模式可用于前端布局开发，但系统信息、网络监控、Tauri Store、剪贴板和应用重启等桌面能力需要在 Tauri 环境中验证。网络监控在浏览器模式会直接返回结构化的“不支持平台”错误，不会生成模拟流量。

## 常用命令

| 命令 | 用途 |
| --- | --- |
| `pnpm dev` | 启动 Vite 开发服务器。 |
| `pnpm tauri:dev` | 启动带热更新的 Tauri 桌面应用。 |
| `pnpm typecheck` | 执行 TypeScript 严格类型检查。 |
| `pnpm test` | 运行一次前端测试。 |
| `pnpm test:watch` | 以监听模式运行前端测试。 |
| `pnpm build` | 类型检查并构建前端生产资源。 |
| `pnpm tauri:build` | 构建当前平台的桌面安装包。 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 检查 Rust 格式。 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | 运行 Rust 静态检查。 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 运行 Rust 单元测试。 |

## 项目结构

```text
@/
  components/ui/           shadcn/Base UI 基础组件
  components/theme-provider.tsx
  hooks/                   UI 基础层 Hook
  lib/                     UI 通用工具
src/
  app/                     路由定义
  components/              应用组件
  data/                    Dashboard Mock 数据
  i18n/                    国际化初始化、翻译与格式化
  layouts/                 共享页面布局
  pages/                   路由页面
  services/
    storage/               偏好持久化边界
    tauri/                 前端到 Tauri/Mock 服务边界
  stores/                  Zustand 状态
  test/                    前端测试初始化与 Fixture
  types/                   领域类型
  utils/                   业务工具
src-tauri/
  capabilities/            主窗口能力与插件权限
  permissions/             应用命令权限
  src/network_monitor/     应用流量 DTO、Windows ETW 辅助进程、累计器、Manager 与 SQLite 存储
  src/system_information/  系统信息 DTO、采集器与 GPU 枚举
  src/lib.rs               插件注册与命令入口
```

`@/*` 映射到仓库根目录的 `@/`，仅用于可复用 UI 基础层；`src/*` 映射到应用代码。两个别名同时配置在 `tsconfig.json` 和 `vite.config.ts`。

## 开发约定

- 应用先显示启动界面，再读取偏好、应用主题并初始化 i18n，最后渲染主应用。不要破坏这个启动顺序。
- 所有新增的用户可见文案都应使用 `react-i18next`，并同步更新 `src/i18n/locales/zh-CN.ts` 与 `src/i18n/locales/en-US.ts`。
- UI 偏好通过 `src/services/storage/preferences-storage.ts` 读写；不要在组件中直接访问 Store 或 `localStorage`。
- 前端到桌面能力的调用统一放在 `src/services/tauri/`。新增真实系统操作时，必须同时实现受限的 Rust 命令或官方插件权限。
- 应用组件优先复用 `@/components/ui/` 中的现有基础组件和 `cn()`，不要引入重复的组件或样式体系。

## 接入真实服务管理

`src/services/tauri/service-manager.ts` 当前只模拟异步操作。若要接入真实服务管理，需要：

1. 在 `src-tauri/src/` 实现明确且可审计的 Rust 命令。
2. 在 `src-tauri/capabilities/` 和必要的权限文件中仅开放所需操作。
3. 保持 Rust 命令名、前端 `invoke()` 名称和 TypeScript 类型同步。
4. 替换 Mock 前先补充 Rust 单元测试、前端状态测试和目标平台桌面烟测。

不要从 WebView 直接执行任意 Shell 命令，也不要把当前 Dashboard 操作描述为真实的系统控制。

## 网络监控数据与隐私

- Manager 默认关闭。首次进入网络监控页会先说明本地存储与隐私范围，用户选择开始后才采集；此前不会创建 `network-usage.sqlite3`。
- 主应用始终保持普通权限。启用监控时，Windows 通过 UAC `runas` 启动同一可执行文件的隐藏辅助模式；ETW 会话和需要管理员权限的采集只存在于辅助进程。
- 主进程与辅助进程通过仅限本机的命名管道通信，并校验当前用户/SYSTEM ACL、启动 PID、随机 nonce 和协议版本。禁用监控、主进程退出或管道断开会停止 ETW 并结束辅助进程。
- “当前会话启停”和“应用启动时开始监控”相互独立。启动偏好只影响后续启动，启动采集失败不会阻止应用壳打开。
- 采样间隔可在网络监控页选择 1、3、5 或 10 秒，默认 5 秒；该偏好会在下次启动时先于自动监控恢复。
- 最近 10 分钟保留在内存环形缓存中；SQLite 保存最近 24 小时的一分钟桶和第 2–7 天的五分钟桶，超过 7 天的数据会被清理。
- 实时应用表与历史应用表支持 10、20、50 条分页；更多数据在表格内部滚动，窄窗口不会产生页面级横向溢出。
- 数据库位于 Tauri app data 目录，使用 WAL、`synchronous=NORMAL`、外键、忙等待和事务迁移。批次 ID 与提交记录位于同一事务，重试不会重复累计。
- 数据库 schema v2 仅保存规范化可执行路径的 SHA-256 应用 ID、可执行文件名、网络路径、桶时间和上下行字节数。不会保存完整程序路径、PID、原始 MAC、主机名、远程地址、域名、URL、端口、数据包内容或命令行。
- 从 v1 接口/VPN schema 升级时会在同一事务中清空旧历史，因为旧数据无法可靠映射到具体应用。
- 网络路径内部为 `proxy`、`direct` 和 `unknown`。静态代理端点可归为代理；PAC、自动代理、透明代理、TUN 和配置读取失败等不确定情况归为未知。未知仅计入“全部”，不会被伪装成代理或非代理。
- 应用流量是进程网络事件聚合值，不等同于网卡线速总量。本地代理转发可能同时计入发起应用和代理应用，页面会明确提示这一边界。
- 诊断复制白名单没有加入网络应用明细。

## 网络监控平台范围

| 平台 | 实现 | 页面行为 |
| --- | --- | --- |
| Windows 10/11 | 提权辅助进程持有 TCP/UDP IPv4/IPv6 ETW 实时会话，按应用和网络路径聚合后传回主进程。 | 提供完整实时与历史功能。 |
| Linux / macOS / 浏览器 | 不启动采集器，也不生成估算或 Mock 流量。 | 保留导航和路由，仅显示当前系统不支持。 |

UAC 取消、ETW 启动失败、辅助进程断开、协议不兼容和事件丢失都会以结构化状态显示；启动时恢复失败不会阻止应用主界面打开。

## 验证

前端修改至少运行：

```bash
pnpm typecheck
pnpm test
pnpm build
```

Rust 或 Tauri 权限修改还应运行：

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

可见 UI 修改应使用 `pnpm tauri:dev` 验证桌面流程；涉及响应式布局时，再通过 Web 前端检查 1024px 以下的 Sidebar 行为。表格应在自身容器内滚动，不应造成页面级横向溢出。

## 推荐编辑器

[VS Code](https://code.visualstudio.com/) 配合 [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) 与 [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) 扩展可获得较好的开发体验。
