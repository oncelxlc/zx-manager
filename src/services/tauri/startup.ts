import { invoke, isTauri } from "@tauri-apps/api/core";

let completionPromise: Promise<void> | null = null;

export function completeStartup(): Promise<void> {
  if (!isTauri()) {
    return Promise.resolve();
  }

  if (!completionPromise) {
    completionPromise = invoke<void>("complete_startup").catch((error) => {
      completionPromise = null;
      throw error;
    });
  }

  return completionPromise;
}
