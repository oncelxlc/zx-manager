import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { toast } from "@/components/ui/toast";
import {
  aggregateApplications,
  compareApplications,
  getSafePageIndex,
  rangeDurations,
  systemApplicationId,
  toLocalInputValue,
  type ApplicationSortBy,
  type RangePreset,
  unknownApplicationId,
} from "src/components/network-monitor/network-monitor-utils";
import { useNetworkMonitorLifecycle } from "src/components/network-monitor/useNetworkMonitorLifecycle";
import {
  setNetworkMonitorConfigured,
  setNetworkMonitorSampleInterval as persistNetworkMonitorSampleInterval,
} from "src/services/storage/preferences-storage";
import { useNetworkMonitorStore } from "src/stores/network-monitor-store";
import type {
  NetworkPathFilter,
  NetworkUsageSortBy,
  SortDirection,
} from "src/types/network-monitor";
import {
  networkMonitorSampleIntervals,
  type NetworkMonitorSampleInterval,
} from "src/types/preferences";

import type { NetworkTablePageSize } from "src/components/network-monitor/NetworkTablePagination";

const defaultPageSize: NetworkTablePageSize = 10;
export const pathFilterValues: NetworkPathFilter[] = ["all", "proxy", "direct"];

/**
 * Keeps external synchronization (store lifecycle and history requests) out of
 * the route component. Filter and pagination transitions reset page state at
 * their interaction boundary instead of relying on state-reset effects.
 */
export function useNetworkMonitorController() {
  const { i18n, t } = useTranslation(["networkMonitor", "common"]);
  const locale = i18n.resolvedLanguage ?? i18n.language;
  const capabilities = useNetworkMonitorStore((state) => state.capabilities);
  const status = useNetworkMonitorStore((state) => state.status);
  const realtime = useNetworkMonitorStore((state) => state.realtime);
  const history = useNetworkMonitorStore((state) => state.history);
  const loading = useNetworkMonitorStore((state) => state.loading);
  const historyLoading = useNetworkMonitorStore((state) => state.historyLoading);
  const sampleIntervalLoading = useNetworkMonitorStore(
    (state) => state.sampleIntervalLoading,
  );
  const stale = useNetworkMonitorStore((state) => state.stale);
  const error = useNetworkMonitorStore((state) => state.error);
  const initialize = useNetworkMonitorStore((state) => state.initialize);
  const startRealtime = useNetworkMonitorStore((state) => state.startRealtime);
  const stopRealtime = useNetworkMonitorStore((state) => state.stopRealtime);
  const setEnabled = useNetworkMonitorStore((state) => state.setEnabled);
  const setSampleInterval = useNetworkMonitorStore(
    (state) => state.setSampleInterval,
  );
  const refresh = useNetworkMonitorStore((state) => state.refresh);
  const queryHistory = useNetworkMonitorStore((state) => state.queryHistory);
  const clearUsage = useNetworkMonitorStore((state) => state.clearUsage);
  const { configured, setConfigured } = useNetworkMonitorLifecycle({
    initialize,
    startRealtime,
    stopRealtime,
  });
  const platformSupported = capabilities?.platformSupported
    ?? (error?.code === "unsupportedPlatform" ? false : undefined);
  const [applicationPath, setApplicationPath] = useState<NetworkPathFilter>("all");
  const [applicationSortBy, setApplicationSortBy] = useState<ApplicationSortBy>("downloadRate");
  const [applicationSortDirection, setApplicationSortDirection] = useState<SortDirection>("desc");
  const [historyPath, setHistoryPath] = useState<NetworkPathFilter>("all");
  const [historySortBy, setHistorySortBy] = useState<NetworkUsageSortBy>("total");
  const [historySortDirection, setHistorySortDirection] = useState<SortDirection>("desc");
  const [range, setRange] = useState<RangePreset>("1h");
  const [customFrom, setCustomFrom] = useState(() => toLocalInputValue(Date.now() - 60 * 60 * 1_000));
  const [customTo, setCustomTo] = useState(() => toLocalInputValue(Date.now()));
  const [clearOpen, setClearOpen] = useState(false);
  const [applicationPageIndex, setApplicationPageIndex] = useState(0);
  const [applicationPageSize, setApplicationPageSize] = useState<NetworkTablePageSize>(defaultPageSize);
  const [historyPageIndex, setHistoryPageIndex] = useState(0);
  const [historyPageSize, setHistoryPageSize] = useState<NetworkTablePageSize>(defaultPageSize);

  const applicationDisplayName = useCallback((applicationId: string, displayName: string) => {
    if (applicationId === systemApplicationId) return t("applications.system");
    if (applicationId === unknownApplicationId) return t("applications.unknown");
    return displayName;
  }, [t]);

  const runHistoryQuery = useCallback(() => {
    const now = Date.now();
    const from = range === "custom" ? new Date(customFrom).getTime() : now - rangeDurations[range];
    const to = range === "custom" ? new Date(customTo).getTime() : now;
    if (!Number.isFinite(from) || !Number.isFinite(to) || from >= to) {
      toast.add({title: t("history.invalidRange"), type: "error"});
      return;
    }
    void queryHistory({
      from,
      to,
      networkPath: historyPath,
      limit: historyPageSize,
      cursor: String(getSafePageIndex(historyPageIndex, history?.totalCount ?? 0, historyPageSize) * historyPageSize),
      sortBy: historySortBy,
      sortDirection: historySortDirection,
      timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC",
    });
  }, [customFrom, customTo, history?.totalCount, historyPageIndex, historyPageSize, historyPath, historySortBy, historySortDirection, queryHistory, range, t]);

  useEffect(() => {
    if (configured && status && platformSupported) runHistoryQuery();
  }, [configured, platformSupported, runHistoryQuery, status?.generation]);

  // Aggregate before sorting: direct/proxy variants never render as duplicate apps.
  const applications = useMemo(() => [...aggregateApplications(
    realtime[realtime.length - 1]?.applications ?? [], applicationPath,
  )].sort((left, right) => compareApplications(
    left, right, applicationSortBy, applicationSortDirection,
  )), [applicationPath, applicationSortBy, applicationSortDirection, realtime]);
  const safeApplicationPageIndex = getSafePageIndex(applicationPageIndex, applications.length, applicationPageSize);
  const visibleApplications = applications.slice(
    safeApplicationPageIndex * applicationPageSize,
    (safeApplicationPageIndex + 1) * applicationPageSize,
  );
  const safeHistoryPageIndex = getSafePageIndex(historyPageIndex, history?.totalCount ?? 0, historyPageSize);

  const handleCurrentSession = useCallback(async () => {
    const succeeded = await setEnabled(!status?.enabled);
    if (succeeded) toast.add({title: t(status?.enabled ? "toast.stopped" : "toast.starting"), type: "success"});
  }, [setEnabled, status?.enabled, t]);
  const handleSampleIntervalChange = useCallback(async (value: unknown) => {
    const interval = Number(value) as NetworkMonitorSampleInterval;
    if (networkMonitorSampleIntervals.includes(interval) && await setSampleInterval(interval)) {
      await persistNetworkMonitorSampleInterval(interval);
    }
  }, [setSampleInterval]);
  const handleFirstChoice = useCallback(async (start: boolean) => {
    await setNetworkMonitorConfigured(true);
    setConfigured(true);
    if (start) await setEnabled(true);
  }, [setConfigured, setEnabled]);
  const setApplicationSort = useCallback((sortBy: ApplicationSortBy) => {
    setApplicationSortDirection((direction) => applicationSortBy !== sortBy
      ? sortBy === "application" ? "asc" : "desc"
      : direction === "asc" ? "desc" : "asc");
    setApplicationSortBy(sortBy);
    setApplicationPageIndex(0);
  }, [applicationSortBy]);
  const setHistorySort = useCallback((sortBy: NetworkUsageSortBy) => {
    setHistorySortDirection((direction) => historySortBy !== sortBy
      ? sortBy === "application" ? "asc" : "desc"
      : direction === "asc" ? "desc" : "asc");
    setHistorySortBy(sortBy);
    setHistoryPageIndex(0);
  }, [historySortBy]);
  const clearAllUsage = useCallback(() => {
    void clearUsage({scope: "all"}).then((result) => {
      if (result) {
        toast.add({title: t("clear.success"), type: "success"});
        setApplicationPageIndex(0);
        setHistoryPageIndex(0);
      }
      setClearOpen(false);
    });
  }, [clearUsage, t]);

  return {
    applicationDisplayName, applications, applicationPageIndex,
    applicationPageSize, applicationPath, applicationSortBy, applicationSortDirection,
    clearAllUsage, clearOpen, configured, customFrom, customTo, error,
    handleCurrentSession, handleFirstChoice, handleSampleIntervalChange,
    history, historyLoading, historyPageIndex, historyPageSize, historyPath,
    historySortBy, historySortDirection, latest: realtime[realtime.length - 1], loading,
    locale, platformSupported, range, refresh, safeApplicationPageIndex,
    safeHistoryPageIndex, sampleIntervalItems: networkMonitorSampleIntervals.map((interval) => ({
      label: t("sampling.option", {count: interval}), value: String(interval),
    })), sampleIntervalLoading, setApplicationPageIndex, setApplicationPageSize,
    setApplicationPath, setApplicationSort, setClearOpen, setCustomFrom, setCustomTo,
    setHistoryPageIndex, setHistoryPageSize, setHistoryPath, setHistorySort, setRange,
    runHistoryQuery, stale, status, t, visibleApplications,
  };
}
