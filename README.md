# zx-once

zx-once 是基于 Tauri 2、React、Vite 和 React Router 的桌面项目。Windows 版包含主窗口和独立的置顶灵动岛。浏览器和桌面共用中文欢迎页，点击「灵动岛设置」可进入设置中心。

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

## Windows 灵动岛

桌面功能面向 Windows 10 / 11，主界面需要 WebView2，开发需要 Rust 1.89 或更高版本。运行 `pnpm tauri:dev` 即可启动 Rust 原生灵动岛，默认在主显示器顶部居中显示收起的胶囊。主窗口使用 `#/`，设置中心使用 `#/settings/island`。在普通浏览器运行 `pnpm dev` 可以编辑草稿、播放设置预览。

- **显示与后台**　默认关闭主窗口会将它隐藏，灵动岛继续运行。从通知区域的 zx-once 菜单可打开主界面、打开设置、显示或隐藏灵动岛、退出应用。在「启动管理」中可以调整关闭行为和登录启动。登录启动使用当前用户的 Run 项。
- **交互**　默认悬停或点击可展开。展开后可以固定、取消固定或隐藏。固定后保留展开状态。拖动胶囊可以更换位置，打开位置记忆后，松开鼠标时保存。设置中心也支持选择显示器、九宫格锚点、自定义坐标和展开方向。坐标与尺寸使用逻辑像素，自定义坐标的原点是所选显示器工作区的左上角。
- **快捷键**　默认用 `Ctrl+Alt+I` 切换、`Ctrl+Alt+S` 显示、`Ctrl+Alt+H` 隐藏。在「交互与快捷键」可以更改组合或留空以释放该快捷键。完全隐藏后仍可用托盘唤醒。
- **边界隐藏**　关闭边界策略、离开后收起、保留可悬停唤醒的边缘窄条、完全隐藏，共四种模式。隐藏和唤醒延迟可调。全屏暂停默认开启，退出全屏后恢复之前的状态。
- **动画**　在「动画效果」选择 Subtle、Smooth、Spring 或 Custom，调整时长、幅度与性能档位。Custom 还可选择默认平滑、匀速、渐快或渐慢曲线。「减少动态效果」和 Windows 动画偏好控制过渡。预览可以取消，点击「保存设置」才会提交草稿。

### 配置和故障排查

灵动岛在同一进程的专用消息线程创建 Win32 工具窗，以 Direct2D、DirectWrite 和 DXGI 绘制内容，并使用 DirectComposition 播放过渡。设置、快捷键和位置记忆沿用已有 v1 配置。设置页的「重启岛屿」会重建原生窗口。要单独验证窗口、合成和资源释放，可在仓库根目录运行以下自检程序。

```powershell
cargo run --manifest-path src-tauri/Cargo.toml --locked --example island_native_poc
```

实测数据、采样口径和兼容性验收范围见[性能基线](docs/island-performance-baseline.md)、[原生渲染设计](docs/island-native-renderer-design.md)和[迁移验证报告](docs/island-performance-report.md)。

Windows 配置保存在 `%APPDATA%\com.oncel.zx-once\island.json`，再次保存时会保留上一份配置为 `island.json.bak`。遇到版本冲突，放弃旧草稿后按当前配置重新编辑。如果文件损坏，先退出应用，保留损坏文件，再将备份复制为 `island.json`。高于当前应用支持的配置版本需要对应版本的应用。

若快捷键被占用，先通过托盘打开设置，换一个组合再保存。找不到岛屿时，在总览确认已启用，退出全屏应用并从托盘选择显示。显示器断开后会回退到可用显示器，也可以在设置中重新选择显示器。独占全屏、混合 DPI、焦点和动画表现需要在目标机器验收。

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
