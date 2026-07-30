export const supportedLocales = ["zh-CN", "en-US"] as const;

export type SupportedLocale = (typeof supportedLocales)[number];

export const themeModes = ["light", "dark", "system"] as const;

export type ThemeMode = (typeof themeModes)[number];

export type ResolvedTheme = Exclude<ThemeMode, "system">;

export const networkMonitorSampleIntervals = [1, 3, 5, 10] as const;

export type NetworkMonitorSampleInterval =
  (typeof networkMonitorSampleIntervals)[number];

export interface UserPreferences {
  locale: SupportedLocale;
  theme: ThemeMode;
  networkMonitorConfigured: boolean;
  networkMonitorSampleIntervalSeconds: NetworkMonitorSampleInterval;
}

export function isSupportedLocale(value: unknown): value is SupportedLocale {
  return typeof value === "string" && supportedLocales.includes(value as SupportedLocale);
}

export function isThemeMode(value: unknown): value is ThemeMode {
  return typeof value === "string" && themeModes.includes(value as ThemeMode);
}

export function isNetworkMonitorSampleInterval(
  value: unknown,
): value is NetworkMonitorSampleInterval {
  return typeof value === "number"
    && networkMonitorSampleIntervals.includes(
      value as NetworkMonitorSampleInterval,
    );
}
