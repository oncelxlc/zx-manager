import { invoke } from "@tauri-apps/api/core";
import { islandApi } from "./features/island/islandApi";
import type { IslandConfig } from "./features/island/types";

interface MainWindowSession {
  dependenciesExpanded: boolean;
  draft: IslandConfig | null;
}

declare global {
  interface Window {
    __zxMainSession?: MainWindowSession;
  }
}

export const mainWindowSession: MainWindowSession = { dependenciesExpanded: true, draft: null };

export async function restoreMainWindow() {
  if (!islandApi.isDesktop()) return;
  try {
    const snapshot = await invoke<(MainWindowSession & { route: string }) | null>("main_window_restore");
    if (snapshot) {
      mainWindowSession.dependenciesExpanded = snapshot.dependenciesExpanded;
      mainWindowSession.draft = snapshot.draft;
      window.history.replaceState(null, "", snapshot.route);
    }
  } finally {
    window.__zxMainSession = mainWindowSession;
  }
}
