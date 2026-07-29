import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

import type {
  CommandError,
  SystemInformation,
  SystemSummary,
} from "src/types/system-information";

export function getSystemSummary(): Promise<SystemSummary> {
  return invoke<SystemSummary>("get_system_summary");
}

export function getSystemInformation(): Promise<SystemInformation> {
  return invoke<SystemInformation>("get_system_information");
}

export function writeDiagnosticText(value: string): Promise<void> {
  return writeText(value);
}

export function toCommandError(error: unknown): CommandError {
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
    code: "unknown",
    message: error instanceof Error ? error.message : String(error),
  };
}
