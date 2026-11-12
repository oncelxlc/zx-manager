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

桌面功能面向 Windows 10 / 11，主界面需要 WebView2，开发需要 Rust 1.89 或更高版本。运行 `pnpm tauri:dev` 即可启动原生灵动岛，默认在主显示器工作区顶部居中显示。主窗口使用 `#/`，设置中心使用 `#/settings/island`。在普通浏览器运行 `pnpm dev` 可以编辑草稿、查看无媒体时的预览。

- **主窗口与托盘**　关闭主窗口会保存当前页面、设置分组、设置草稿和首页清单的展开状态，然后释放 WebView。左键点击托盘图标可恢复到原处。右键菜单仍提供打开主界面、灵动岛设置、显示、隐藏和退出。选择「退出应用」会停止后台监听并清除内存中的页面快照。
- **常驻栏**　默认使用岛仔的经典黑贴顶刘海。100% 大小时，顶部常驻主体高 30，左右各留 16，槽间距 12。默认伙伴和时钟占用 120 宽，电池增加 32，音乐增加 26。仅显示时钟时主体宽 96。刘海上缘两侧各向外延伸 5。笔记本接通电源时显示绿色电量环和闪电。[岛仔常驻栏源码](https://github.com/Renly1994/xiaotang-daozai/blob/6eb33732f9b2150e02eef1b1f824b0c087a2c5f2/src/bar_layout.rs#L47-L75)
- **音乐首页**　悬停或点击即可展开。顶部和左右停靠共用横向 419 × 140 的音乐主体。Windows 媒体会话提供曲目、封面、进度和可用的播放控制，空闲时显示「暂无音乐」。主窗口和托盘均可打开设置。有曲目时默认使用歌词分栏，播放器未提供歌词时显示「暂无歌词」。按「词」可切换到全宽曲目信息。应用音量和收起时的歌词入口以不可用状态展示。[岛仔音乐首页源码](https://github.com/Renly1994/xiaotang-daozai/blob/6eb33732f9b2150e02eef1b1f824b0c087a2c5f2/src/pages.rs)
- **外观和停靠**　岛体大小可从 80% 调至 140%，每档 5%，再随显示器 DPI 缩放。顶部、左侧和右侧均可沿边移动，位置范围为 −100% 到 100%。胶囊距停靠边 6；刘海贴边。极简模式将顶部常驻栏收为 44 × 4.5 的细线，侧边相应转为竖线。以上尺寸均为 100% 下的逻辑像素。[岛仔设置源码](https://github.com/Renly1994/xiaotang-daozai/blob/6eb33732f9b2150e02eef1b1f824b0c087a2c5f2/src/model.rs#L173-L181)
- **边界与动画**　收起后可留在原处、缩成可唤醒的边缘窄条或完全隐藏。边缘隐藏和唤醒可单独设定延迟。完全隐藏时从托盘显示。全屏暂停默认开启。动画支持 Subtle、Smooth、Spring 和 Custom，遵循应用及 Windows 的减少动态效果偏好。
- **通知**　启用通知后，收到可归属的 Windows 通知时，常驻栏会显示对应的微信或 QQ 图标和红点。点击可切换到窗口，同一来源有多个实例时先选择实例。音乐首页仍可从常驻栏展开。
- **启动管理**　可调整登录启动、启动时显示和初始展开状态。普通安装使用当前用户的 Run 项，MSIX 使用 Windows 启动任务。

普通 Windows 安装支持原生窗口发现和切换。系统通知红点需要具备通知能力的包身份和用户授权。可选 x64 MSIX 的打包方法见[通知与窗口入口设计](docs/notification-island-design.md)。

### 配置和故障排查

灵动岛在同一进程的专用消息线程创建 Win32 工具窗，以 Direct2D、DirectWrite 和 DXGI 绘制内容，并使用 DirectComposition 播放过渡。v6 将独立宽高、圆角及边缘条尺寸替换为岛仔的布局和 100% 初始缩放。迁移保留原配置备份和目标显示器，按显示器尺寸将旧像素偏移换算成沿边百分比；旧自由或底部位置投影至最近的可用边。若曾单独关闭某一通知来源或系统通知监听，迁移后总开关保持关闭。设置页的「重启岛屿」会重建原生窗口。要单独验证窗口、合成和资源释放，可在仓库根目录运行以下自检程序。

```powershell
cargo run --manifest-path src-tauri/Cargo.toml --locked --example island_native_poc
```

实测数据、采样口径和兼容性验收范围见[性能基线](docs/island-performance-baseline.md)、[原生渲染设计](docs/island-native-renderer-design.md)和[迁移验证报告](docs/island-performance-report.md)。

普通 Windows 安装的配置保存在 `%APPDATA%\com.oncel.zx-once\island.json`。MSIX 的配置受 Windows 包数据目录规则管理。保存或迁移时会保留上一份配置为 `island.json.bak`。遇到版本冲突，放弃旧草稿后按当前配置重新编辑。如果文件损坏，先退出应用，保留损坏文件，再将备份复制为 `island.json`。高于当前应用支持的配置版本需要对应版本的应用。

找不到岛屿时，在总览确认已启用，退出全屏应用并从托盘选择显示。显示器断开后会回退到可用显示器，也可以在设置中重新选择显示器。独占全屏、混合 DPI、焦点和动画表现需要在目标机器验收。

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
