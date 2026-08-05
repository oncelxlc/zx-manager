import { load, type Store } from "@tauri-apps/plugin-store";

import {
  isSupportedLocale,
  isThemeMode,
  normalizeNginxPreferences,
  type NginxPreferences,
  type SupportedLocale,
  type ThemeMode,
  type UserPreferences,
} from "src/types/preferences";

const preferencesStorageKey = "local-console.preferences";
const legacyThemeStorageKey = "vite-ui-theme";
const storeFileName = "preferences.json";
const storePreferencesKey = "preferences";

interface StoredPreferences {
  version: 1 | 2 | 3 | 4 | 5 | 6 | 7;
  locale?: SupportedLocale;
  theme?: ThemeMode;
  nginx?: Partial<NginxPreferences>;
}

interface StoredPreferencesV7 extends Omit<StoredPreferences, "version"> {
  version: 7;
}

type PreferencesStore = Pick<Store, "get" | "save" | "set">;

let storePromise: Promise<PreferencesStore> | null | undefined;

function canUseStorage() {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function getStoredVersion(value: unknown): StoredPreferences["version"] | undefined {
  if (typeof value !== "object" || value === null || !("version" in value)) {
    return undefined;
  }

  const version = value.version;
  return typeof version === "number" && version >= 1 && version <= 7
    ? version as StoredPreferences["version"]
    : undefined;
}

function normalizePreferences(value: unknown): Partial<UserPreferences> {
  const version = getStoredVersion(value);
  if (!version) {
    return {};
  }

  const preferences = value as StoredPreferences;
  return {
    locale: isSupportedLocale(preferences.locale) ? preferences.locale : undefined,
    theme: isThemeMode(preferences.theme) ? preferences.theme : undefined,
    nginx: normalizeNginxPreferences(
      version >= 6 ? preferences.nginx : undefined,
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
    const nextValue: StoredPreferencesV7 = {
      version: 7,
      ...current,
      ...preferences,
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
    || preferences.nginx !== undefined;
}

function mergePreferences(
  primary: Partial<UserPreferences>,
  fallback: Partial<UserPreferences>,
): Partial<UserPreferences> {
  return {
    locale: primary.locale ?? fallback.locale,
    theme: primary.theme ?? fallback.theme,
    nginx: normalizeNginxPreferences(primary.nginx ?? fallback.nginx),
  };
}

function preferencesMatch(
  left: Partial<UserPreferences>,
  right: Partial<UserPreferences>,
) {
  return left.locale === right.locale
    && left.theme === right.theme
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
  const value: StoredPreferencesV7 = {
    version: 7,
    ...preferences,
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
    const rawPreferences = await store.get<unknown>(storePreferencesKey);
    const storedPreferences = normalizePreferences(rawPreferences);
    const preferences = mergePreferences(storedPreferences, legacyPreferences);
    const storedVersion = getStoredVersion(rawPreferences);
    const needsStoreMigration = storedVersion !== undefined && storedVersion !== 7;

    if (
      needsStoreMigration
      || (hasPreferences(legacyPreferences)
        && !preferencesMatch(preferences, storedPreferences))
    ) {
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
