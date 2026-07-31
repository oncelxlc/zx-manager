export type NginxAuthorizationLevel = "readOnly" | "full";
export type NginxLifecycleState =
  | "available"
  | "changed"
  | "missing"
  | "uninstalled";
export type NginxRuntimeStatus = "running" | "stopped" | "unknown";
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
  name: string;
  authorizationLevel: NginxAuthorizationLevel;
}
