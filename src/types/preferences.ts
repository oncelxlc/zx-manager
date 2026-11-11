export const supportedLocales = ["zh-CN", "en-US"] as const;

export type SupportedLocale = (typeof supportedLocales)[number];

export const themeModes = ["light", "dark", "system"] as const;

export type ThemeMode = (typeof themeModes)[number];

export type ResolvedTheme = Exclude<ThemeMode, "system">;

export const nginxReleaseChannels = ["stable", "mainline"] as const;
export type NginxReleaseChannel = (typeof nginxReleaseChannels)[number];

export interface NginxConfigCustomGroup {
  id: string;
  name: string;
}

export interface NginxPreferences {
  releaseChannel: NginxReleaseChannel;
  updateCheckIntervalHours: number;
  backupRetentionCount: number;
  logFollow: boolean;
  logBufferLines: number;
  configCustomGroups: NginxConfigCustomGroup[];
  configNodeGroups: Record<string, string>;
}

export const defaultNginxPreferences: NginxPreferences = {
  releaseChannel: "stable",
  updateCheckIntervalHours: 24,
  backupRetentionCount: 5,
  logFollow: true,
  logBufferLines: 20_000,
  configCustomGroups: [],
  configNodeGroups: {},
};

export interface UserPreferences {
  locale: SupportedLocale;
  theme: ThemeMode;
  nginx: NginxPreferences;
}

export function isSupportedLocale(value: unknown): value is SupportedLocale {
  return typeof value === "string" && supportedLocales.includes(value as SupportedLocale);
}

export function isThemeMode(value: unknown): value is ThemeMode {
  return typeof value === "string" && themeModes.includes(value as ThemeMode);
}

export function normalizeNginxPreferences(value: unknown): NginxPreferences {
  const candidate = typeof value === "object" && value !== null
    ? value as Partial<NginxPreferences>
    : {};
  const configCustomGroups = Array.isArray(candidate.configCustomGroups)
    ? candidate.configCustomGroups
      .filter((group): group is NginxConfigCustomGroup => (
        typeof group === "object"
        && group !== null
        && typeof group.id === "string"
        && /^[a-zA-Z0-9_-]{1,64}$/.test(group.id)
        && typeof group.name === "string"
        && group.name.trim().length >= 1
        && group.name.trim().length <= 80
      ))
      .slice(0, 100)
      .map((group) => ({ id: group.id, name: group.name.trim() }))
    : [];
  const groupIds = new Set(configCustomGroups.map((group) => group.id));
  const configNodeGroups = typeof candidate.configNodeGroups === "object"
    && candidate.configNodeGroups !== null
    ? Object.fromEntries(
      Object.entries(candidate.configNodeGroups)
        .filter(([nodeId, groupId]) => (
          /^[a-f0-9]{64}$/i.test(nodeId)
          && typeof groupId === "string"
          && groupIds.has(groupId)
        ))
        .slice(0, 10_000),
    )
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
    configCustomGroups,
    configNodeGroups,
  };
}
