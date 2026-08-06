export type NginxAuthorizationLevel = "readOnly" | "full";
export type NginxLifecycleState =
  | "available"
  | "changed"
  | "missing"
  | "uninstalled";
export type NginxRuntimeStatus = "running" | "stopped" | "conflict" | "unknown";
export type NginxControlBackend =
  | "portable"
  | "windowsScm"
  | "systemd"
  | "launchAgent"
  | "launchDaemonReadOnly"
  | "none";

export interface NginxCommandError {
  code: string;
  message: string;
}

export interface NginxDirectorySelection {
  selectionId: string;
  displayPath: string;
  expiresAt: string;
}

export interface NginxInspection {
  inspectionId: string;
  displayRoot: string;
  displayBinary: string;
  version: string;
  configureArguments: string[];
  configPath: string | null;
  binaryFingerprint: string;
  warnings: string[];
  expiresAt: string;
}

export interface NginxInstance {
  id: string;
  name: string;
  kind: string;
  rootPath: string;
  binaryPath: string;
  configPath: string | null;
  authorizedRoots: string[];
  authorizationLevel: NginxAuthorizationLevel;
  version: string;
  configureArguments: string[];
  binaryFingerprint: string;
  providerIdentity: { provider: string; externalId: string | null };
  controlBackend: NginxControlBackend;
  lifecycleState: NginxLifecycleState;
  runtimeStatus: NginxRuntimeStatus;
  capabilities: {
    canRead: boolean;
    canEdit: boolean;
    canControl: boolean;
    canUnregister: boolean;
  };
  createdAt: string;
  updatedAt: string;
}

export interface RegisterNginxInstanceInput {
  inspectionId: string;
  authorizationLevel: NginxAuthorizationLevel;
}

export type NginxRegistryState =
  | { status: "empty"; instance: null; migrationCandidates: [] }
  | { status: "ready"; instance: NginxInstance; migrationCandidates: [] }
  | {
    status: "migrationRequired";
    instance: null;
    migrationCandidates: NginxInstance[];
  };

export interface NginxRelease {
  version: string;
  downloadUrl: string;
  signatureUrl: string;
}

export interface NginxReleaseStatus {
  channel: "stable" | "mainline";
  latestRelease: NginxRelease | null;
  checkedAt: string | null;
  stale: boolean;
  source: "none" | "cache" | "network";
  updateAvailableCount: number;
  outdatedInstanceIds: string[];
}

export interface NginxSourceLocation {
  sourceId: string;
  line: number;
  column: number;
  endLine: number;
  endColumn: number;
}

export interface NginxDirective {
  name: string;
  arguments: string[];
  raw: string;
  location: NginxSourceLocation;
  children: NginxDirective[];
}

export interface NginxConfigSource {
  id: string;
  displayPath: string;
  text: string;
  directives: NginxDirective[];
}

export interface NginxConfigDiagnostic {
  code: string;
  severity: string;
  message: string;
  location: NginxSourceLocation | null;
}

export interface NginxSite {
  id: string;
  context: string;
  listens: string[];
  serverNames: string[];
  root: string | null;
  proxyPass: string[];
  locations: string[];
  source: NginxSourceLocation;
}

export interface NginxUpstream {
  name: string;
  servers: string[];
  source: NginxSourceLocation;
}

export interface NginxTopologyNode {
  id: string;
  kind: string;
  label: string;
}

export interface NginxTopologyEdge {
  from: string;
  to: string;
  label: string;
}

export interface NginxConfiguration {
  instanceId: string;
  entrySourceId: string;
  sources: NginxConfigSource[];
  diagnostics: NginxConfigDiagnostic[];
  sites: NginxSite[];
  upstreams: NginxUpstream[];
  topologyNodes: NginxTopologyNode[];
  topologyEdges: NginxTopologyEdge[];
  pidPath: string | null;
  accessLogs: string[];
  errorLogs: string[];
}

export type NginxControlAction = "start" | "stop" | "reload" | "restart";

export interface NginxOperationRecord {
  id: string;
  instanceId: string;
  action: NginxControlAction;
  backend: NginxControlBackend;
  startedAt: string;
  completedAt: string;
  success: boolean;
  outcome: "executed" | "noop";
  resultingStatus: NginxRuntimeStatus;
  errorCode: string | null;
  stdout: string;
  stderr: string;
}

export type NginxProcessRole = "master" | "worker";
export type NginxRuntimeMetricAvailability = "available" | "unavailable";

export interface NginxRuntimeProcess {
  pid: number;
  parentPid: number | null;
  role: NginxProcessRole;
  cpuUsage: number;
  memoryBytes: number;
  startedAt: string | null;
  uptimeSeconds: number;
  executableVerified: boolean;
}

export interface NginxRuntimeDetails {
  instanceId: string;
  observedAt: string;
  status: NginxRuntimeStatus;
  masterPid: number | null;
  workerCount: number;
  totalCpuUsage: number;
  totalMemoryBytes: number;
  startedAt: string | null;
  uptimeSeconds: number | null;
  processes: NginxRuntimeProcess[];
  processMetrics: NginxRuntimeMetricAvailability;
  listeners: string[];
  listenerMetrics: NginxRuntimeMetricAvailability;
  connectionMetrics: NginxRuntimeMetricAvailability;
}

export type NginxOperationPhase =
  | "starting"
  | "stopping"
  | "reloading"
  | "restarting"
  | "downloading"
  | "verifying"
  | "backingUp"
  | "replacing"
  | "restoringRuntime"
  | "rollingBack";

export interface NginxStatusEvent {
  generation: number;
  sequence: number;
  observedAt: string;
  instance: NginxInstance | null;
  runtimeDetails: NginxRuntimeDetails | null;
  operationPhase: NginxOperationPhase | null;
}

export interface NginxStatusSubscription {
  subscriptionId: number;
  initialEvent: NginxStatusEvent;
}

export interface NginxUpgradeProgress {
  phase: NginxOperationPhase;
  progress: number;
  messageCode: string;
}

export interface NginxUpgradeResult {
  fromVersion: string;
  targetVersion: string;
  backupId: string | null;
  success: boolean;
  rolledBack: boolean;
  rollbackSucceeded: boolean | null;
  errorCode: string | null;
  resultingStatus: NginxRuntimeStatus;
}
