import type { ActivityRecord, LocalService, ResourceMetric, ResourceRange } from "src/types/service";

const service = (
  id: string,
  name: string,
  descriptionKey: string,
  type: LocalService["type"],
  status: LocalService["status"],
  version: string,
  port: number | undefined,
  cpu: number,
  memory: string,
): LocalService => ({ id, name, descriptionKey, type, status, version, port, cpu, memory });

export const initialServices: LocalService[] = [
  service("nginx", "Nginx", "nginx", "nginx", "running", "1.26.2", 80, 1.8, "42 MB"),
  service("postgresql", "PostgreSQL", "postgresql", "database", "running", "16.4", 5432, 4.6, "612 MB"),
  service("redis", "Redis", "redis", "cache", "warning", "7.4.0", 6379, 2.2, "128 MB"),
  service("node-api", "Node API", "nodeApi", "node", "running", "22.12.0", 3001, 8.4, "284 MB"),
  service("local-web-app", "Local Web App", "localWebApp", "application", "stopped", "0.9.4", 1420, 0, "0 MB"),
  service("certificate-watcher", "Certificate Watcher", "certificateWatcher", "system", "error", "2.1.3", undefined, 0, "0 MB"),
  service("docker-engine", "Docker Engine", "dockerEngine", "system", "running", "27.3.1", 2375, 3.7, "428 MB"),
  service("rabbitmq", "RabbitMQ", "rabbitmq", "application", "running", "4.0.2", 5672, 2.1, "196 MB"),
  service("minio", "MinIO", "minio", "application", "running", "2026.07", 9000, 1.4, "172 MB"),
  service("prometheus", "Prometheus", "prometheus", "system", "running", "3.4.1", 9090, 3.1, "318 MB"),
  service("grafana", "Grafana", "grafana", "application", "running", "12.1.0", 3000, 2.8, "246 MB"),
  service("mailpit", "Mailpit", "mailpit", "application", "running", "1.27.4", 8025, 0.6, "34 MB"),
  service("ssh-agent", "SSH Agent", "sshAgent", "system", "running", "Windows", undefined, 0.2, "18 MB"),
  service("background-worker", "Background Worker", "backgroundWorker", "node", "running", "2.8.0", 3010, 4.2, "212 MB"),
  service("file-sync", "File Sync", "fileSync", "system", "running", "1.5.6", undefined, 1.1, "76 MB"),
];

function metricSeries(start: string, count: number, stepHours: number, cpu: number[], memory: number[]): ResourceMetric[] {
  const startTime = new Date(start).getTime();
  return Array.from({ length: count }, (_, index) => ({
    date: new Date(startTime + index * stepHours * 3_600_000).toISOString(),
    cpu: cpu[index % cpu.length],
    memory: memory[index % memory.length],
  }));
}

export const resourceMetrics: Record<ResourceRange, ResourceMetric[]> = {
  "24h": metricSeries("2026-07-27T00:00:00", 12, 2, [18, 14, 12, 20, 32, 46, 38, 52, 41, 35, 29, 25], [36, 35, 34, 36, 39, 43, 42, 45]),
  "7d": metricSeries("2026-07-21T12:00:00", 7, 24, [31, 44, 36, 53, 47, 28, 25], [42, 46, 44, 49, 45, 41]),
  "30d": metricSeries("2026-06-28T12:00:00", 11, 3 * 24, [29, 34, 46, 38, 51, 43, 49, 32, 27, 25], [39, 42, 46, 43, 48, 45, 44]),
};

const activity = (id: string, titleKey: string, descriptionKey: string, timestampKey: string, status: ActivityRecord["status"]): ActivityRecord => ({ id, titleKey, descriptionKey, timestampKey, status });

export const recentEvents = [
  activity("event-1", "nginxReloaded", "nginxReloaded", "twoMinutes", "success"),
  activity("event-2", "redisThreshold", "redisThreshold", "eighteenMinutes", "warning"),
  activity("event-3", "nodeRestarted", "nodeRestarted", "fortyTwoMinutes", "info"),
  activity("event-4", "certificateScan", "certificateScan", "oneHour", "warning"),
  activity("event-5", "postgresBackup", "postgresBackup", "threeHours", "success"),
  activity("event-6", "portReleased", "portReleased", "fiveMinutes", "info"),
  activity("event-7", "resourceCheck", "resourceCheck", "yesterday", "success"),
  activity("event-8", "proxyAdded", "proxyAdded", "yesterday", "info"),
];

export const configurationChanges = [
  activity("change-1", "nginx.conf", "configNginx", "today1422", "success"),
  activity("change-2", "postgresql.conf", "configPostgres", "yesterday1008", "info"),
  activity("change-3", ".env.local", "configEnv", "jul251840", "warning"),
];

export const healthChecks = [
  activity("health-1", "localGateway", "localGateway", "healthy", "success"),
  activity("health-2", "certificateValidity", "certificateValidity", "attention", "warning"),
];
