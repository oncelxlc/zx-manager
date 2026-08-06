import { Channel, invoke, isTauri } from "@tauri-apps/api/core";

import type {
  NginxCommandError,
  NginxConfigGraph,
  NginxConfigNodeDetail,
  NginxConfigValidationResult,
  NginxGlobalConfigApplyMode,
  NginxGlobalConfigApplyResult,
  NginxGlobalConfigPatchValidation,
  NginxGlobalConfiguration,
  NginxGlobalConfigurationPatch,
  NginxLogEvent,
  NginxLogPage,
  NginxLogSource,
  NginxLogSubscription,
  NginxConfiguration,
  NginxControlAction,
  NginxDirectorySelection,
  NginxInspection,
  NginxInstance,
  NginxReleaseStatus,
  NginxOperationRecord,
  NginxRegistryState,
  NginxRuntimeDetails,
  NginxStatusEvent,
  NginxStatusSubscription,
  NginxUpgradeProgress,
  NginxUpgradeResult,
  RegisterNginxInstanceInput,
} from "src/types/nginx";

export async function selectNginxDirectory(
  purpose: "inspectInstance" | "authorizeAdditionalRoot",
): Promise<NginxDirectorySelection | null> {
  assertNginxDesktop();
  return invoke("select_nginx_directory", { purpose });
}

export function inspectNginxDirectory(
  selectionId: string,
): Promise<NginxInspection> {
  assertNginxDesktop();
  return invoke("inspect_nginx_directory", { selectionId });
}

export function registerNginxInstance(
  input: RegisterNginxInstanceInput,
): Promise<NginxInstance> {
  assertNginxDesktop();
  return invoke("register_nginx_instance", { input });
}

export function getNginxRegistryState(): Promise<NginxRegistryState> {
  assertNginxDesktop();
  return invoke("get_nginx_registry_state");
}

export function resolveNginxRegistryMigration(
  keepInstanceId: string,
): Promise<NginxRegistryState> {
  assertNginxDesktop();
  return invoke("resolve_nginx_registry_migration", {
    input: { keepInstanceId },
  });
}

export function refreshNginxInstance(
  instanceId: string,
): Promise<NginxInstance> {
  assertNginxDesktop();
  return invoke("refresh_nginx_instance", { instanceId });
}

export async function unregisterNginxInstance(
  instanceId: string,
): Promise<void> {
  assertNginxDesktop();
  await invoke("unregister_nginx_instance", { instanceId });
}

export async function authorizeNginxInstanceRoot(
  instanceId: string,
): Promise<NginxInstance | null> {
  const selection = await selectNginxDirectory("authorizeAdditionalRoot");
  if (!selection) {
    return null;
  }
  return invoke("authorize_nginx_instance_root", {
    input: { instanceId, selectionId: selection.selectionId },
  });
}

export function getNginxReleaseStatus(
  channel: "stable" | "mainline",
): Promise<NginxReleaseStatus> {
  assertNginxDesktop();
  return invoke("get_nginx_release_status", { channel });
}

export function checkNginxUpdates(
  channel: "stable" | "mainline",
  force: boolean,
  maxAgeHours: number,
): Promise<NginxReleaseStatus> {
  assertNginxDesktop();
  return invoke("check_nginx_updates", {
    input: { channel, force, maxAgeHours },
  });
}

export function getNginxConfiguration(
  instanceId: string,
): Promise<NginxConfiguration> {
  assertNginxDesktop();
  return invoke("get_nginx_configuration", { instanceId });
}

export function readNginxConfigGraph(
  instanceId: string,
): Promise<NginxConfigGraph> {
  assertNginxDesktop();
  return invoke("read_nginx_config_graph", { instanceId });
}

export function readNginxConfigNode(
  instanceId: string,
  nodeId: string,
): Promise<NginxConfigNodeDetail> {
  assertNginxDesktop();
  return invoke("read_nginx_config_node", { instanceId, nodeId });
}

export function validateNginxConfiguration(
  instanceId: string,
): Promise<NginxConfigValidationResult> {
  assertNginxDesktop();
  return invoke("validate_nginx_configuration", { instanceId });
}

export function getNginxGlobalConfiguration(
  instanceId: string,
): Promise<NginxGlobalConfiguration> {
  assertNginxDesktop();
  return invoke("get_nginx_global_configuration", { instanceId });
}

export function validateNginxGlobalConfigurationPatch(
  instanceId: string,
  expectedRevision: string,
  patch: NginxGlobalConfigurationPatch,
): Promise<NginxGlobalConfigPatchValidation> {
  assertNginxDesktop();
  return invoke("validate_nginx_global_configuration_patch", {
    input: { instanceId, expectedRevision, patch },
  });
}

export function applyNginxGlobalConfigurationPatch(
  instanceId: string,
  expectedRevision: string,
  patch: NginxGlobalConfigurationPatch,
  mode: NginxGlobalConfigApplyMode,
): Promise<NginxGlobalConfigApplyResult> {
  assertNginxDesktop();
  return invoke("apply_nginx_global_configuration_patch", {
    input: { instanceId, expectedRevision, patch, mode },
  });
}

export function listNginxLogSources(instanceId: string): Promise<NginxLogSource[]> {
  assertNginxDesktop();
  return invoke("list_nginx_log_sources", { instanceId });
}

export function readNginxLogPage(
  instanceId: string,
  sourceId: string,
  cursor: string | null,
): Promise<NginxLogPage> {
  assertNginxDesktop();
  return invoke("read_nginx_log_page", { input: { instanceId, sourceId, cursor } });
}

export async function subscribeNginxLog(
  instanceId: string,
  sourceId: string,
  onMessage: (event: NginxLogEvent) => void,
) {
  assertNginxDesktop();
  const channel = new Channel<NginxLogEvent>();
  channel.onmessage = onMessage;
  const subscription = await invoke<NginxLogSubscription>("subscribe_nginx_log", {
    instanceId,
    sourceId,
    channel,
  });
  let cleaned = false;
  return {
    subscription,
    cleanup: async () => {
      if (cleaned) return;
      cleaned = true;
      channel.onmessage = () => undefined;
      await invoke("unsubscribe_nginx_log", { subscriptionId: subscription.subscriptionId });
    },
  };
}

export function getNginxRuntimeDetails(
  instanceId: string,
): Promise<NginxRuntimeDetails> {
  assertNginxDesktop();
  return invoke("get_nginx_runtime_details", { instanceId });
}

export function controlNginxInstance(
  instanceId: string,
  action: NginxControlAction,
): Promise<NginxOperationRecord> {
  assertNginxDesktop();
  return invoke("control_nginx_instance", { input: { instanceId, action } });
}

export interface NginxStatusChannelSubscription {
  subscription: NginxStatusSubscription;
  cleanup: () => Promise<void>;
}

export async function subscribeNginxStatus(
  onMessage: (event: NginxStatusEvent) => void,
): Promise<NginxStatusChannelSubscription> {
  assertNginxDesktop();
  const channel = new Channel<NginxStatusEvent>();
  channel.onmessage = onMessage;
  const subscription = await invoke<NginxStatusSubscription>(
    "subscribe_nginx_status",
    { channel },
  );
  let cleanedUp = false;
  return {
    subscription,
    cleanup: async () => {
      if (cleanedUp) return;
      cleanedUp = true;
      channel.onmessage = () => undefined;
      await invoke("unsubscribe_nginx_status", {
        subscriptionId: subscription.subscriptionId,
      });
    },
  };
}

export function upgradeNginxInstance(
  input: {
    instanceId: string;
    channel: "stable" | "mainline";
    targetVersion: string;
    backupRetentionCount: number;
  },
  onProgress: (progress: NginxUpgradeProgress) => void,
): Promise<NginxUpgradeResult> {
  assertNginxDesktop();
  const progressChannel = new Channel<NginxUpgradeProgress>();
  progressChannel.onmessage = onProgress;
  return invoke("upgrade_nginx_instance", { input, progressChannel });
}

export function getNginxOperationHistory(
  instanceId: string | null,
  limit = 100,
): Promise<NginxOperationRecord[]> {
  assertNginxDesktop();
  return invoke("get_nginx_operation_history", {
    input: { instanceId, limit },
  });
}

export function toNginxCommandError(error: unknown): NginxCommandError {
  if (
    typeof error === "object"
    && error !== null
    && "code" in error
    && typeof error.code === "string"
  ) {
    return {
      code: error.code,
      message:
        "message" in error && typeof error.message === "string"
          ? error.message
          : "",
    };
  }
  return {
    code: "NGINX_UNKNOWN",
    message: error instanceof Error ? error.message : String(error),
  };
}

function assertNginxDesktop() {
  if (!isTauri()) {
    throw {
      code: "NGINX_DESKTOP_REQUIRED",
      message: "Nginx management requires the Tauri desktop runtime.",
    } satisfies NginxCommandError;
  }
}
