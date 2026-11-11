# Windows 灵动岛实施报告

本次在 Windows 11 26H2 x64（26300.9974）、WebView2 148.0.3967.96 上完成了前端和 Rust 检查，以及下表列出的桌面联调。23 项自动化用例全部通过。

## 工程基线（IS-001）

| 检查项 | 开工时的工程状态 |
| --- | --- |
| Rust | 本机 rustc 1.99.1、Cargo 1.99.1，项目使用 edition 2021，仓库未固定 rust-toolchain |
| Tauri | Cargo.lock 中 tauri 2.11.3、tauri-build 2.6.3，无独立 Tauri 插件。前端 CLI 2.12.4 |
| 前端 | pnpm 12.16.1；React 19.3.1、React Router 8.5.1、Redux Toolkit 2.14.0、StyleX 0.19.1 |
| 入口 | main.rs 调用 lib.rs 的 run，tauri.conf.json 声明唯一的 main 窗口；默认关闭即退出，无托盘和系统快捷键 |
| 路由与状态 | main.tsx 提供 HashRouter、Redux Provider；App.tsx 只有欢迎页和未知路由回退，store.ts 管理依赖清单展开状态 |
| 权限 | 没有自定义 commands、commands 权限或 capabilities 文件 |

## 实现范围

| 编号 | 已写入仓库的实现 |
| --- | --- |
| IS-002、003 | 权威 Rust 配置、字段默认值、范围校验、旧版迁移与 revision 冲突检查。跨进程文件锁覆盖读取和替换过程，临时文件写入并同步后再替换，同时保留上次成功配置 |
| IS-004、005 | IslandManager 串行处理命令、系统事件和当前有效的延迟任务。纯状态机管理停用、隐藏、边缘待唤醒、收起、展开、固定、全屏暂停七种状态 |
| IS-006、007 | 动态创建独立 island WebView，等待页面就绪后显示。窗口置顶、无边框、透明，使用无激活显示和胶囊形状命中区域 |
| IS-008 至 011 | 工作区和显示器缩放、九宫格锚点、自定义坐标、方向选择、边缘窄条、悬停/点击/固定、外部点击收起、系统拖动和松手保存 |
| IS-012 至 015 | 当前用户快捷键注册与释放、前台窗口和位置变化通知、全屏暂停与恢复、当前用户登录启动、托盘和关闭到后台 |
| IS-016、017 | 两种窗口尺寸、主题、颜色、透明度、圆角和边界外观。Rust 负责窗口几何动画，React 负责内容过渡和可取消预览；支持时长、幅度、动效预设、减少动态效果与性能档位 |
| IS-018、019 | 按主窗口和岛屿窗口区分 IPC 权限。提供设置中心的七个分组、独立岛屿路由和 Redux 配置/状态同步，保留原欢迎页 |
| IS-020 | 状态、持久化、几何、权限/IPC 契约和 React 交互用例均已通过，Windows 桌面端已实测设置保存、跨窗口状态同步及快捷键 |
| IS-021 | 已完成下列 Windows 11 单机用例，Windows 10 和多屏混合 DPI 等项目仍待实测 |

## 变更位置

| 文件或目录 | 职责 |
| --- | --- |
| `src-tauri/src/lib.rs`、`src-tauri/src/island/mod.rs` | Windows 安装入口、托盘菜单、关闭和退出 |
| `src-tauri/src/island/manager.rs`、`manager/settings.rs` | 命令队列、状态通知、可取消延迟、配置提交、系统设置及持久化失败时的恢复 |
| `src-tauri/src/island/config.rs`、`config_store.rs`、`state.rs` | 配置契约、磁盘存储、纯状态转换 |
| `src-tauri/src/island/commands.rs`、`src-tauri/permissions/island.toml`、`src-tauri/capabilities/{main,island}.json` | 命令参数、调用窗口校验、事件订阅权限 |
| `src-tauri/src/island/window.rs`、`geometry.rs`、`native.rs`、`native/{motion,watch}.rs` | 窗口、显示器和缩放、原生动画与命中区域、系统事件、登录启动和快捷键 |
| `src-tauri/src/island/{config_store_tests,state_tests,contract_tests}.rs` | 持久化、状态转换和 IPC 权限用例；配置与坐标用例与各自实现同文件 |
| `src/{App,WelcomePage}.tsx`、`src/store.ts` | 路由、设置入口、Redux reducer |
| `src/features/island/{types.ts,defaults.json,islandApi.ts,islandSlice.ts,useIsland.ts}` | 配置契约、浏览器预览默认值、IPC 和 Redux 同步 |
| `src/features/island/{IslandSettingsPage,SettingsFields,SettingsGroups,IslandPreview,IslandWindowPage,IslandShell}.tsx`、`settingsStyles.ts` | 设置表单、各组字段、预览、胶囊内容及样式 |
| `src/features/island/IslandSettingsPage.test.tsx` | 设置、预览及岛屿交互的自动化用例 |
| `src-tauri/{Cargo.toml,Cargo.lock}`、`package.json`、`pnpm-lock.yaml` | 直接依赖与锁文件 |
| `README.md`、本报告 | 使用入口、默认行为、配置恢复和验收状态 |

## 依赖与平台决策

前端使用官方 `@tauri-apps/api` 封装 invoke 和事件订阅。首次桌面启动时，CLI 报告 npm API 2.12.4 与 Rust tauri 2.11.3 的次版本不匹配。已将 npm 约束调整为 `~2.11.1`，同步锁文件并安装 2.11.1。重新运行前端检查和桌面冷启动后，版本诊断正常。Rust 沿用 Tauri 2.11.3 及其 tray-icon 特性，直接引用锁中已有的 serde 1.0.228、serde_json 1.0.150、windows-sys 0.61.2、winreg 0.55.0。原生模块仅在 Windows 编译。

Windows 使用 `RegisterHotKey` 接收指定快捷键。非激活窗口无法通过焦点变化可靠识别外部点击，因此仅在已展开且开启「点击岛屿外部时收起」时安装低级鼠标监听，它只判断点击是否落在窗口外。窗口位置和尺寸由 Rust 在同一个控制器上更新。隐藏或新动作会取消先前的几何动画，稳定状态下没有逐帧动画定时器。毛玻璃启用后使用系统 Acrylic 材质，在不支持的环境中保留半透明底色。Tauri 窗口权限和托盘采用官方接口。[Tauri capabilities](https://v2.tauri.app/security/capabilities/) · [Tauri 系统托盘](https://v2.tauri.app/learn/system-tray/)

位置记忆采用 Windows 显示设备名。显卡或扩展坞变化后系统可能重新编号，届时可在设置中重新选择目标显示器。

## 验证状态

| 项目 | 本次结果 |
| --- | --- |
| 依赖准备 | 已按前端锁文件安装依赖，跳过安装脚本；已离线更新 Cargo 锁文件，原有包版本未变 |
| `pnpm check`（类型检查、测试、构建） | 通过。TypeScript、组件行数检查、6 项 Vitest 用例、3 项 Node 用例和 Vite 生产构建均成功 |
| `cargo fmt --check` | 通过 |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | 通过。首次检查发现性能档位枚举与变体重名、托盘退出分支嵌套，修复后无警告；配置序列化值沿用原格式 |
| `cargo test --locked` | 14 项全部通过，0 项失败、0 项忽略。包含手动唤醒边缘条后重新启动隐藏计时的回归用例 |
| `pnpm tauri:dev` | 最终冷启动成功，Vite 在本地 1420 端口就绪，Cargo 启动 debug 可执行文件。欢迎页和设置页正常加载，总览显示「已收起」 |

### Windows 11 实窗结果

| 操作 | 观察结果 |
| --- | --- |
| 从欢迎页进入设置，在总览点击「固定岛屿」，最大化主窗口 | 设置页收到「已固定」，屏幕顶部可见独立的 328 × 168 胶囊。收起时捕获尺寸为 156 × 44 |
| 在设置窗口按 `Ctrl+Alt+H`，再按 `Ctrl+Alt+S` | 总览由「已固定」变为「已隐藏」，随后变为「已收起」。从「已隐藏」点击「重启岛屿」也能恢复为「已收起」 |
| 选择「仅露出边缘条」，点击保存，指针留在窗外 | 页面确认保存。磁盘配置为 `revision=1`、`edge.mode=peek`，延迟后顶部实际窗口缩为 156 × 6。开发进程重建后自动恢复「边缘待唤醒」 |
| 在边缘条状态按 `Ctrl+Alt+I`，保持指针静止在主窗口 | 首测发现唤醒后超过 900 ms 仍停在 156 × 44。修复后，同一操作约 153 ms 时读到「已收起」，约 1541 ms 时已回到「边缘待唤醒」 |
| 在「动画效果」点击「播放预览」 | 按钮切换为「取消预览」，结束后恢复「播放预览」。磁盘 revision 保持不变；主动取消且不保存草稿另有 Vitest 用例覆盖 |
| 保持「关闭主窗口后留在托盘」开启，按 Alt+F4 | 主窗口从可见窗口列表消失，本次启动的 zx-once 进程仍有响应，窗口标题为「zx-once 灵动岛」 |

边缘唤醒修复涉及 `state.rs`、`manager.rs` 和 `state_tests.rs`。手动显示或切换唤醒后，状态机通知管理器读取一次实际指针位置。指针在窗外时重新安排隐藏，悬停唤醒沿用原有指针事件。未引入轮询。

联调结束时已恢复测试前的默认选项。磁盘配置为 `revision=2`，逐项比较确认除版本计数外与默认值一致。最终桌面开发实例保持运行，停在设置总览。

### 待实测

本次桌面工具能捕获独立胶囊画面，但将胶囊内的直接点击识别为另一个 WebView2 窗口，且未列出可单独操作的系统通知区。因此岛内按钮、物理拖动和托盘菜单的逐项点击仍待验证。外部点击、前台焦点、快速悬停与动画反转、全屏应用、实际登录启动及退出后的系统资源释放也未作实机验收。

Windows 10、100% / 125% / 150% / 200% 混合缩放、负坐标副屏、四边任务栏、热插拔、睡眠唤醒和性能测量仍需对应环境。
