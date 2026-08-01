import type {
  ApplicationTrafficSnapshot,
  AttributionQuality,
  NetworkPathFilter,
  SortDirection,
  TrafficValues,
} from "src/types/network-monitor";

export type RangePreset = "10m" | "1h" | "24h" | "7d" | "custom";

export type ApplicationSortBy =
  | "application"
  | "downloadRate"
  | "uploadRate"
  | "download"
  | "upload";

export interface AggregatedApplication {
  applicationId: string;
  displayName: string;
  traffic: TrafficValues;
  quality: AttributionQuality;
}

export const rangeDurations: Record<Exclude<RangePreset, "custom">, number> = {
  "10m": 10 * 60 * 1_000,
  "1h": 60 * 60 * 1_000,
  "24h": 24 * 60 * 60 * 1_000,
  "7d": 7 * 24 * 60 * 60 * 1_000,
};

export const unknownApplicationId = "0".repeat(64);
export const systemApplicationId = "f".repeat(64);

export function toLocalInputValue(timestamp: number) {
  const date = new Date(
    timestamp - new Date(timestamp).getTimezoneOffset() * 60_000,
  );
  return date.toISOString().slice(0, 16);
}

export function aggregateApplications(
  applications: ApplicationTrafficSnapshot[],
  filter: NetworkPathFilter,
): AggregatedApplication[] {
  const grouped = new Map<string, AggregatedApplication>();
  for (const application of applications) {
    if (filter !== "all" && application.networkPath !== filter) {
      continue;
    }
    const existing = grouped.get(application.applicationId);
    if (existing) {
      existing.traffic.downloadBytesPerSecond =
        (existing.traffic.downloadBytesPerSecond ?? 0)
        + (application.traffic.downloadBytesPerSecond ?? 0);
      existing.traffic.uploadBytesPerSecond =
        (existing.traffic.uploadBytesPerSecond ?? 0)
        + (application.traffic.uploadBytesPerSecond ?? 0);
      existing.traffic.sessionDownloadBytes += application.traffic.sessionDownloadBytes;
      existing.traffic.sessionUploadBytes += application.traffic.sessionUploadBytes;
      if (application.quality === "partial") {
        existing.quality = "partial";
      }
      continue;
    }
    grouped.set(application.applicationId, {
      applicationId: application.applicationId,
      displayName: application.displayName,
      traffic: { ...application.traffic },
      quality: application.quality,
    });
  }
  return [...grouped.values()];
}

export function compareApplications(
  left: AggregatedApplication,
  right: AggregatedApplication,
  sortBy: ApplicationSortBy,
  direction: SortDirection,
) {
  const comparison = (() => {
    switch (sortBy) {
      case "application":
        return left.displayName.localeCompare(right.displayName);
      case "downloadRate":
        return (left.traffic.downloadBytesPerSecond ?? 0)
          - (right.traffic.downloadBytesPerSecond ?? 0);
      case "uploadRate":
        return (left.traffic.uploadBytesPerSecond ?? 0)
          - (right.traffic.uploadBytesPerSecond ?? 0);
      case "download":
        return left.traffic.sessionDownloadBytes - right.traffic.sessionDownloadBytes;
      case "upload":
        return left.traffic.sessionUploadBytes - right.traffic.sessionUploadBytes;
    }
  })();
  const ordered = direction === "asc" ? comparison : -comparison;
  return ordered || left.applicationId.localeCompare(right.applicationId);
}

export function getSafePageIndex(pageIndex: number, totalCount: number, pageSize: number) {
  return Math.min(pageIndex, Math.max(0, Math.ceil(totalCount / pageSize) - 1));
}
