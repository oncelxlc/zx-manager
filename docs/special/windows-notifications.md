# Windows 通知与窗口切换

微信和 QQ 的红点表示 Windows 通知中心仍保留着该应用在本次运行期间发出的提醒。灵动岛只读取通知的应用标识、ID 和创建时间。红点按应用归属，多开时共用同一个红点。

## 启用与授权

1. 在 Windows 桌面版打开「灵动岛设置」，在「总览」启用灵动岛并保存。
2. 在「通知」勾选「启用通知」，点击「保存设置」。这会启动当前 Windows 会话的客户端发现和系统通知监听。
3. 从具有通知访问能力的 MSIX 包入口启动时，若页面提供「请求 Windows 通知访问」，点击后完成系统授权。若已拒绝，点击「打开 Windows 通知隐私设置」调整授权。

普通开发进程、MSI 和 NSIS 安装的进程缺少 MSIX 包身份时，仍可发现客户端，但无法读取通知中心。浏览器版仅提供设置草稿和外观预览。关闭 Windows 主窗口到托盘后，已启用的监听会继续运行。停用灵动岛或关闭通知并保存，会停止监听并清空实例和提醒。

## 红点与窗口入口

- 工作线程每两秒扫描一次客户端及其顶层窗口。受支持的程序名为 `WeChat.exe`、`Weixin.exe` 和 `QQ.exe`，同时校验本地固定磁盘上的文件、产品名称和腾讯发布者签名。没有可激活窗口的客户端可以保留后台实例。
- 常规收起态仅为有系统提醒的运行来源显示图标和红点，极简模式隐藏这些图标。单个可激活实例可从图标直接切换，有多个实例时先选择窗口，条目较多时可翻页。收到提醒本身不会改变前台窗口。
- 切换前会复核进程、启动时间和窗口归属，再恢复最小化窗口并请求置前。客户端已退出时重新选择窗口，Windows 拒绝置前时可从任务栏打开。
- 遇到「已获准读取 Windows 通知」但没有红点，先检查是否使用常规收起态、对应客户端是否运行，再查看 Windows 通知中心是否保留了本次运行期间的提醒。仅在客户端内弹出的消息不会触发红点。移除某应用的最后一条系统提醒后，该应用的红点随之清除。

设置界面只接收权限状态及其更新序号。通知识别、窗口列表和激活均在原生进程内处理。[通知监听实现](../../src-tauri/src/island/notifications/toasts.rs) · [窗口识别与激活](../../src-tauri/src/island/notifications/windows.rs)

## 构建具备通知能力的 x64 MSIX

准备 x64 Windows 开发环境、Rust 的 `x86_64-pc-windows-msvc` 目标、项目的 pnpm 依赖，以及包含 `MakeAppx.exe` 的 Windows SDK。以下命令在仓库根目录运行，使用 Cargo 工作区默认的 `target/` 输出目录。将 `CN=实际发布者` 替换为签名证书完整的 Subject。

```powershell
pnpm tauri build --no-bundle --target x86_64-pc-windows-msvc
./scripts/package-notification-msix.ps1 -Publisher "CN=实际发布者" -Executable ./target/x86_64-pc-windows-msvc/release/zx-once.exe -Output ./target/zx-once.msix
```

生成的 `target/zx-once.msix` 是未签名包。若自定义了 Cargo 输出目录，将 `-Executable` 指向实际生成的 x64 Release 程序，并将 `-Output` 指向已有目录下的新文件。脚本会拒绝覆盖已有包。使用发布者匹配、目标系统信任的证书签名后，安装包并从开始菜单启动。主窗口需要 WebView2。包清单通过 `userNotificationListener` 声明通知访问能力。[MSIX 清单](../../src-tauri/msix/AppxManifest.xml) · [打包脚本](../../scripts/package-notification-msix.ps1)
