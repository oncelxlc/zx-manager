export interface RemoteFile {
  name: string;
  size: string;
  mode: string;
  modified: string;
  children?: RemoteFile[];
}

interface DockerResource {
  type: "Containers" | "Images" | "Volumes" | "Networks";
  name: string;
  image: string;
  status: string;
  ports: string;
  created: string;
}

interface Deployment {
  version: string;
  commit: string;
  at: string;
  status: "完成" | "失败";
  log: string[];
}

export interface Server {
  id: string;
  name: string;
  os: string;
  cwd: string;
  cpu: string;
  memory: string;
  disk: string;
  uptime: string;
  sampledAt: string;
  files: RemoteFile[];
  docker: DockerResource[];
  deployments: Deployment[];
}

function files(entries: [string, string][]): RemoteFile[] {
  return entries.map(([name, size]) => ({ name, size, mode: "-rw-r--r--", modified: "2026-11-12 14:15" }));
}

function docker(service: string, port: string): DockerResource[] {
  const created = "2026-11-12";
  const image = service + ":1.4";
  return [
    { type: "Containers", name: service, image, status: "运行中", ports: port, created },
    { type: "Images", name: image, image, status: "可用", ports: "—", created },
    { type: "Volumes", name: service + "-data", image: "—", status: "已挂载", ports: "—", created },
    { type: "Networks", name: service + "-bridge", image: "—", status: "可用", ports: "—", created },
  ];
}

export const servers: Server[] = [
  {
    id: "staging", name: "staging-01", os: "Ubuntu 24.04 LTS", cwd: "/srv/workbench",
    cpu: "12% · 4 核", memory: "2.1 / 8 GB", disk: "34 / 120 GB", uptime: "12 天 6 小时", sampledAt: "2026-11-13 09:40 UTC",
    files: [{ name: "app", size: "—", mode: "drwxr-xr-x", modified: "2026-11-12 14:20",
      children: files([["server.js", "4.8 KB"], ["manifest.json", "846 B"]]) }, ...files([["compose.yml", "1.2 KB"], ["release.txt", "240 B"]])],
    docker: docker("staging-api", "8080 → 3000"),
    deployments: [
      { version: "v1.4.0", commit: "ac83d10", at: "2026-11-12 14:20 UTC", status: "完成",
        log: ["读取 staging 清单", "装载 staging-api:1.4", "健康检查通过"] },
      { version: "v1.4.0-rc1", commit: "d982bfa", at: "2026-11-11 10:18 UTC", status: "失败",
        log: ["装载候选镜像", "健康检查超时", "已回退到上一演示版本"] },
    ],
  },
  {
    id: "build", name: "build-02", os: "Debian 12", cwd: "/opt/runner",
    cpu: "36% · 8 核", memory: "6.4 / 16 GB", disk: "88 / 240 GB", uptime: "6 天 2 小时", sampledAt: "2026-11-13 09:35 UTC",
    files: [{ name: "artifacts", size: "—", mode: "drwxr-xr-x", modified: "2026-11-12 11:30",
      children: files([["build.log", "8.1 KB"], ["checksums.txt", "298 B"]]) }, ...files([["runner.toml", "920 B"]])],
    docker: docker("build-agent", "9090 → 9090"),
    deployments: [{ version: "v0.9.3", commit: "bf218aa", at: "2026-11-12 11:30 UTC", status: "完成",
      log: ["加载构建节点清单", "替换 build-agent 服务", "构建任务队列就绪"] }],
  },
];
