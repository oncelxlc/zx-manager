export type SystemWarningCode =
  | "cpuUsageUnavailable"
  | "disksUnavailable"
  | "gpuUnavailable"
  | "runtimeTargetUnavailable"
  | "unsupportedSystem"
  | "webviewVersionUnavailable";

export type GpuBackend =
  | "vulkan"
  | "metal"
  | "dx12"
  | "gl"
  | "browser-webgpu"
  | "unknown";

export type GpuDeviceType =
  | "integrated"
  | "discrete"
  | "virtual"
  | "cpu"
  | "other"
  | "unknown";

export interface CommandError {
  code: string;
  message: string;
}

export interface SystemSummary {
  osName: string | null;
  osVersion: string | null;
  osLongVersion: string | null;
  architecture: string | null;
  hostName: string | null;
  cpuModel: string | null;
  logicalCoreCount: number | null;
  totalMemoryBytes: number | null;
  collectedAt: string;
  warnings: SystemWarningCode[];
}

export interface SystemDetails {
  platform: "windows" | "macos" | "linux" | "unknown";
  osName: string | null;
  osVersion: string | null;
  osLongVersion: string | null;
  kernelVersion: string | null;
  architecture: string | null;
  hostName: string | null;
  uptimeSeconds: number | null;
}

export interface CpuInformation {
  model: string | null;
  vendor: string | null;
  physicalCoreCount: number | null;
  logicalCoreCount: number | null;
  frequencyMhz: number | null;
  usagePercent: number | null;
}

export interface MemoryInformation {
  totalBytes: number | null;
  usedBytes: number | null;
  availableBytes: number | null;
  usagePercent: number | null;
  swapTotalBytes: number | null;
  swapUsedBytes: number | null;
  swapUsagePercent: number | null;
}

export interface GpuInformation {
  name: string;
  vendorId: number | null;
  deviceId: number | null;
  deviceType: GpuDeviceType;
  backend: GpuBackend;
  driver: string | null;
  driverInfo: string | null;
  dedicatedMemoryBytes: number | null;
  sharedMemoryBytes: number | null;
}

export interface DiskInformation {
  name: string | null;
  fileSystem: string | null;
  mountPoint: string | null;
  kind: string;
  removable: boolean;
  totalBytes: number | null;
  availableBytes: number | null;
  usedBytes: number | null;
  usagePercent: number | null;
}

export interface RuntimeInformation {
  appName: string;
  appVersion: string;
  tauriVersion: string;
  targetTriple: string | null;
  webviewVersion: string | null;
}

export interface DataAvailability {
  cpuUsage: boolean;
  memory: boolean;
  swap: boolean;
  disks: boolean;
  gpuBasic: boolean;
  gpuMemory: boolean;
  runtimeTarget: boolean;
  webviewVersion: boolean;
}

export interface SystemInformation {
  summary: SystemSummary;
  system: SystemDetails;
  cpu: CpuInformation;
  memory: MemoryInformation;
  gpus: GpuInformation[];
  disks: DiskInformation[];
  runtime: RuntimeInformation;
  availability: DataAvailability;
  collectedAt: string;
  warnings: SystemWarningCode[];
}
