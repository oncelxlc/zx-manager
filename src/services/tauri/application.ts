import { relaunch } from "@tauri-apps/plugin-process";

export function restartApplication(): Promise<void> {
  return relaunch();
}
