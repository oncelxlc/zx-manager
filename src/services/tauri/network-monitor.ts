import { Channel, invoke, isTauri } from "@tauri-apps/api/core";

import type {
  ClearNetworkUsageRequest,
  ClearNetworkUsageResult,
  NetworkMonitorCapabilities,
  NetworkMonitorCommandError,
  NetworkMonitorStatus,
  NetworkRealtimeEvent,
  NetworkSubscription,
  NetworkUsageQuery,
  NetworkUsageResult,
} from "src/types/network-monitor";
import type { NetworkMonitorSampleInterval } from "src/types/preferences";

export interface NetworkRealtimeSubscription {
  subscription: NetworkSubscription;
  cleanup: () => Promise<void>;
}

function assertTauri() {
  if (!isTauri()) {
    throw {
      code: "unsupportedPlatform",
      message: "Network monitoring requires the Tauri desktop runtime.",
    } satisfies NetworkMonitorCommandError;
  }
}

export async function getNetworkMonitorCapabilities(): Promise<NetworkMonitorCapabilities> {
  assertTauri();
  return invoke("get_network_monitor_capabilities");
}

export async function getNetworkMonitorStatus(): Promise<NetworkMonitorStatus> {
  assertTauri();
  return invoke("get_network_monitor_status");
}

export async function prepareNetworkMonitor(): Promise<NetworkMonitorStatus> {
  assertTauri();
  return invoke("prepare_network_monitor");
}

export async function subscribeNetworkRealtime(
  onMessage: (event: NetworkRealtimeEvent) => void,
): Promise<NetworkRealtimeSubscription> {
  assertTauri();
  const channel = new Channel<NetworkRealtimeEvent>();
  channel.onmessage = onMessage;
  const subscription = await invoke<NetworkSubscription>(
    "subscribe_network_realtime",
    { channel },
  );
  let cleanedUp = false;

  return {
    subscription,
    cleanup: async () => {
      if (cleanedUp) {
        return;
      }
      cleanedUp = true;
      channel.onmessage = () => undefined;
      await invoke("unsubscribe_network_realtime", {
        subscriptionId: subscription.subscriptionId,
      });
    },
  };
}

export async function queryNetworkUsage(
  request: NetworkUsageQuery,
): Promise<NetworkUsageResult> {
  assertTauri();
  return invoke("query_network_usage", { request });
}

export async function setNetworkMonitorEnabled(
  enabled: boolean,
): Promise<NetworkMonitorStatus> {
  assertTauri();
  return invoke("set_network_monitor_enabled", { enabled });
}

export async function setNetworkMonitorSampleInterval(
  sampleIntervalSeconds: NetworkMonitorSampleInterval,
): Promise<NetworkMonitorStatus> {
  assertTauri();
  return invoke("set_network_monitor_sample_interval", {
    sampleIntervalSeconds,
  });
}

export async function clearNetworkUsage(
  request: ClearNetworkUsageRequest,
): Promise<ClearNetworkUsageResult> {
  assertTauri();
  return invoke("clear_network_usage", { request });
}

export function toNetworkMonitorError(error: unknown): NetworkMonitorCommandError {
  if (
    typeof error === "object"
    && error !== null
    && "code" in error
    && typeof error.code === "string"
  ) {
    return {
      code: error.code as NetworkMonitorCommandError["code"],
      message:
        "message" in error && typeof error.message === "string"
          ? error.message
          : "",
    };
  }
  return {
    code: "unknown",
    message: error instanceof Error ? error.message : String(error),
  };
}
