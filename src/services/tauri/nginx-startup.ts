import { useNginxReleaseStore } from "src/stores/nginx-release-store";
import { useNginxStore } from "src/stores/nginx-store";
import { defaultNginxPreferences } from "src/types/preferences";
import type { UserPreferences } from "src/types/preferences";

export async function restoreNginxOnStartup(
  preferences: Partial<UserPreferences>,
): Promise<void> {
  const nginxPreferences = preferences.nginx ?? defaultNginxPreferences;
  useNginxReleaseStore.getState().configure(nginxPreferences);
  await Promise.allSettled([
    useNginxStore.getState().loadRegistry(true),
    useNginxStore.getState().ensureStatusSubscription(),
    useNginxReleaseStore.getState().check(false),
  ]);
}
