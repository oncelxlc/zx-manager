export const supportedLocales = ["zh-CN", "en-US"] as const;

export type SupportedLocale = (typeof supportedLocales)[number];

export const themeModes = ["light", "dark", "system"] as const;

export type ThemeMode = (typeof themeModes)[number];

export type ResolvedTheme = Exclude<ThemeMode, "system">;

export const networkMonitorSampleIntervals = [1, 3, 5, 10] as const;

export type NetworkMonitorSampleInterval =
  (typeof networkMonitorSampleIntervals)[number];

export const nginxReleaseChannels = ["stable", "mainline"] as const;
export type NginxReleaseChannel = (typeof nginxReleaseChannels)[number];

export interface NginxPreferences {
  releaseChannel: NginxReleaseChannel;
  updateCheckIntervalHours: number;
  backupRetentionCount: number;
  logFollow: boolean;
  logBufferLines: number;
}

export const defaultNginxPreferences: NginxPreferences = {
  releaseChannel: "stable",
  updateCheckIntervalHours: 24,
  backupRetentionCount: 5,
  logFollow: true,
  logBufferLines: 20_000,
};

export interface UserPreferences {
  locale: SupportedLocale;
  theme: ThemeMode;
  networkMonitorConfigured: boolean;
  networkMonitorStartOnLaunch: boolean;
  networkMonitorSampleIntervalSeconds: NetworkMonitorSampleInterval;
  nginx: NginxPreferences;
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

export function normalizeNginxPreferences(value: unknown): NginxPreferences {
  const candidate = typeof value === "object" && value !== null
    ? value as Partial<NginxPreferences>
    : {};
  return {
    releaseChannel:
      typeof candidate.releaseChannel === "string"
      && nginxReleaseChannels.includes(candidate.releaseChannel as NginxReleaseChannel)
        ? candidate.releaseChannel as NginxReleaseChannel
        : defaultNginxPreferences.releaseChannel,
    updateCheckIntervalHours:
      typeof candidate.updateCheckIntervalHours === "number"
      && Number.isInteger(candidate.updateCheckIntervalHours)
      && candidate.updateCheckIntervalHours >= 1
      && candidate.updateCheckIntervalHours <= 168
        ? candidate.updateCheckIntervalHours
        : defaultNginxPreferences.updateCheckIntervalHours,
    backupRetentionCount:
      typeof candidate.backupRetentionCount === "number"
      && Number.isInteger(candidate.backupRetentionCount)
      && candidate.backupRetentionCount >= 1
      && candidate.backupRetentionCount <= 50
        ? candidate.backupRetentionCount
        : defaultNginxPreferences.backupRetentionCount,
    logFollow:
      typeof candidate.logFollow === "boolean"
        ? candidate.logFollow
        : defaultNginxPreferences.logFollow,
    logBufferLines:
      typeof candidate.logBufferLines === "number"
      && Number.isInteger(candidate.logBufferLines)
      && candidate.logBufferLines >= 1_000
      && candidate.logBufferLines <= 100_000
        ? candidate.logBufferLines
        : defaultNginxPreferences.logBufferLines,
  };
}
