# Windows 灵动岛原生渲染迁移报告

## 交付范围

Windows 桌面版统一使用 Rust 原生灵动岛。本文保留迁移时的双后端对照数据（提交 `4682531`），供后续回归比较。当前启动方式和实现见 [README](../README.md) 和[原生渲染设计](island-native-renderer-design.md)。

```text
WebView    设置/托盘 → Rust Machine → WebView2 岛屿 + CSS 内容过渡
                                   → 16 ms HWND 位移/变形 → DWM
Native     设置/托盘 → 同一 Machine → 专用线程上的 Win32 HWND
                                   → D2D/DirectWrite → DXGI → DComp → DWM
                                   ← 本地消息、区域命中、MSAA 按钮动作
```

## 实现文件和依赖

| 文件 | 内容 |
| --- | --- |
| `src-tauri/src/island/{displays,native}.rs`、`native/watch.rs` | 显示器选择、系统事件监听和快捷键 |
| `src-tauri/src/island/surface.rs`、`surface/` | STA 消息线程、窗口生命周期、定位与命中、按钮与拖动、辅助功能、独立 PoC |
| `src-tauri/src/island/render/{mod,device,paint,animation}.rs` | D3D11/D2D/DWrite/DXGI/DComp 设备、分状态纹理缓存、合成属性动画、诊断及恢复 |
| `src-tauri/src/island/{config,commands,contract_tests,manager,mod}.rs`、`manager/settings.rs`、`src-tauri/src/lib.rs`、`src-tauri/permissions/island.toml` | 管理桥、配置扩展与旧版读取、IPC 权限测试、原生事件与拖动保存 |
| `src/features/island/{types.ts,SettingsGroups.tsx,PreviewContent.tsx,IslandPreview.tsx,IslandSettingsPage.test.tsx}` | Custom 曲线、设置与草稿预览、前端交互回归 |
| `src-tauri/{Cargo.toml,Cargo.lock}` | Windows 专用依赖和 API features |
| `src-tauri/examples/`、`scripts/island-benchmark.{ps1,rs}` | 原生 PoC、屏幕变化采样和隔离配置冒烟测试 |
| `README.md`、`docs/island-{performance-baseline,native-renderer-design,performance-report}.md` | 运行方式、测量口径、架构设计和验收结果 |

新增直接引用的 `windows`、`windows-core`、`windows-numerics` 提供 COM 图形接口、MSAA 实现宏及几何类型。`window-vibrancy` 和 `raw-window-handle` 复用 Tauri 依赖树中的 Acrylic 实现。`windows-sys` 继续提供原有 Win32 调用。所有包均使用锁文件中已有版本。

## 产品和视觉差异

- 设置页预览渲染本地草稿，桌面岛屿由原生窗口绘制。管理命令及诊断字段见[原生设计](island-native-renderer-design.md)。
- 保留状态点、品牌、中文标题、上下箭头、固定/取消固定、隐藏、边缘条、背景与描边。原实现没有通知流或进度条。DirectWrite 采用 Segoe UI 与系统字体回退，原生线条重绘图标。灰度文字边缘和用细描边模拟的内阴影会与浏览器栅格化略有不同。
- 模糊使用 Windows Acrylic。合成过渡中涉及宿主窗口透明区域时会暂时关闭材质，稳定后恢复。Windows 不支持材质时仍显示配置的半透明底色。
- 旧 Custom 配置保持平滑曲线。可选曲线在设置预览和原生合成中均生效。系统动画偏好、减少动态效果和性能档位仍参与过渡选择。

## 采集当前原生版本的指标

在 Windows 交互桌面的仓库根目录运行以下命令。

```powershell
pnpm build
pwsh -File scripts/island-benchmark.ps1
pwsh -File scripts/island-benchmark.ps1 -WarmIdle
cargo build --manifest-path src-tauri/Cargo.toml --locked --example island_frame_probe
pwsh -File scripts/island-benchmark.ps1 -VisualProbe src-tauri/target/debug/examples/island_frame_probe.exe
```

资源基准与画面基准分开运行。脚本为每次测试创建独立的配置与 WebView2 数据目录，只在临时源码副本中加入驱动和关闭登录项写入，退出后保留原始测量结果的路径。前者采集应用进程树，后者只回读隔离背景上的岛屿矩形，并输出变化统计。桌面合成像素变化与物理显示器发光之间仍有显示链路延迟。

## 功能回归

| 场景 | 结果 |
| --- | --- |
| 桌面冷启动 | 通过。以独立应用标识执行 `pnpm tauri:dev --no-watch`，设置总览显示「已收起」。在实际设置窗口选择 Custom 后出现「过渡曲线」，初始值为「默认平滑」。检查结束后停止开发实例 |
| 窗口和命中 | PoC 检查置顶、非激活、工具窗样式、胶囊中心与透明圆角。跨进程测试复现并修复了展开时透明区域截住点击的问题。修复后的稳定态和展开中测试点均命中原下层窗口 |
| 原生按钮 | Windows 通过 `AccessibleObjectFromWindow` 读出按钮名称，并通过默认动作触发取消固定。鼠标、辅助功能和设置页动作进入同一状态机 |
| 方向与主题 | 主屏中央收起区域为 (1202, 754, 156, 44)。Quality 模式下四向展开均为 328 × 168，上、下、左、右的实际左上角依次为 (1116, 630)、(1116, 754)、(1030, 692)、(1202, 692)。暗色/浅色实窗背景分别取到 `#202326` / `#f5f5f4`，固定按钮底色分别为 `#275447` / `#cdece3`。跟随系统在该机器上取到暗色。禁用固定后 MSAA 按钮数从 3 降至 2 |
| 拖动与多屏 | 拖动开始后可见矩形保持固定。真实副屏工作区原点为 (-2560, 0)，在该屏检查了收起、展开和边缘条。矩形中心落在屏外或显示器间空隙时的选屏、嵌套拖动消息泵中的管理读取和取消有独立回归用例 |
| 配置和重启 | 原生版本的隔离基准检查配置落盘、颜色更新、重启后的 revision、快捷键重新注册与释放，同时校验当前进程仅有一个原生岛屿 HWND，Tauri 仅有主窗口。v1 不带曲线字段的配置保留既有效果 |
| GPU 恢复和 DPI | 在隐藏 HWND 上受控释放 D3D/DComp 资源后，重新绑定设备并恢复当前画面。在同一台机器上输入 1.25、1.5 和 2.0 缩放参数，检查交换链 resize、重绘和重复应用的缓存命中。物理驱动重启及混合 DPI 换屏另列验收 |

原生状态层第一次拆分时，尺寸和主色都能通过检查，文字却被背景盖住。根据实际内容像素和 DirectComposition 的子视觉排序规则，将内容明确置于背景之上，并把文字和固定提示纳入回归检查。仅对比窗口矩形会漏掉这个错误。[AddVisual 子视觉排序](https://learn.microsoft.com/en-us/windows/win32/api/dcomp/nf-dcomp-idcompositionvisual-addvisual)

## Windows 11 桌面合成实测

同机 Cargo dev 构建，主窗口隐藏，深色 Smooth，保留实际岛屿窗口。资源基准统计宿主和其 WebView2 子进程，100% CPU 表示一个逻辑核。工作集为各进程之和，共享页可能重复计入。

| 资源场景 | CPU：WebView → Native | 工作集 MiB：WebView → Native | 应用进程树最忙 GPU 引擎均值：WebView → Native |
| --- | ---: | ---: | ---: |
| 启动预热后收起空闲 10 s | 2.19% → 3.91% | 524.36 → 478.29 | 0.00% → 0.00% |
| 100 对完整展开/收起 | 46.03% → 9.02% | 578.01 → 486.17 | 0.90% → 0.00% |
| 100 对快速反向 | 75.32% → 29.74% | 582.05 → 486.42 | 0.96% → 0.00% |

完整切换时，旧 HWND 动画回调平均墙钟 4.403 ms、615 万 CPU 周期，原生命中区域回调平均 0.936 ms、148 万 CPU 周期。两者工作内容不同。原生路径完成 400 次状态切换后，累计 2 次静态纹理 Present、401 次合成 Commit，收起空闲期间没有动画回调。GPU 百分比来自有限的 Windows 性能计数器采样，不含共享 DWM，0.00% 读数不能用于推断整机 GPU 工作量。

空闲 CPU 的短样本没有稳定优势。预热延长到 20 s 后，连续三个 10 s 窗口中，WebView 分别为 2.97%、1.09%、1.41%，Native 为 2.66%、1.09%、0.94%。这次复测中 Native 工作集约 491 MiB，WebView 约 531 MiB。两条路径的宿主动画回调均为零，原生纹理和合成计数保持不变。进程拆分及每窗样本见[性能基线](island-performance-baseline.md)。

主屏为 2560 × 1600、60 Hz、96 DPI。下表是从桌面复制图像中观测到的变化。每个场景各运行 12 对展开/收起，管理请求和桌面更新均在 QPC 时间轴上定位。[DXGI 桌面更新时间](https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_2/ns-dxgi1_2-dxgi_outdupl_frame_info)

| 场景与指标，单位 ms | WebView P50 / P95 / P99 | Native P50 / P95 / P99 |
| --- | ---: | ---: |
| 完整切换，可见变化帧间隔 | 16.73 / 32.61 / 33.49 | 16.67 / 17.21 / 17.80 |
| 完整切换，请求至首个同向画面变化 | 32.44 / 57.64 / 64.91 | 16.52 / 19.77 / 20.14 |
| 快速反向，可见变化帧间隔 | 16.82 / 32.66 / 33.74 | 16.63 / 17.14 / 17.23 |
| 快速反向，请求至首个同向画面变化 | 41.08 / 65.40 / 79.42 | 22.69 / 37.36 / 38.36 |

完整切换中，超过 1.5 个刷新周期的可见变化间隔为 WebView 27/401、Native 0/398，快速反向为 6/96、0/105。没有按应用归因的 GPU 丢帧率。两种模式各 48 次切换均出现预期方向变化。显、隐、边缘收起和唤醒另各做 4 次，每次均抵达预期稳态。单独隐藏的末次像素变化 P95 从 16.03 ms 到 26.47 ms，这个短场景没有显示加速。各动作首末变化详见[性能基线](island-performance-baseline.md)。

修复图层顺序后，两种后端的默认胶囊主色均为 RGB(39,42,44)，边缘条均呈白色。收起/展开的亮文字及图标像素由 203/1018 变为 173/894，固定态绿色填充由 880 像素变为 865 像素。早期无捕获边框轮次实测收起 156 × 44、展开及固定 328 × 168、边缘条 156 × 6；最终轮次受 Windows 捕获指示边框影响，收起/边缘态精确宽度仍采用前次记录。字体抗锯齿、描边和像素数存在可见差异。

## 构建与测试

- `pnpm check` 通过。包含 TypeScript、组件行数检查、6 项 Vitest、3 项 Node 测试以及 Vite 生产构建。
- `cargo fmt --check` 和 `cargo clippy --locked --all-targets --all-features -- -D warnings` 通过。沙箱内的增量缓存不能使用硬链接时，编译器改用文件复制。
- `cargo test --locked` 的 26 项常规 Rust 测试全部通过。另以 `cargo test --locked --lib -- --ignored --test-threads=1 --nocapture` 在交互桌面运行其余 2 项。快捷键和系统钩子两轮注册与注销成功，图形设备在同一 HWND 上受控重建成功，本轮耗时 165.485 ms。
- `cargo test --locked --all-targets --all-features` 还覆盖了桌面画面采样器的间隔统计、显隐稳态识别与捕获边框识别测试。

## 待实测的发布条件

本次系统为 Windows 11，主屏报告 60 Hz，副屏报告 59 Hz，两屏均为 96 DPI。Windows 10、120/144 Hz、物理混合 DPI、显示器热拔插、不同任务栏位置、独占全屏、睡眠唤醒、真实驱动重置和长时间资源稳定性仍需目标环境。物理按键到屏幕发光需要额外的端到端测量。
