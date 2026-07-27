import {
  isSupportedLocale,
  isThemeMode,
  type SupportedLocale,
  type ThemeMode,
  type UserPreferences,
} from "src/types/preferences";

const preferencesStorageKey = "local-console.preferences";
const legacyThemeStorageKey = "vite-ui-theme";

interface StoredPreferencesV1 {
  version: 1;
  locale?: SupportedLocale;
  theme?: ThemeMode;
}

function canUseStorage() {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function readStoredPreferences(): Partial<UserPreferences> {
  if (!canUseStorage()) {
    return {};
  }

  try {
    const rawValue = window.localStorage.getItem(preferencesStorageKey);
    if (rawValue) {
      const parsedValue: unknown = JSON.parse(rawValue);
      if (typeof parsedValue === "object" && parsedValue !== null) {
        const value = parsedValue as StoredPreferencesV1;
        if (value.version === 1) {
          return {
            locale: isSupportedLocale(value.locale) ? value.locale : undefined,
            theme: isThemeMode(value.theme) ? value.theme : undefined,
          };
        }
      }
    }

    const legacyTheme = window.localStorage.getItem(legacyThemeStorageKey);
    return isThemeMode(legacyTheme) ? { theme: legacyTheme } : {};
  } catch {
    return {};
  }
}

function writeStoredPreferences(preferences: Partial<UserPreferences>) {
  if (!canUseStorage()) {
    return;
  }

  try {
    const current = readStoredPreferences();
    const nextValue: StoredPreferencesV1 = {
      version: 1,
      ...current,
      ...preferences,
    };
    window.localStorage.setItem(preferencesStorageKey, JSON.stringify(nextValue));
  } catch {
    // Preferences are optional; a blocked storage write must not interrupt the UI.
  }
}

export async function getPreferences(): Promise<Partial<UserPreferences>> {
  const preferences = readStoredPreferences();
  if (canUseStorage() && !window.localStorage.getItem(preferencesStorageKey) && preferences.theme) {
    writeStoredPreferences(preferences);
  }
  return preferences;
}

export async function setLocale(locale: SupportedLocale): Promise<void> {
  writeStoredPreferences({ locale });
}

export async function setTheme(theme: ThemeMode): Promise<void> {
  writeStoredPreferences({ theme });
}
