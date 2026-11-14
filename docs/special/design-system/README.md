# 界面主题与配色

工作台使用 Graphite 中性色和 Mineral Blue 交互色。「设置 → 外观」提供浅色、深色和跟随系统三种选项。HTML 入口在样式加载前读取主题偏好，工作台挂载后在跟随系统模式下监听系统外观变化。[主题初始化](../../../index.html) · [主题切换](../../../src/app/AppShell.tsx)

## 颜色变量

浅色值定义在 `:root`，深色值由 `:root.dark` 覆盖。StyleX 可以直接使用 `var(--card)` 等颜色。shadcn/ui 通过 Tailwind 的 `@theme inline` 将同一变量映射为 `bg-card`、`text-foreground`、`border-input` 和 `ring-ring` 等工具类。[主题变量定义](../../../src/styles.css)

| CSS 变量 | 浅色 | 深色 | 用途 |
| --- | --- | --- | --- |
| `--background` | `#F5F6F7` | `#171A1F` | 页面底色、状态栏 |
| `--foreground` | `#20262D` | `#E5E9EE` | 标题、正文 |
| `--sidebar` | `#F0F2F4` | `#1C2127` | 侧栏与窄窗口顶栏 |
| `--card` | `#FFFFFF` | `#1E242B` | 普通面板、表单控件 |
| `--popover` | `#FFFFFF` | `#242B33` | 菜单、弹窗、浮层 |
| `--secondary` | `#E9EDF1` | `#282E36` | 次要操作、元数据徽章 |
| `--muted` | `#ECEFF2` | `#252B33` | 普通徽章、悬停、骨架屏 |
| `--muted-foreground` | `#68727E` | `#9AA4AF` | 页面或卡片上的辅助文字、占位符 |
| `--accent` | `#E7EDF2` | `#2A3641` | 选中的导航、选项和切换控件 |
| `--border` | `#DDE2E7` | `#303842` | 面板边界、分隔线 |
| `--input` | `#D5DCE3` | `#343D47` | 输入框、次要按钮边界 |
| `--primary` | `#496F91` | `#789AB7` | 主操作、链接、选择标记 |
| `--primary-hover` | `#3F617F` | `#8AAAC4` | 主按钮悬停 |
| `--primary-foreground` | `#FFFFFF` | `#111820` | 主按钮文字、复选框勾号 |
| `--ring` | `#6486A3` | `#789AB7` | 键盘焦点 |
| `--overlay` | `#171A1F52` | `#111820A6` | Dialog 和 Sheet 遮罩，末两位为透明度 |
| `--scrollbar-thumb` | `#7D8791` | `#6B7784` | 原生滚动条滑块 |

### 状态色

`--success`、`--warning`、`--destructive` 和 `--info` 用于文字和图标，对应的 `-muted` 用于徽章底色。状态同时保留文字或图标。类型、时间、分支名和版本号使用中性徽章。

| 状态 | 浅色文字 / 底色 | 深色文字 / 底色 | 界面示例 |
| --- | --- | --- | --- |
| Success | `#396A52` / `#E7F0EB` | `#A0C5AF` / `#24352E` | 已安装、已连接、工作区干净 |
| Warning | `#795924` / `#F4EEE2` | `#D5B781` / `#393228` | 工作区有变更、分支落后 |
| Destructive | `#98474B` / `#F6E9E9` | `#DFA4A4` / `#3C2D32` | 读取失败、部署失败 |
| Info | `#456987` / `#E6EEF4` | `#A0BED5` / `#283643` | 传输排队、分支领先 |
| Neutral | `foreground` / `muted` | `foreground` / `muted` | 普通 Badge，次要变体使用 `secondary-foreground` / `secondary` |

## shadcn/ui 映射

- `card-foreground`、`popover-foreground`、`secondary-foreground`、`accent-foreground` 和 `sidebar-foreground` 共用 `foreground`。选中背景上的小字使用完整前景色。
- `sidebar-primary`、`sidebar-primary-foreground`、`sidebar-accent`、`sidebar-accent-foreground`、`sidebar-border`、`sidebar-ring` 分别引用同名的基础角色。导航用 `sidebar-accent` 铺底，2px 侧标及图标取 `sidebar-primary`。
- `chart-1` 至 `chart-5` 依次引用 `primary`、`muted-foreground`、`success`、`warning`、`destructive`。实心危险操作的文字角色 `destructive-foreground` 引用 `primary-foreground`。

| 组件或场景 | 使用的颜色 |
| --- | --- |
| Button、Checkbox、Slider | 主按钮和勾选用 `primary` / `primary-foreground`。滑轨用 `muted`，已填部分用 `primary`，拖柄用 `card` 和 `ring` |
| Input、Textarea、Select、Field | 控件用 `card` 和 `input`，焦点用 `ring`，错误用 `destructive` |
| Tabs、Toggle、Sidebar | 中性悬停，选中项使用 `accent` 或 `card` 底色，配合主色描边、图标或选中标记 |
| ContextMenu、Select 列表、Dialog、Sheet、Toast | 面板取 `popover` / `popover-foreground`，浮层边界取 `border`，模态遮罩取 `overlay`。Toast 按结果给图标着色 |
| Tooltip | 反相中性色 `foreground` 底、`background` 字，箭头与底色一致 |
| Badge、Alert | 普通信息取中性色，业务状态显式指定对应变体 |
| Card、Item、Table、Skeleton | 卡片取 `card`，行悬停取中性表面，分隔线取 `border`，骨架取 `muted` |

## 灵动岛外观

灵动岛设置页和预览画布的外框跟随工作台主题。岛体背景、边框和主题由 `IslandConfig.appearance` 独立控制，页内预览读取当前草稿。预览的媒体封面、充电和低电量色集中在灵动岛模块内。[预览配色](../../../src/features/island/settingsStyles.ts) · [岛体默认配置](../../../src/features/island/defaults.json)

同一总览页面的两种工作台外观可对照查看。[浅色总览截图](overview-light.png) · [深色总览截图](overview-dark.png)
