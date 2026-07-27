export type ServiceStatus = "running" | "stopped" | "warning" | "error";

export type ServiceType =
  | "nginx"
  | "database"
  | "cache"
  | "node"
  | "application"
  | "system";

export type StartupMode = "automatic" | "manual" | "disabled";

export interface LocalService {
  id: string;
  name: string;
  descriptionKey: string;
  descriptionValues?: Record<string, string | number>;
  type: ServiceType;
  status: ServiceStatus;
  version: string;
  port?: number;
  cpu: number;
  memory: string;
}

export interface AddServiceInput {
  name: string;
  type: ServiceType;
  executablePath: string;
  workingDirectory: string;
  startupMode: StartupMode;
  port?: number;
}

export interface ResourceMetric {
  date: string;
  cpu: number;
  memory: number;
}

export type ResourceRange = "24h" | "7d" | "30d";

export type ServiceColumn =
  | "type"
  | "status"
  | "version"
  | "port"
  | "cpu"
  | "memory"
  | "actions";

export type ServiceOperation = "start" | "stop" | "restart";

export interface ActivityRecord {
  id: string;
  titleKey: string;
  descriptionKey: string;
  timestampKey: string;
  status: "info" | "success" | "warning";
}
