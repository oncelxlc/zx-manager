import { load, type Store } from "@tauri-apps/plugin-store";

import {
  isNetworkMonitorSampleInterval,
  isSupportedLocale,
  isThemeMode,
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

interface StoredPreferencesV4 {
  version: 4;
  locale?: SupportedLocale;
  theme?: ThemeMode;
  networkMonitorConfigured?: boolean;
  networkMonitorSampleIntervalSeconds?: NetworkMonitorSampleInterval;
}

interface StoredPreferencesV3 extends Omit<StoredPreferencesV4, "version"> {
  version: 3;
  networkMonitorStartOnLaunch?: boolean;
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
    | StoredPreferencesV4;
  if (
    preferences.version !== 1
    && preferences.version !== 2
    && preferences.version !== 3
    && preferences.version !== 4
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
    networkMonitorSampleIntervalSeconds:
      (preferences.version === 3 || preferences.version === 4)
      && isNetworkMonitorSampleInterval(
        preferences.networkMonitorSampleIntervalSeconds,
      )
        ? preferences.networkMonitorSampleIntervalSeconds
        : 5,
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
    const nextValue: StoredPreferencesV4 = {
      version: 4,
      ...current,
      ...preferences,
      networkMonitorConfigured:
        preferences.networkMonitorConfigured
        ?? current.networkMonitorConfigured
        ?? false,
      networkMonitorSampleIntervalSeconds:
        preferences.networkMonitorSampleIntervalSeconds
        ?? current.networkMonitorSampleIntervalSeconds
        ?? 5,
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
    || preferences.networkMonitorSampleIntervalSeconds !== undefined;
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
    networkMonitorSampleIntervalSeconds:
      primary.networkMonitorSampleIntervalSeconds
      ?? fallback.networkMonitorSampleIntervalSeconds
      ?? 5,
  };
}

function preferencesMatch(
  left: Partial<UserPreferences>,
  right: Partial<UserPreferences>,
) {
  return left.locale === right.locale
    && left.theme === right.theme
    && left.networkMonitorConfigured === right.networkMonitorConfigured
    && left.networkMonitorSampleIntervalSeconds
      === right.networkMonitorSampleIntervalSeconds;
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
  const value: StoredPreferencesV4 = {
    version: 4,
    ...preferences,
    networkMonitorConfigured: preferences.networkMonitorConfigured ?? false,
    networkMonitorSampleIntervalSeconds:
      preferences.networkMonitorSampleIntervalSeconds ?? 5,
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

export async function setNetworkMonitorSampleInterval(
  interval: NetworkMonitorSampleInterval,
): Promise<void> {
  await updatePreferences({ networkMonitorSampleIntervalSeconds: interval });
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
