# 开发约定

- `src/` 存放 React 和 TypeScript 欢迎页，`src-tauri/` 存放 Tauri 桌面入口与打包配置。桌面主窗口直接加载前端。
- 使用 React Router 的哈希路由，让浏览器与 Tauri 共用入口。跨组件状态通过 Redux Toolkit 与 React Redux 管理。
- 基础控件使用 `src/components/ui/` 中的 shadcn/ui 源码，沿用 `components.json` 指定的 Base Nova 组件样式与 Neutral 主题，由 Tailwind CSS 编译。页面布局和灵动岛预览可使用 StyleX，图标从 `lucide-react` 按需导入。
- 使用 pnpm 管理前端依赖，使用 Cargo 管理 Rust 依赖。调整依赖时同步更新对应的锁文件。
- TypeScript 保持严格类型检查，使用两空格缩进、双引号和分号。生产 React 组件遵循仓库的 200 有效行检查。
- 修改前端后运行 `pnpm check`，依次完成类型检查、自动化测试和前端构建。界面变动还应通过 `pnpm tauri:dev` 检查桌面显示。
- 将交互测试放在实现旁的 `*.test.tsx`，使用 Vitest、Testing Library 和 jsdom。路由和 Redux 行为使用真实路由和独立的测试 store 验证。
- 修改 Rust 后，在 `src-tauri/` 中运行 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings` 和 `cargo test`。
- 确需引入主机操作时，仅授予该操作所需的 Tauri 权限。用户可见功能、依赖或启动方式变化时同步更新 README。

## 代码注释

- 当命名和类型不足以说明实现原因时，在相关代码段上方添加单行注释（`//`）。例如，保留最后一份有效草稿时，说明表单数值临时为空对窗口快照的影响。
- 当组件或方法的正确使用依赖调用顺序、资源清理或跨端契约时，在定义处用多行注释说明必要的前提和可观察的行为。TypeScript 可用 `/** … */`，Rust 对外接口可用连续的 `///`。
- 注释语言沿用所在模块的惯例。跳过仅复述变量名、JSX 结构或方法签名的说明。行为变化时同步更新或移除过时注释。
