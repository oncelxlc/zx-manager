import { load, type Store } from "@tauri-apps/plugin-store";

import {
  isNetworkMonitorSampleInterval,
  isSupportedLocale,
  isThemeMode,
  normalizeNginxPreferences,
  type NginxPreferences,
  type NetworkMonitorSampleInterval,
  type SupportedLocale,
  type ThemeMode,
  type UserPreferences,
} from "src/types/preferences";

const preferencesStorageKey = "local-console.preferences";
const legacyThemeStorageKey = "vite-ui-theme";
const storeFileName = "preferences.json";
const storePreferencesKey = "preferences";

interface StoredPreferencesV1 {
  version: 1;
  locale?: SupportedLocale;
  theme?: ThemeMode;
}

interface StoredPreferencesV2 {
  version: 2;
  locale?: SupportedLocale;
  theme?: ThemeMode;
  networkMonitorConfigured?: boolean;
  networkMonitorStartOnLaunch?: boolean;
}

interface StoredPreferencesV5 {
  version: 5;
  locale?: SupportedLocale;
  theme?: ThemeMode;
  networkMonitorConfigured?: boolean;
  networkMonitorStartOnLaunch?: boolean;
  networkMonitorSampleIntervalSeconds?: NetworkMonitorSampleInterval;
}

interface StoredPreferencesV6 extends Omit<StoredPreferencesV5, "version"> {
  version: 6;
  nginx?: Partial<NginxPreferences>;
}

interface StoredPreferencesV4 extends Omit<StoredPreferencesV5, "version"> {
  version: 4;
  networkMonitorStartOnLaunch?: never;
}

interface StoredPreferencesV3 extends Omit<StoredPreferencesV5, "version"> {
  version: 3;
}

type PreferencesStore = Pick<Store, "get" | "save" | "set">;

let storePromise: Promise<PreferencesStore> | null | undefined;

function canUseStorage() {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function normalizePreferences(value: unknown): Partial<UserPreferences> {
  if (typeof value !== "object" || value === null) {
    return {};
  }

  const preferences = value as
    | StoredPreferencesV1
    | StoredPreferencesV2
    | StoredPreferencesV3
    | StoredPreferencesV4
    | StoredPreferencesV5
    | StoredPreferencesV6;
  if (
    preferences.version !== 1
    && preferences.version !== 2
    && preferences.version !== 3
    && preferences.version !== 4
    && preferences.version !== 5
    && preferences.version !== 6
  ) {
    return {};
  }

  return {
    locale: isSupportedLocale(preferences.locale) ? preferences.locale : undefined,
    theme: isThemeMode(preferences.theme) ? preferences.theme : undefined,
    networkMonitorConfigured:
      preferences.version !== 1
      && typeof preferences.networkMonitorConfigured === "boolean"
        ? preferences.networkMonitorConfigured
        : undefined,
    networkMonitorStartOnLaunch:
      (preferences.version === 2
        || preferences.version === 3
        || preferences.version === 5
        || preferences.version === 6)
      && typeof preferences.networkMonitorStartOnLaunch === "boolean"
        ? preferences.networkMonitorStartOnLaunch
        : true,
    networkMonitorSampleIntervalSeconds:
      (preferences.version === 3
        || preferences.version === 4
        || preferences.version === 5
        || preferences.version === 6)
      && isNetworkMonitorSampleInterval(
        preferences.networkMonitorSampleIntervalSeconds,
      )
        ? preferences.networkMonitorSampleIntervalSeconds
        : 5,
    nginx: normalizeNginxPreferences(
      preferences.version === 6 ? preferences.nginx : undefined,
    ),
  };
}

function readLegacyPreferences(): Partial<UserPreferences> {
  if (!canUseStorage()) {
    return {};
  }

  try {
    const rawValue = window.localStorage.getItem(preferencesStorageKey);
    const storedPreferences = rawValue
      ? normalizePreferences(JSON.parse(rawValue) as unknown)
      : {};
    const legacyTheme = window.localStorage.getItem(legacyThemeStorageKey);

    return {
      ...storedPreferences,
      theme: storedPreferences.theme ?? (isThemeMode(legacyTheme) ? legacyTheme : undefined),
    };
  } catch {
    return {};
  }
}

function writeLegacyPreferences(preferences: Partial<UserPreferences>) {
  if (!canUseStorage()) {
    return;
  }

  try {
    const current = readLegacyPreferences();
    const nextValue: StoredPreferencesV6 = {
      version: 6,
      ...current,
      ...preferences,
      networkMonitorConfigured:
        preferences.networkMonitorConfigured
        ?? current.networkMonitorConfigured
        ?? false,
      networkMonitorStartOnLaunch:
        preferences.networkMonitorStartOnLaunch
        ?? current.networkMonitorStartOnLaunch
        ?? true,
      networkMonitorSampleIntervalSeconds:
        preferences.networkMonitorSampleIntervalSeconds
        ?? current.networkMonitorSampleIntervalSeconds
        ?? 5,
      nginx: normalizeNginxPreferences(preferences.nginx ?? current.nginx),
    };
    window.localStorage.setItem(preferencesStorageKey, JSON.stringify(nextValue));
  } catch {
    // Preferences are optional; a blocked storage write must not interrupt the UI.
  }
}

function clearLegacyPreferences() {
  if (!canUseStorage()) {
    return;
  }

  try {
    window.localStorage.removeItem(preferencesStorageKey);
    window.localStorage.removeItem(legacyThemeStorageKey);
  } catch {
    // The Store write already succeeded; a blocked legacy cleanup is harmless.
  }
}

function hasPreferences(preferences: Partial<UserPreferences>) {
  return preferences.locale !== undefined
    || preferences.theme !== undefined
    || preferences.networkMonitorConfigured !== undefined
    || preferences.networkMonitorStartOnLaunch !== undefined
    || preferences.networkMonitorSampleIntervalSeconds !== undefined
    || preferences.nginx !== undefined;
}

function mergePreferences(
  primary: Partial<UserPreferences>,
  fallback: Partial<UserPreferences>,
): Partial<UserPreferences> {
  return {
    locale: primary.locale ?? fallback.locale,
    theme: primary.theme ?? fallback.theme,
    networkMonitorConfigured:
      primary.networkMonitorConfigured ?? fallback.networkMonitorConfigured,
    networkMonitorStartOnLaunch:
      primary.networkMonitorStartOnLaunch
      ?? fallback.networkMonitorStartOnLaunch
      ?? true,
    networkMonitorSampleIntervalSeconds:
      primary.networkMonitorSampleIntervalSeconds
      ?? fallback.networkMonitorSampleIntervalSeconds
      ?? 5,
    nginx: normalizeNginxPreferences(primary.nginx ?? fallback.nginx),
  };
}

function preferencesMatch(
  left: Partial<UserPreferences>,
  right: Partial<UserPreferences>,
) {
  return left.locale === right.locale
    && left.theme === right.theme
    && left.networkMonitorConfigured === right.networkMonitorConfigured
    && left.networkMonitorStartOnLaunch === right.networkMonitorStartOnLaunch
    && left.networkMonitorSampleIntervalSeconds
      === right.networkMonitorSampleIntervalSeconds
    && JSON.stringify(left.nginx) === JSON.stringify(right.nginx);
}

async function getStore(): Promise<PreferencesStore | null> {
  if (storePromise === null) {
    return null;
  }

  if (!storePromise) {
    storePromise = load(storeFileName, { autoSave: false });
  }

  try {
    return await storePromise;
  } catch {
    storePromise = null;
    return null;
  }
}

async function savePreferences(
  store: PreferencesStore,
  preferences: Partial<UserPreferences>,
) {
  const value: StoredPreferencesV6 = {
    version: 6,
    ...preferences,
    networkMonitorConfigured: preferences.networkMonitorConfigured ?? false,
    networkMonitorStartOnLaunch:
      preferences.networkMonitorStartOnLaunch ?? true,
    networkMonitorSampleIntervalSeconds:
      preferences.networkMonitorSampleIntervalSeconds ?? 5,
    nginx: normalizeNginxPreferences(preferences.nginx),
  };
  await store.set(storePreferencesKey, value);
  await store.save();
}

export async function getPreferences(): Promise<Partial<UserPreferences>> {
  const store = await getStore();
  const legacyPreferences = readLegacyPreferences();

  if (!store) {
    return legacyPreferences;
  }

  try {
    const storedPreferences = normalizePreferences(
      await store.get<unknown>(storePreferencesKey),
    );
    const preferences = mergePreferences(storedPreferences, legacyPreferences);

    if (hasPreferences(legacyPreferences) && !preferencesMatch(preferences, storedPreferences)) {
      await savePreferences(store, preferences);
      clearLegacyPreferences();
    }

    return preferences;
  } catch {
    return legacyPreferences;
  }
}

export async function setLocale(locale: SupportedLocale): Promise<void> {
  await updatePreferences({ locale });
}

export async function setTheme(theme: ThemeMode): Promise<void> {
  await updatePreferences({ theme });
}

export async function setNetworkMonitorConfigured(configured: boolean): Promise<void> {
  await updatePreferences({ networkMonitorConfigured: configured });
}

export async function setNetworkMonitorStartOnLaunch(
  startOnLaunch: boolean,
): Promise<void> {
  await updatePreferences({ networkMonitorStartOnLaunch: startOnLaunch });
}

export async function setNetworkMonitorSampleInterval(
  interval: NetworkMonitorSampleInterval,
): Promise<void> {
  await updatePreferences({ networkMonitorSampleIntervalSeconds: interval });
}

export async function setNginxPreferences(
  nginx: Partial<NginxPreferences>,
): Promise<void> {
  const current = await getPreferences();
  await updatePreferences({
    nginx: normalizeNginxPreferences({ ...current.nginx, ...nginx }),
  });
}

async function updatePreferences(preferences: Partial<UserPreferences>) {
  const store = await getStore();
  if (!store) {
    writeLegacyPreferences(preferences);
    return;
  }

  try {
    const storedPreferences = normalizePreferences(
      await store.get<unknown>(storePreferencesKey),
    );
    await savePreferences(store, mergePreferences(preferences, storedPreferences));
    clearLegacyPreferences();
  } catch {
    writeLegacyPreferences(preferences);
  }
}
