const zhCN = {
  common: {
    app: { name: "Local Console", description: "基础设施管理器" },
    actions: { add: "添加", cancel: "取消", close: "关闭", confirm: "确认", more: "更多操作", refresh: "刷新", save: "保存", search: "搜索" },
    status: { error: "错误", failed: "失败", healthy: "运行正常", loading: "加载中", success: "成功", warning: "警告" },
  },
  navigation: {
    groups: { management: "管理", resources: "资源" },
    items: { certificates: "证书", configuration: "配置", dashboard: "仪表盘", help: "帮助", logs: "日志", networkPorts: "网络端口", nginx: "Nginx", search: "搜索", services: "服务", settings: "设置", systemMonitor: "系统监控" },
    labels: { openMachineActions: "打开本机操作" },
    machine: { name: "本地计算机", platform: "Windows 11 · x64", openGuide: "打开指南", restartApp: "重启应用", systemInfo: "系统信息" },
    toast: { futureAction: "此操作已为后续 Tauri 命令准备就绪。", moduleMock: "此模块当前以本地仪表盘模拟方式呈现。", selected: "已选择 {{label}}" },
  },
  dashboard: {
    title: "仪表盘", subtitle: "监控和管理本地基础设施服务 · {{updated}}", updatedJustNow: "刚刚更新", updatedSecondsAgo: "数秒前更新",
    systemHealthy: "系统健康", systemStatus: "系统状态", refreshing: "正在刷新", refreshSuccessTitle: "仪表盘已刷新", refreshSuccessDescription: "本地服务和资源快照已更新。",
    cards: { nginx: { title: "Nginx 状态", value: "运行中", description: "Nginx 1.26.2 正在运行", detail: "正在监听端口 80 和 443" }, cpu: { title: "CPU 使用率", description: "系统负载正常", detail: "8 核 · 平均 2.4 GHz" }, memory: { title: "内存使用率", description: "已使用 16 GB 中的 6.4 GB", detail: "可用 9.6 GB" }, services: { title: "活跃服务", description: "15 个服务中有 12 个正在运行", detail: "3 个服务已停止或需要关注" } },
    headerActions: { dashboardSettings: "仪表盘设置", exportDiagnostics: "导出诊断信息", openActions: "打开仪表盘操作", openSystemReport: "打开系统报告", mockDescription: "此仪表盘操作当前在模拟模式下运行。" },
    chart: { title: "系统资源使用率", description: "所选时间段内的 CPU 和内存使用率", cpu: "CPU 使用率", memory: "内存使用率", range: { "24h": "最近 24 小时", "7d": "最近 7 天", "30d": "最近 30 天" }, rangeLabel: "资源图表时间范围" },
    tabs: { services: "服务", events: "最近事件", changes: "配置变更", health: "健康检查", views: "服务管理视图" },
    columns: { actions: "操作", cpu: "CPU", memory: "内存", port: "端口", status: "状态", type: "类型", version: "版本" },
    columnSettings: { customize: "自定义列", visible: "可见列" },
    activity: { changes: "配置变更", events: "最近事件", health: "健康检查", status: { info: "信息", success: "成功", warning: "警告" }, timestamps: { attention: "需要关注", fiveMinutes: "5 分钟前", fortyTwoMinutes: "42 分钟前", oneHour: "1 小时前", eighteenMinutes: "18 分钟前", threeHours: "3 小时前", today1422: "今天 14:22", yesterday: "昨天", yesterday1008: "昨天 10:08", jul251840: "7 月 25 日 18:40", twoMinutes: "2 分钟前", healthy: "健康" }, titles: { nginxReloaded: "Nginx 配置已重新加载", redisThreshold: "Redis 内存阈值已达到", nodeRestarted: "Node API 已重启", certificateScan: "证书扫描已完成", postgresBackup: "PostgreSQL 备份已创建", portReleased: "端口 8080 已释放", resourceCheck: "系统资源检查", proxyAdded: "代理路由已添加", localGateway: "本地网关", certificateValidity: "证书有效期" }, descriptions: { nginxReloaded: "配置校验已成功完成。", redisThreshold: "内存使用率超过配置的 70% 阈值。", nodeRestarted: "开发进程在文件变更后已重启。", certificateScan: "有一个证书需要关注。", postgresBackup: "本地快照已成功保存。", portReleased: "上一个开发进程已正常退出。", resourceCheck: "CPU 和内存水平处于正常范围。", proxyAdded: "已为本地 Node 服务添加 /api 路由。", configNginx: "已更新 gzip 和 cache-control 指令。", configPostgres: "已将 max_connections 从 100 提升到 150。", configEnv: "已更新内部 API 基础 URL。", localGateway: "HTTP、TLS 和上游检查在 43 毫秒内通过。", certificateValidity: "一个证书将在未来 14 天内过期。" } },
  },
  services: {
    table: { actionsFor: "打开 {{name}} 的操作", select: "选择 {{name}}", selectAll: "选择所有服务", service: "服务" },
    status: { error: "错误", running: "运行中", stopped: "已停止", warning: "警告" },
    types: { application: "应用程序", cache: "缓存", database: "数据库", nginx: "Nginx", node: "Node", system: "系统" }, startupModes: { automatic: "自动", disabled: "禁用", manual: "手动" },
    actions: { editConfiguration: "编辑配置", openLogs: "打开日志", remove: "移除服务", restart: "重启", start: "启动", stop: "停止" },
    dialog: { add: "添加服务", title: "添加本地服务", description: "登记一个本地可执行文件以供管理；不会执行任何系统命令。", serviceName: "服务名称", serviceType: "服务类型", executablePath: "可执行文件路径", workingDirectory: "工作目录", startupMode: "启动模式", port: "端口", selectServiceType: "请选择服务类型", selectStartupMode: "请选择启动模式", optional: "可选" },
    confirmation: { removeTitle: "移除服务？", stopTitle: "停止服务？", removeDescription: "{{name}} 将从此本地仪表盘移除；其文件不会被删除。", stopDescription: "{{name}} 将停止接受本地流量，直到再次启动。" },
    toast: { added: "已添加 {{name}}", addedDescription: "该服务已在本地登记为停止状态。", localAction: "{{name}} 已准备好用于后续 Tauri 集成。", operationError: "无法{{operation}} {{name}}", operationSuccess: "{{name}} 已{{operation}}", operationSuccessDescription: "仪表盘模拟状态已成功更新。", removed: "已移除 {{name}}", removedDescription: "仅移除了本地仪表盘条目。", operations: { restart: "重启", start: "启动", stop: "停止" } },
    descriptions: { nginx: "本地反向代理和静态文件服务器", postgresql: "主要本地开发数据库", redis: "缓存和后台队列存储", nodeApi: "内部 REST API 开发进程", localWebApp: "Vite 前端预览服务器", certificateWatcher: "监控本地 TLS 证书到期时间", dockerEngine: "本地容器运行时和镜像服务", rabbitmq: "用于集成测试的本地消息代理", minio: "兼容 S3 的本地对象存储", prometheus: "指标采集和查询服务", grafana: "本地仪表盘与指标分析", mailpit: "本地邮件捕获与检查", sshAgent: "管理本地开发凭据", backgroundWorker: "处理排队的开发任务", fileSync: "同步共享的本地项目资产", custom: "{{startupMode}}服务 · {{path}}" },
  },
  settings: { title: "设置", appearance: { title: "外观", theme: "主题", light: "浅色", dark: "深色", system: "跟随系统", systemResolvedLight: "跟随系统", systemResolvedDark: "跟随系统" }, language: { title: "语言", label: "界面语言", zhCN: "简体中文", enUS: "English" }, storageDescription: "语言和主题设置保存在本机。", toast: { languageChanged: "界面语言已切换为{{language}}", themeChanged: "主题已切换为{{theme}}" }, labels: { changeLanguage: "切换界面语言", changeTheme: "切换主题" } },
  validation: { executablePathRequired: "请输入可执行文件路径。", invalidPort: "请输入 1 到 65535 之间的整数端口号。", serviceNameRequired: "请输入服务名称。", serviceTypeRequired: "请选择服务类型。", startupModeRequired: "请选择启动模式。", workingDirectoryRequired: "请输入工作目录。" },
} as const;

export default zhCN;
