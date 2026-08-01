export type AttributionQuality = "exact" | "partial";
export type NetworkPath = "proxy" | "direct" | "unknown";
export type NetworkPathFilter = "all" | "proxy" | "direct";
export type NetworkUsageSortBy = "application" | "download" | "upload" | "total";
export type SortDirection = "asc" | "desc";
export type SampleState = "sample" | "gap" | "paused";
export type CollectorState =
  | "disabled"
  | "starting"
  | "running"
  | "degraded"
  | "stopped";
export type HelperState = "stopped" | "starting" | "running" | "failed";

export interface NetworkMonitorCommandError {
  code:
    | "invalidRequest"
    | "unsupportedPlatform"
    | "elevationCancelled"
    | "helperDisconnected"
    | "protocolMismatch"
    | "collectorUnavailable"
    | "storageUnavailable"
    | "internal"
    | "unknown";
  message: string;
}

export interface NetworkMonitorWarning {
  code: string;
  message: string | null;
}

export interface NetworkMonitorCapabilities {
  platform: string;
  platformSupported: boolean;
  requiresElevation: boolean;
  applicationTraffic: boolean;
  proxyClassification: boolean;
  historyStorage: boolean;
  retentionDays: number;
}

export interface NetworkMonitorStatus {
  platformSupported: boolean;
  requiresElevation: boolean;
  authorizationReady: boolean;
  enabled: boolean;
  collectorState: CollectorState;
  helperState: HelperState;
  generation: number;
  subscriberCount: number;
  sampleIntervalSeconds: number;
  databaseCreated: boolean;
  lastSampledAt: number | null;
  lostEvents: number;
  unresolvedEvents: number;
  partialData: boolean;
  lastError: NetworkMonitorWarning | null;
}

export interface TrafficValues {
  downloadBytesPerSecond: number | null;
  uploadBytesPerSecond: number | null;
  sessionDownloadBytes: number;
  sessionUploadBytes: number;
}

export interface ApplicationTrafficSnapshot {
  applicationId: string;
  displayName: string;
  networkPath: NetworkPath;
  traffic: TrafficValues;
  quality: AttributionQuality;
}

export interface NetworkRealtimeEvent {
  generation: number;
  sequence: number;
  sampledAt: number;
  elapsedMs: number | null;
  sampleState: SampleState;
  applications: ApplicationTrafficSnapshot[];
  unknownTraffic: TrafficValues;
  lostEvents: number;
  unresolvedEvents: number;
  warnings: NetworkMonitorWarning[];
}

export interface NetworkSubscription {
  subscriptionId: number;
  generation: number;
  initialEvents: NetworkRealtimeEvent[];
}

export interface NetworkUsageQuery {
  from: number;
  to: number;
  networkPath: NetworkPathFilter;
  timeZone: string;
  limit?: number;
  cursor?: string;
  sortBy?: NetworkUsageSortBy;
  sortDirection?: SortDirection;
}

export interface NetworkUsagePoint {
  applicationId: string;
  displayName: string;
  downloadBytes: number;
  uploadBytes: number;
  totalBytes: number;
  includesUnknown: boolean;
  quality: AttributionQuality;
}

export interface NetworkUsageResult {
  generation: number;
  requestedFrom: number;
  requestedTo: number;
  actualFrom: number;
  actualTo: number;
  points: NetworkUsagePoint[];
  totalCount: number;
  nextCursor: string | null;
  partial: boolean;
  warnings: NetworkMonitorWarning[];
}

export interface ClearNetworkUsageRequest {
  scope: "all";
}

export interface ClearNetworkUsageResult {
  generation: number;
  deletedBuckets: number;
  clearedAt: number;
}
