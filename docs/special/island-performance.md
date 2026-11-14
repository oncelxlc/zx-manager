# Windows 灵动岛渲染自检与性能采样

Windows 桌面的灵动岛由独立 Win32 工具窗口绘制。检查原生窗口、合成过渡或指针命中时，在目标显示器上运行原生自检。设置页的 React 预览用于编辑外观。

## 运行原生窗口自检

在已配置 Rust 和 Windows 桌面开发环境的交互式桌面，从仓库根目录运行以下命令。

```powershell
cargo run --locked --manifest-path src-tauri/Cargo.toml --example island_native_poc
```

自检在主屏工作区顶部创建测试窗口，使用独立的内存配置。它检查首次显示、透明区域点击穿透、展开中途反向、辅助功能展开操作、窗口定位及关闭后的资源释放。保持测试区域可见，其下方需要有其他进程的窗口，以便验证点击穿透。有副屏时会检查实际副屏位置和 DPI，模拟缩放检查使用 125%、150% 和 200%。成功结束时控制台输出 `Native Island PoC` 和渲染计数，失败时返回对应检查的错误。[自检入口与断言](../../src-tauri/src/island/surface/poc.rs)

## 读取当前窗口的渲染统计

灵动岛已启动时，Tauri 主窗口可调用 `island_get_performance_stats`。返回对象中的 `renderer` 为 `native`，`native` 对象包含下列字段。自检程序输出同一组原生渲染字段。要观察一次操作带来的绘制或提交次数，可在同一窗口操作前后分别读取累计值并求差。

| 字段 | 口径 |
| --- | --- |
| `presents`、`commits` | 当前渲染器的纹理呈现次数和 DirectComposition 提交次数。一次提交可启动由系统播放的过渡。 |
| `recoveries`、`lastRecoveryUs` | 图形设备恢复成功次数和最近一次恢复耗时，单位为微秒。 |
| `lastApplyUs` | 最近一次应用渲染状态的宿主耗时，单位为微秒。显示器刷新间隔和输入到画面的延迟需要单独测量。 |

命令返回「灵动岛窗口未启动」时，先在设置中启用或重启岛屿，再重新读取。[诊断命令](../../src-tauri/src/island/commands.rs) · [渲染计数实现](../../src-tauri/src/island/render/mod.rs)

## 进程与画面采样脚本

在 Windows 交互式桌面打开 PowerShell 7.4 或更高版本。安装前端依赖后，在仓库根目录构建前端并运行默认采样。脚本自行在本地端口 16421 启动和关闭 Vite 预览服务，每次只运行一种模式。

```powershell
pnpm build
.\scripts\island-benchmark.ps1
```

每次运行都会在 `src-tauri/target/island-bench-<id>/` 创建临时工作区。脚本复制仓库根目录的 `Cargo.toml` 和 `Cargo.lock`，将插入采样代码的桌面程序放在其 `src-tauri/` 成员下，用 `cargo build --locked` 构建 Debug 程序。测试配置和 WebView2 数据也位于本次运行目录。[工作区配置](../../Cargo.toml) · [采样脚本](../../scripts/island-benchmark.ps1)

| 模式 | 采样内容 |
| --- | --- |
| 默认 | 收起空闲 10 秒、100 对常规展开与收起、100 对快速反向。记录宿主及所属 WebView2 进程的 CPU、阶段末工作集、管理请求耗时和可用的 GPU 引擎计数器。CPU 的 100% 代表占满一个逻辑核。 |
| `-WarmIdle` | 预热 20 秒后，连续采集三个 10 秒的空闲窗口，分列宿主和 WebView2 的 CPU。 |
| `-VisualProbe` | 配合 `island_frame_probe` 捕获岛屿区域，统计状态切换后首次同向像素变化和可见变化间隔。采样观察的是桌面合成画面。 |

预热空闲模式运行 `.\scripts\island-benchmark.ps1 -WarmIdle`。画面模式需要先构建探针，再将其可执行文件传给脚本。以下路径采用 Cargo 默认的输出目录。

```powershell
cargo build --locked --manifest-path src-tauri/Cargo.toml --example island_frame_probe
.\scripts\island-benchmark.ps1 -VisualProbe .\target\debug\examples\island_frame_probe.exe
```

控制台的 `Results:` 行给出本次运行目录。默认模式在其 `native/` 下写出 `idle.json`、`cycles.json`、`reverse.json`，预热空闲模式写出 `warm1.json` 至 `warm3.json`。各文件记录阶段耗时、进程数、工作集和原生渲染计数。`dispatch_us` 依次为状态切换请求的 P50、P95、P99，空闲阶段三项均为 `null`。解读 GPU 利用率时一并查看 `gpu_samples`，它为 0 表示没有可用样本。[资源采样实现](../../scripts/island-benchmark.rs)

画面模式的结果为 `native/visual-output.json`，包含刷新率、展开收起与快速反向的画面变化、隐藏与唤醒的延迟以及各稳定状态的像素检查。脚本报错时，可在同目录查看 `app.err`、`error.txt` 或画面模式的 `probe.err`。

**画面探针对边缘条有已知误报。** 默认深色配置的 `edgePeek` 是一条无文字的黑色窄条。探针要求各稳定状态都有亮色像素，还要求唤出边缘条时没有深色像素，因此会写出 JSON 后以非零状态退出，并提示「岛屿内容未显示或固定态尺寸异常」。排查时可在 `stable.edgePeek` 中核对 `boundsPx` 和 `pixels` 判断是否捕获窄条，同时检查其他状态的内容及固定态尺寸。[画面采样实现](../../src-tauri/examples/frame_probe/windows.rs) · [边缘条绘制](../../src-tauri/src/island/render/paint.rs)
