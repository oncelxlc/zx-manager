import { invoke, isTauri } from "@tauri-apps/api/core";

import type {
  NginxCommandError,
  NginxDirectorySelection,
  NginxInspection,
  NginxInstance,
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

export function listNginxInstances(): Promise<NginxInstance[]> {
  assertNginxDesktop();
  return invoke("list_nginx_instances");
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
