export type TrafficLayer = "physical" | "tunnel" | "application";
export type AttributionQuality = "exact" | "interfaceOnly" | "unavailable";
export type SampleState = "sample" | "gap" | "paused";
export type CollectorState =
  | "disabled"
  | "starting"
  | "running"
  | "degraded"
  | "stopped";
export type InterfaceKind =
  | "ethernet"
  | "wifi"
  | "tunnel"
  | "loopback"
  | "virtual"
  | "other";
export type InterfaceState = "up" | "down" | "unknown";
export type RouteMode = "fullTunnel" | "splitTunnel" | "direct" | "unknown";
export type QueryInterval = "auto" | "second" | "minute" | "fiveMinutes";
export type QueryGroupBy = "time" | "interface" | "application" | "proxySession";

export interface NetworkMonitorCommandError {
  code:
    | "invalidRequest"
    | "unsupportedPlatform"
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

export interface CapabilityStatus {
  available: boolean;
  quality: AttributionQuality;
  reason: string | null;
}

export interface NetworkMonitorCapabilities {
  platform: string;
  interfaceTraffic: CapabilityStatus;
  applicationTraffic: CapabilityStatus;
  proxyConfiguration: CapabilityStatus;
  vpnDetection: CapabilityStatus;
  routeModeDetection: CapabilityStatus;
  historyStorage: CapabilityStatus;
  retentionDays: number;
}

export interface NetworkMonitorStatus {
  enabled: boolean;
  collectorState: CollectorState;
  generation: number;
  subscriberCount: number;
  sampleIntervalSeconds: number;
  databaseCreated: boolean;
  lastSampledAt: number | null;
  lastError: NetworkMonitorWarning | null;
}

export interface TrafficValues {
  downloadBytesPerSecond: number | null;
  uploadBytesPerSecond: number | null;
  sessionDownloadBytes: number;
  sessionUploadBytes: number;
}

export interface InterfaceTrafficSnapshot {
  id: string;
  name: string;
  kind: InterfaceKind;
  state: InterfaceState;
  layer: TrafficLayer;
  isVirtual: boolean;
  tunnelType: string | null;
  traffic: TrafficValues;
  quality: AttributionQuality;
}

export interface ApplicationTrafficSnapshot {
  applicationId: string;
  displayName: string;
  traffic: TrafficValues;
  quality: AttributionQuality;
}

export interface ProxyVpnSnapshot {
  proxyConfigured: boolean;
  proxyKinds: string[];
  pacEnabled: boolean;
  vpnConnected: boolean;
  routeMode: RouteMode | null;
  virtualInterfaceIds: string[];
  traffic: TrafficValues;
  quality: AttributionQuality;
}

export interface NetworkRealtimeEvent {
  generation: number;
  sequence: number;
  sampledAt: number;
  elapsedMs: number | null;
  sampleState: SampleState;
  device: TrafficValues;
  interfaces: InterfaceTrafficSnapshot[];
  applications: ApplicationTrafficSnapshot[];
  proxyVpn: ProxyVpnSnapshot;
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
  interval?: QueryInterval;
  groupBy?: QueryGroupBy;
  interfaceIds?: string[];
  applicationIds?: string[];
  proxySessionIds?: string[];
  layers?: TrafficLayer[];
  timeZone: string;
  limit?: number;
  cursor?: string;
}

export interface NetworkUsagePoint {
  from: number;
  to: number;
  groupId: string;
  layer: TrafficLayer;
  downloadBytes: number;
  uploadBytes: number;
  quality: AttributionQuality;
}

export interface NetworkUsageResult {
  generation: number;
  requestedFrom: number;
  requestedTo: number;
  actualFrom: number;
  actualTo: number;
  interval: QueryInterval;
  points: NetworkUsagePoint[];
  totalCount: number;
  nextCursor: string | null;
  partial: boolean;
  warnings: NetworkMonitorWarning[];
}

export interface ClearNetworkUsageRequest {
  scope: "all" | "application" | "timeRange";
  applicationId?: string;
  from?: number;
  to?: number;
}

export interface ClearNetworkUsageResult {
  generation: number;
  deletedBuckets: number;
  clearedAt: number;
  clearedFrom: number | null;
  clearedTo: number | null;
}
