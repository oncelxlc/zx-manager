# zx-once

zx-once 是基于 React 和 Tauri 2 的中文开发者工作台。浏览器和桌面共用总览、项目、开发环境、Git、GitHub、远程中心和工具箱。项目与远程操作使用会话内的演示数据，工具箱在本地处理输入。Windows 桌面另提供托盘和原生置顶灵动岛，可查看媒体、控制播放并切换有提醒的微信或 QQ 窗口。

## 快速启动

在仓库根目录使用 Node.js 24.15+（24.x）或 26.x，按 `packageManager` 指定的 pnpm 版本安装依赖并启动开发服务器。[包管理器版本](package.json)

```sh
pnpm install --frozen-lockfile
pnpm dev
```

浏览器打开 `http://localhost:1420`。在 Windows 上体验托盘和灵动岛，先准备 Rust 1.90 或更高版本、Visual Studio C++ 构建工具和 WebView2，然后在仓库根目录单独运行 `pnpm tauri:dev`。该命令会自行启动前端开发服务器。

## 项目文档

| 分类 | 内容 |
| --- | --- |
| 总体 | [文档目录](docs/README.md) · [项目结构、运行与验证](docs/overview/project.md) |
| 模块 | [开发者工作台](docs/modules/workbench.md) · [Windows 灵动岛](docs/modules/island.md) |
| 特殊 | [界面主题与配色](docs/special/design-system/README.md) · [Windows 通知与 MSIX 打包](docs/special/windows-notifications.md) · [灵动岛性能诊断](docs/special/island-performance.md) |

项目采用 MIT 许可。灵动岛的外观与布局改编自岛仔（Daozai）。[许可证与署名](LICENSE)
