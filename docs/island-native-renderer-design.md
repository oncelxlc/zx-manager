# Windows 灵动岛原生渲染设计

## 现有边界

Windows 桌面灵动岛由 Rust 创建和绘制。持久化、定时状态迁移、托盘、全屏检测和拖动后的坐标换算均在 Rust 中。设置页的草稿预览在 React 页面内运行，桌面窗口使用已保存的配置。

```text
React 设置页 / 托盘 ── IslandManager ── ConfigStore / Machine
                           │       ▲
             配置、目标状态与显示器   │ 进入、离开、点击、拖动和系统事件
                           ▼       │
                    原生 HWND 消息线程
                           │
                    Direct2D / DirectWrite
                           ▼
                   DXGI 合成交换链
                           ▼
                  DirectComposition → DWM
```

## 工作线程和接口

管理线程串行处理命令、配置 revision、状态优先级和单个状态截止时间。显示器继续使用 Tauri 提供的设备标识和工作区，保留已保存的屏幕选择。配置中的尺寸及偏移使用 DIP，目标矩形使用虚拟桌面的物理像素。选择的屏幕断开时按主屏、首个可用屏幕的顺序回退。拖动完成后才把屏幕位置转换回显示器局部 DIP。

原生消息线程拥有 HWND 和图形对象。窗口过程将有业务含义的输入投递给管理线程。进入、离开以及操作按钮才触发状态处理。动画中间值留在合成器中。窗口所属线程按需监听前台窗口及全屏状态。关闭时先解除监听，再释放窗口和线程。

合成交换链采用预乘 alpha。背景、文字、图标和边缘条按内容变化更新。合成器接管视觉属性。实机测试发现，展开时仅靠 `HTTRANSPARENT` 会截住其他进程窗口上的点击，因此窗口线程在几何过渡期间还会等待与合成节拍对齐的单次计时器。轮廓实际变化时才用 `SetWindowRgn` 更新系统命中区域，关闭重绘标志。到达终点即停钟，线程继续等待消息。[DirectComposition 动画](https://learn.microsoft.com/en-us/windows/win32/directcomp/animation) · [DXGI 合成交换链](https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_2/nf-dxgi1_2-idxgifactory2-createswapchainforcomposition) · [WM_NCHITTEST](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-nchittest)

## 窗口、资源与动画

- 管理器直接持有原生窗口。重启时先解除系统监听、关闭旧 HWND 并等待消息线程结束，然后创建下一代窗口。管理器按 generation 丢弃旧窗口的迟到事件。
- HWND 使用 `WS_POPUP`、`WS_EX_NOACTIVATE`、`WS_EX_TOOLWINDOW`、`WS_EX_TOPMOST` 和 `WS_EX_NOREDIRECTIONBITMAP`。显示与拖动遵循非激活策略。隐藏后可从托盘唤醒，停用后重新启用时按需创建窗口。移动监测依靠窗口消息与 `TrackMouseEvent`。
- 静态表面按收起、展开、固定、边缘条分别缓存，以物理尺寸、DPI、配色、状态、按钮和外观属性决定是否重绘。D2D 设备上下文、画刷和 DirectWrite 布局复用。每条交换链使用两张预乘 alpha 的 BGRA 缓冲，只有内容更新或资源重建才调用 Present。
- 过渡使用 DComp 的有终点三次函数。位移、缩放和各层透明度共享 QPC 起点。背景和内容在同一交换链上分区绘制，用裁剪分成两个合成视觉。Balanced 随幅度调整内容位移，Quality 额外应用内容缩放。反向时用同一条曲线求当前可见值，再安装新的属性动画。拖动开始时固定当前视觉和宿主矩形，交给系统拖动。窗口按状态切换定位，空闲和动画中间帧均无需管理 IPC。
- 命中区域覆盖胶囊、实例入口及翻页操作，并与绘制共用布局。MSAA 提供中文控件名、按钮角色、状态与默认动作。读屏操作进入同一状态机。
- `DXGI_ERROR_DEVICE_REMOVED/RESET/HUNG`、驱动内部错误及 D2D 目标失效可触发一次设备与目标重建，并重画当前状态。其他图形错误返回现有的运行错误通道。重建失败后用户可在设置页重启岛屿。

## 管理命令与诊断

仅本地 `main` 窗口具有灵动岛管理权限。配置、启停、显示/隐藏、重启、固定、屏幕、位置及状态查询均通过管理命令完成。设置页沿用 `island://state-changed`、`config-changed`、`position-changed`、`display-changed` 和 `runtime-error`。岛内指针输入和拖动由 Win32 消息直接驱动。

`island_get_performance_stats` 返回固定为 `"native"` 的 `renderer` 字段及 `native` 统计对象。统计按本次窗口生命周期记录成功的静态纹理呈现次数 `presents`、成功合成提交次数 `commits`、成功重建设备次数 `recoveries`，以及最近一次恢复和应用渲染状态的墙钟耗时 `lastRecoveryUs`、`lastApplyUs`。屏幕可见帧间隔由独立采样取得。
