import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AppWindowIcon,
  ArrowDownIcon,
  ArrowUpIcon,
  DatabaseIcon,
  NetworkIcon,
  RefreshCwIcon,
  ShieldAlertIcon,
  Trash2Icon,
  TriangleAlertIcon,
} from "lucide-react";
import {
  CartesianGrid,
  Line,
  LineChart,
  XAxis,
  YAxis,
} from "recharts";

import {
  Alert,
  AlertDescription,
  AlertTitle,
} from "@/components/ui/alert";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogMedia,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
import {
  Empty,
  EmptyHeader,
  EmptyTitle,
} from "@/components/ui/empty";
import {
  Field,
  FieldGroup,
  FieldLabel,
  FieldTitle,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { toast } from "@/components/ui/toast";
import {
  NetworkTablePagination,
  type NetworkTablePageSize,
} from "src/components/network-monitor/NetworkTablePagination";
import { useMainLayoutHeader } from "src/layouts/MainLayout";
import {
  getPreferences,
  setNetworkMonitorConfigured,
  setNetworkMonitorSampleInterval as persistNetworkMonitorSampleInterval,
} from "src/services/storage/preferences-storage";
import { useNetworkMonitorStore } from "src/stores/network-monitor-store";
import type {
  ApplicationTrafficSnapshot,
  AttributionQuality,
  NetworkPathFilter,
  NetworkRealtimeEvent,
  TrafficValues,
} from "src/types/network-monitor";
import {
  networkMonitorSampleIntervals,
  type NetworkMonitorSampleInterval,
} from "src/types/preferences";
import { formatDataRate, formatDataSize } from "src/utils/format-data-size";

type RangePreset = "10m" | "1h" | "24h" | "7d" | "custom";

interface AggregatedApplication {
  applicationId: string;
  displayName: string;
  traffic: TrafficValues;
  quality: AttributionQuality;
}

const rangeDurations: Record<Exclude<RangePreset, "custom">, number> = {
  "10m": 10 * 60 * 1_000,
  "1h": 60 * 60 * 1_000,
  "24h": 24 * 60 * 60 * 1_000,
  "7d": 7 * 24 * 60 * 60 * 1_000,
};

const defaultPageSize: NetworkTablePageSize = 10;
const pathFilterValues: NetworkPathFilter[] = ["all", "proxy", "direct"];
const unknownApplicationId = "0".repeat(64);
const systemApplicationId = "f".repeat(64);

function toLocalInputValue(timestamp: number) {
  const date = new Date(
    timestamp - new Date(timestamp).getTimezoneOffset() * 60_000,
  );
  return date.toISOString().slice(0, 16);
}

function pathMatches(
  application: ApplicationTrafficSnapshot,
  filter: NetworkPathFilter,
) {
  return filter === "all" || application.networkPath === filter;
}

function aggregateApplications(
  applications: ApplicationTrafficSnapshot[],
  filter: NetworkPathFilter,
): AggregatedApplication[] {
  const grouped = new Map<string, AggregatedApplication>();
  for (const application of applications) {
    if (!pathMatches(application, filter)) {
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
      existing.traffic.sessionDownloadBytes +=
        application.traffic.sessionDownloadBytes;
      existing.traffic.sessionUploadBytes +=
        application.traffic.sessionUploadBytes;
      if (application.quality === "partial") {
        existing.quality = "partial";
      }
      continue;
    }
    grouped.set(application.applicationId, {
      applicationId: application.applicationId,
      displayName: application.displayName,
      traffic: {...application.traffic},
      quality: application.quality,
    });
  }
  return [...grouped.values()].sort((left, right) => {
    const leftRate =
      (left.traffic.downloadBytesPerSecond ?? 0)
      + (left.traffic.uploadBytesPerSecond ?? 0);
    const rightRate =
      (right.traffic.downloadBytesPerSecond ?? 0)
      + (right.traffic.uploadBytesPerSecond ?? 0);
    return rightRate - leftRate || left.applicationId.localeCompare(right.applicationId);
  });
}

function sumApplicationTraffic(
  applications: AggregatedApplication[],
): TrafficValues {
  return applications.reduce<TrafficValues>(
    (total, application) => ({
      downloadBytesPerSecond:
        (total.downloadBytesPerSecond ?? 0)
        + (application.traffic.downloadBytesPerSecond ?? 0),
      uploadBytesPerSecond:
        (total.uploadBytesPerSecond ?? 0)
        + (application.traffic.uploadBytesPerSecond ?? 0),
      sessionDownloadBytes:
        total.sessionDownloadBytes
        + application.traffic.sessionDownloadBytes,
      sessionUploadBytes:
        total.sessionUploadBytes
        + application.traffic.sessionUploadBytes,
    }),
    {
      downloadBytesPerSecond: 0,
      uploadBytesPerSecond: 0,
      sessionDownloadBytes: 0,
      sessionUploadBytes: 0,
    },
  );
}

function eventTraffic(
  event: NetworkRealtimeEvent,
  filter: NetworkPathFilter,
) {
  if (event.sampleState !== "sample") {
    return {download: null, upload: null};
  }
  const traffic = sumApplicationTraffic(
    aggregateApplications(event.applications, filter),
  );
  return {
    download: traffic.downloadBytesPerSecond,
    upload: traffic.uploadBytesPerSecond,
  };
}

export function NetworkMonitorPage() {
  const {i18n, t} = useTranslation(["networkMonitor", "common"]);
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
  const applicationDisplayName = useCallback(
    (applicationId: string, displayName: string) => {
      if (applicationId === systemApplicationId) {
        return t("applications.system");
      }
      if (applicationId === unknownApplicationId) {
        return t("applications.unknown");
      }
      return displayName;
    },
    [t],
  );

  const [configured, setConfigured] = useState<boolean | null>(null);
  const platformSupported =
    capabilities?.platformSupported
    ?? (error?.code === "unsupportedPlatform" ? false : undefined);
  const [realtimePath, setRealtimePath] =
    useState<NetworkPathFilter>("all");
  const [historyPath, setHistoryPath] = useState<NetworkPathFilter>("all");
  const [range, setRange] = useState<RangePreset>("1h");
  const [customFrom, setCustomFrom] = useState(() =>
    toLocalInputValue(Date.now() - 60 * 60 * 1_000),
  );
  const [customTo, setCustomTo] = useState(() =>
    toLocalInputValue(Date.now()),
  );
  const [clearOpen, setClearOpen] = useState(false);
  const [applicationPageIndex, setApplicationPageIndex] = useState(0);
  const [applicationPageSize, setApplicationPageSize] =
    useState<NetworkTablePageSize>(defaultPageSize);
  const [historyPageIndex, setHistoryPageIndex] = useState(0);
  const [historyPageSize, setHistoryPageSize] =
    useState<NetworkTablePageSize>(defaultPageSize);

  useEffect(() => {
    let active = true;
    void getPreferences().then((preferences) => {
      if (active) {
        setConfigured(preferences.networkMonitorConfigured ?? false);
      }
    });
    void initialize();
    void startRealtime();
    return () => {
      active = false;
      void stopRealtime();
    };
  }, [initialize, startRealtime, stopRealtime]);

  const runHistoryQuery = useCallback(() => {
    const now = Date.now();
    const from =
      range === "custom"
        ? new Date(customFrom).getTime()
        : now - rangeDurations[range];
    const to = range === "custom" ? new Date(customTo).getTime() : now;
    if (!Number.isFinite(from) || !Number.isFinite(to) || from >= to) {
      toast.add({
        title: t("history.invalidRange"),
        type: "error",
      });
      return;
    }
    void queryHistory({
      from,
      to,
      networkPath: historyPath,
      limit: historyPageSize,
      cursor: String(historyPageIndex * historyPageSize),
      timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC",
    });
  }, [
    customFrom,
    customTo,
    historyPageIndex,
    historyPageSize,
    historyPath,
    queryHistory,
    range,
    t,
  ]);

  useEffect(() => {
    if (configured && status && platformSupported) {
      runHistoryQuery();
    }
  }, [
    configured,
    platformSupported,
    runHistoryQuery,
    status?.generation,
  ]);

  const latest = realtime[realtime.length - 1];
  const applications = useMemo(
    () => aggregateApplications(latest?.applications ?? [], realtimePath),
    [latest?.applications, realtimePath],
  );
  const selectedTraffic = useMemo(
    () => sumApplicationTraffic(applications),
    [applications],
  );
  const applicationPageCount = Math.max(
    1,
    Math.ceil(applications.length / applicationPageSize),
  );
  const visibleApplications = applications.slice(
    applicationPageIndex * applicationPageSize,
    (applicationPageIndex + 1) * applicationPageSize,
  );

  useEffect(() => {
    if (applicationPageIndex >= applicationPageCount) {
      setApplicationPageIndex(applicationPageCount - 1);
    }
  }, [applicationPageCount, applicationPageIndex]);

  useEffect(() => {
    const pageCount = Math.max(
      1,
      Math.ceil((history?.totalCount ?? 0) / historyPageSize),
    );
    if (historyPageIndex >= pageCount) {
      setHistoryPageIndex(pageCount - 1);
    }
  }, [history?.totalCount, historyPageIndex, historyPageSize]);

  useEffect(() => {
    setApplicationPageIndex(0);
  }, [realtimePath, status?.generation]);

  useEffect(() => {
    setHistoryPageIndex(0);
  }, [historyPath, range, status?.generation]);

  const chartData = useMemo(() => {
    const latestSampledAt =
      realtime[realtime.length - 1]?.sampledAt ?? Date.now();
    const chartFrom = latestSampledAt - rangeDurations["10m"];
    return realtime
      .filter((event) => event.sampledAt >= chartFrom)
      .map((event) => ({
        sampledAt: event.sampledAt,
        ...eventTraffic(event, realtimePath),
      }));
  }, [realtime, realtimePath]);
  const chartConfig = {
    download: {
      color: "var(--chart-1)",
      label: t("metrics.downloadRate"),
    },
    upload: {
      color: "var(--chart-2)",
      label: t("metrics.uploadRate"),
    },
  } satisfies ChartConfig;

  const handleCurrentSession = useCallback(async () => {
    const succeeded = await setEnabled(!status?.enabled);
    if (succeeded) {
      toast.add({
        title: t(status?.enabled ? "toast.stopped" : "toast.starting"),
        type: "success",
      });
    }
  }, [setEnabled, status?.enabled, t]);

  const handleSampleIntervalChange = useCallback(
    async (value: unknown) => {
      const interval = Number(value) as NetworkMonitorSampleInterval;
      if (!networkMonitorSampleIntervals.includes(interval)) {
        return;
      }
      const succeeded = await setSampleInterval(interval);
      if (succeeded) {
        await persistNetworkMonitorSampleInterval(interval);
      }
    },
    [setSampleInterval],
  );

  const handleFirstChoice = useCallback(
    async (start: boolean) => {
      await setNetworkMonitorConfigured(true);
      setConfigured(true);
      if (start) {
        await setEnabled(true);
      }
    },
    [setEnabled],
  );

  const headerActions = useMemo(
    () =>
      platformSupported === false ? null : (
        <>
          <Badge variant={status?.enabled ? "default" : "secondary"}>
            {t(status?.enabled ? "status.running" : "status.disabled")}
          </Badge>
          <Button
            disabled={loading}
            onClick={() => void handleCurrentSession()}
            variant="outline"
          >
            {loading ? (
              <Spinner data-icon="inline-start"/>
            ) : (
              <NetworkIcon data-icon="inline-start"/>
            )}
            {t(status?.enabled ? "actions.stopSession" : "actions.startSession")}
          </Button>
          <Button disabled={loading} onClick={() => void refresh()} variant="outline">
            <RefreshCwIcon data-icon="inline-start"/>
            {t("common:actions.refresh")}
          </Button>
          <Button
            onClick={() => setClearOpen(true)}
            variant="destructive"
          >
            <Trash2Icon data-icon="inline-start"/>
            {t("actions.clear")}
          </Button>
        </>
      ),
    [
      handleCurrentSession,
      loading,
      platformSupported,
      refresh,
      status?.enabled,
      t,
    ],
  );

  useMainLayoutHeader({
    actions: headerActions,
    title: t("title"),
  });

  const sampleIntervalItems = networkMonitorSampleIntervals.map((interval) => ({
    label: t("sampling.option", {count: interval}),
    value: String(interval),
  }));
  const unknownBytes =
    (latest?.unknownTraffic.sessionDownloadBytes ?? 0)
    + (latest?.unknownTraffic.sessionUploadBytes ?? 0);
  const summaryCards = [
    {
      icon: ArrowDownIcon,
      label: t("metrics.downloadRate"),
      value: formatDataRate(
        selectedTraffic.downloadBytesPerSecond,
        locale,
      ),
    },
    {
      icon: ArrowUpIcon,
      label: t("metrics.uploadRate"),
      value: formatDataRate(selectedTraffic.uploadBytesPerSecond, locale),
    },
    {
      icon: null,
      label: t("metrics.sessionDownload"),
      value: formatDataSize(selectedTraffic.sessionDownloadBytes, locale),
    },
    {
      icon: null,
      label: t("metrics.sessionUpload"),
      value: formatDataSize(selectedTraffic.sessionUploadBytes, locale),
    },
    {
      icon: AppWindowIcon,
      label: t("metrics.activeApplications"),
      value: String(
        applications.filter(
          (application) =>
            (application.traffic.downloadBytesPerSecond ?? 0)
            + (application.traffic.uploadBytesPerSecond ?? 0)
            > 0,
        ).length,
      ),
    },
    {
      icon: TriangleAlertIcon,
      label: t("metrics.unclassifiedTraffic"),
      value: formatDataSize(unknownBytes, locale),
    },
  ];

  if (platformSupported === false) {
    return (
      <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4 lg:p-6">
        <div className="flex flex-col gap-1">
          <h1 className="text-2xl font-semibold tracking-tight">{t("title")}</h1>
          <p className="text-sm text-muted-foreground">{t("description")}</p>
        </div>
        <Alert>
          <ShieldAlertIcon/>
          <AlertTitle>{t("unsupported.title")}</AlertTitle>
          <AlertDescription>{t("unsupported.description")}</AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4 lg:p-6">
      <div className="flex flex-col gap-1">
        <h1 className="text-2xl font-semibold tracking-tight">{t("title")}</h1>
        <p className="text-sm text-muted-foreground">
          {status?.lastSampledAt
            ? t("lastUpdated", {
              value: new Intl.DateTimeFormat(locale, {
                dateStyle: "short",
                timeStyle: "medium",
              }).format(status.lastSampledAt),
            })
            : t("description")}
          {stale ? ` · ${t("status.stale")}` : ""}
        </p>
      </div>

      {configured === false ? (
        <Card>
          <CardHeader>
            <CardTitle>{t("onboarding.title")}</CardTitle>
            <CardDescription>{t("onboarding.description")}</CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-4">
            <Alert>
              <DatabaseIcon/>
              <AlertTitle>{t("onboarding.storageTitle")}</AlertTitle>
              <AlertDescription>
                {t("onboarding.storageDescription")}
              </AlertDescription>
            </Alert>
            <div className="flex flex-wrap gap-2">
              <Button onClick={() => void handleFirstChoice(true)}>
                {t("onboarding.start")}
              </Button>
              <Button
                onClick={() => void handleFirstChoice(false)}
                variant="outline"
              >
                {t("onboarding.notNow")}
              </Button>
            </div>
          </CardContent>
        </Card>
      ) : null}

      {error || status?.lastError ? (
        <Alert variant="destructive">
          <ShieldAlertIcon/>
          <AlertTitle>{t("errors.title")}</AlertTitle>
          <AlertDescription>
            {t(
              `errors.codes.${error?.code ?? status?.lastError?.code ?? "unknown"}`,
              {
                defaultValue:
                  error?.message ?? status?.lastError?.message ?? "",
              },
            )}
          </AlertDescription>
        </Alert>
      ) : null}

      {status?.partialData || latest?.sampleState === "gap" ? (
        <Alert>
          <TriangleAlertIcon/>
          <AlertTitle>{t("quality.partialTitle")}</AlertTitle>
          <AlertDescription>
            {t("quality.partialDescription", {
              lost: status?.lostEvents ?? latest?.lostEvents ?? 0,
              unresolved:
                status?.unresolvedEvents ?? latest?.unresolvedEvents ?? 0,
            })}
          </AlertDescription>
        </Alert>
      ) : null}

      {unknownBytes > 0 ? (
        <Alert>
          <TriangleAlertIcon/>
          <AlertTitle>{t("quality.unknownTitle")}</AlertTitle>
          <AlertDescription>{t("quality.unknownDescription")}</AlertDescription>
        </Alert>
      ) : null}

      <Alert>
        <NetworkIcon/>
        <AlertTitle>{t("quality.aggregateTitle")}</AlertTitle>
        <AlertDescription>{t("quality.aggregateDescription")}</AlertDescription>
      </Alert>

      <Field orientation="horizontal">
        <FieldTitle id="network-realtime-path-label">
          {t("pathFilter.realtime")}
        </FieldTitle>
        <ToggleGroup
          aria-labelledby="network-realtime-path-label"
          onValueChange={(values) => {
            const value = values[0] as NetworkPathFilter | undefined;
            if (value) {
              setRealtimePath(value);
            }
          }}
          value={[realtimePath]}
          variant="outline"
        >
          {pathFilterValues.map((value) => (
            <ToggleGroupItem key={value} value={value}>
              {t(`pathFilter.options.${value}`)}
            </ToggleGroupItem>
          ))}
        </ToggleGroup>
      </Field>

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-6">
        {summaryCards.map((card) => {
          const Icon = card.icon;
          return (
            <Card key={card.label}>
              <CardHeader>
                <CardDescription>{card.label}</CardDescription>
                <CardTitle className="flex items-center gap-2 text-xl">
                  {Icon ? <Icon aria-hidden="true"/> : null}
                  {card.value}
                </CardTitle>
              </CardHeader>
            </Card>
          );
        })}
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t("realtime.title")}</CardTitle>
          <CardDescription>{t("realtime.description")}</CardDescription>
          <CardAction>
            <Field data-disabled={sampleIntervalLoading ? "" : undefined}>
              <FieldLabel htmlFor="network-sample-interval">
                {t("sampling.label")}
              </FieldLabel>
              <Select
                disabled={sampleIntervalLoading}
                items={sampleIntervalItems}
                onValueChange={(value) =>
                  void handleSampleIntervalChange(value)
                }
                value={String(status?.sampleIntervalSeconds ?? 3)}
              >
                <SelectTrigger
                  className="w-32"
                  id="network-sample-interval"
                >
                  <SelectValue/>
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    {sampleIntervalItems.map((item) => (
                      <SelectItem key={item.value} value={item.value}>
                        {item.label}
                      </SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
            </Field>
          </CardAction>
        </CardHeader>
        <CardContent>
          <ChartContainer
            className="h-70 w-full aspect-auto"
            config={chartConfig}
            initialDimension={{width: 900, height: 280}}
          >
            <LineChart
              accessibilityLayer
              data={chartData}
              margin={{bottom: 0, left: 0, right: 16, top: 8}}
            >
              <CartesianGrid strokeDasharray="3 3" vertical={false}/>
              <XAxis
                dataKey="sampledAt"
                tickFormatter={(value: number) =>
                  new Intl.DateTimeFormat(locale, {
                    hour: "2-digit",
                    minute: "2-digit",
                  }).format(value)
                }
                tickLine={false}
              />
              <YAxis
                tickFormatter={(value: number) =>
                  formatDataSize(value, locale)
                }
                tickLine={false}
                width={72}
              />
              <ChartTooltip
                content={<ChartTooltipContent/>}
                cursor={false}
              />
              <Line
                connectNulls={false}
                dataKey="download"
                dot={false}
                stroke="var(--color-download)"
                strokeWidth={2}
                type="monotone"
              />
              <Line
                connectNulls={false}
                dataKey="upload"
                dot={false}
                stroke="var(--color-upload)"
                strokeWidth={2}
                type="monotone"
              />
            </LineChart>
          </ChartContainer>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("applications.title")}</CardTitle>
          <CardDescription>{t("applications.description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <Table
            className="min-w-224 table-fixed"
            containerClassName="max-h-[52.5rem] overflow-auto rounded-md border"
          >
            <TableHeader className="[&_th]:sticky [&_th]:top-0 [&_th]:bg-card">
              <TableRow>
                <TableHead>{t("applications.columns.name")}</TableHead>
                <TableHead className="w-40 text-right">
                  {t("applications.columns.downloadRate")}
                </TableHead>
                <TableHead className="w-40 text-right">
                  {t("applications.columns.uploadRate")}
                </TableHead>
                <TableHead className="w-40 text-right">
                  {t("applications.columns.sessionDownload")}
                </TableHead>
                <TableHead className="w-40 text-right">
                  {t("applications.columns.sessionUpload")}
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {visibleApplications.map((application) => (
                <TableRow className="h-10" key={application.applicationId}>
                  <TableCell className="min-w-0 font-medium">
                    <div className="flex items-center gap-2">
                      <span
                        className="block truncate"
                        title={applicationDisplayName(
                          application.applicationId,
                          application.displayName,
                        )}
                      >
                        {applicationDisplayName(
                          application.applicationId,
                          application.displayName,
                        )}
                      </span>
                      {application.quality === "partial" ? (
                        <Badge variant="secondary">{t("quality.partial")}</Badge>
                      ) : null}
                    </div>
                  </TableCell>
                  <TableCell className="text-right">
                    {formatDataRate(
                      application.traffic.downloadBytesPerSecond,
                      locale,
                    )}
                  </TableCell>
                  <TableCell className="text-right">
                    {formatDataRate(
                      application.traffic.uploadBytesPerSecond,
                      locale,
                    )}
                  </TableCell>
                  <TableCell className="text-right">
                    {formatDataSize(
                      application.traffic.sessionDownloadBytes,
                      locale,
                    )}
                  </TableCell>
                  <TableCell className="text-right">
                    {formatDataSize(
                      application.traffic.sessionUploadBytes,
                      locale,
                    )}
                  </TableCell>
                </TableRow>
              ))}
              {applications.length === 0 ? (
                <TableRow className="h-100">
                  <TableCell colSpan={5}>
                    <Empty className="h-full border-0">
                      <EmptyHeader>
                        <EmptyTitle>
                          {t(
                            status?.enabled
                              ? "applications.waiting"
                              : "applications.disabled",
                          )}
                        </EmptyTitle>
                      </EmptyHeader>
                    </Empty>
                  </TableCell>
                </TableRow>
              ) : null}
            </TableBody>
          </Table>
          <NetworkTablePagination
            onPageChange={setApplicationPageIndex}
            onPageSizeChange={(pageSize) => {
              setApplicationPageSize(pageSize);
              setApplicationPageIndex(0);
            }}
            pageIndex={applicationPageIndex}
            pageSize={applicationPageSize}
            totalCount={applications.length}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("history.title")}</CardTitle>
          <CardDescription>{t("history.description")}</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-5">
          <FieldGroup>
            <Field>
              <FieldLabel>{t("history.range")}</FieldLabel>
              <ToggleGroup
                onValueChange={(values) => {
                  const value = values[0] as RangePreset | undefined;
                  if (value) {
                    setRange(value);
                  }
                }}
                value={[range]}
                variant="outline"
              >
                {(["10m", "1h", "24h", "7d", "custom"] as const).map(
                  (value) => (
                    <ToggleGroupItem key={value} value={value}>
                      {t(`history.ranges.${value}`)}
                    </ToggleGroupItem>
                  ),
                )}
              </ToggleGroup>
            </Field>
            {range === "custom" ? (
              <FieldGroup className="grid md:grid-cols-2">
                <Field>
                  <FieldLabel htmlFor="network-history-from">
                    {t("history.from")}
                  </FieldLabel>
                  <Input
                    id="network-history-from"
                    onChange={(event) => setCustomFrom(event.target.value)}
                    type="datetime-local"
                    value={customFrom}
                  />
                </Field>
                <Field>
                  <FieldLabel htmlFor="network-history-to">
                    {t("history.to")}
                  </FieldLabel>
                  <Input
                    id="network-history-to"
                    onChange={(event) => setCustomTo(event.target.value)}
                    type="datetime-local"
                    value={customTo}
                  />
                </Field>
              </FieldGroup>
            ) : null}
            <Field>
              <FieldLabel>{t("pathFilter.history")}</FieldLabel>
              <ToggleGroup
                onValueChange={(values) => {
                  const value = values[0] as NetworkPathFilter | undefined;
                  if (value) {
                    setHistoryPath(value);
                  }
                }}
                value={[historyPath]}
                variant="outline"
              >
                {pathFilterValues.map((value) => (
                  <ToggleGroupItem key={value} value={value}>
                    {t(`pathFilter.options.${value}`)}
                  </ToggleGroupItem>
                ))}
              </ToggleGroup>
            </Field>
          </FieldGroup>
          <div>
            <Button
              disabled={historyLoading}
              onClick={() => {
                if (historyPageIndex === 0) {
                  runHistoryQuery();
                } else {
                  setHistoryPageIndex(0);
                }
              }}
            >
              {historyLoading ? (
                <Spinner data-icon="inline-start"/>
              ) : (
                <RefreshCwIcon data-icon="inline-start"/>
              )}
              {t("history.query")}
            </Button>
          </div>
          <Table
            className="min-w-176 table-fixed"
            containerClassName="max-h-[52.5rem] overflow-auto rounded-md border"
          >
            <TableHeader className="[&_th]:sticky [&_th]:top-0 [&_th]:bg-card">
              <TableRow>
                <TableHead>{t("history.columns.application")}</TableHead>
                <TableHead className="w-40 text-right">
                  {t("history.columns.download")}
                </TableHead>
                <TableHead className="w-40 text-right">
                  {t("history.columns.upload")}
                </TableHead>
                <TableHead className="w-40 text-right">
                  {t("history.columns.total")}
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(history?.points ?? []).map((point) => (
                <TableRow className="h-10" key={point.applicationId}>
                  <TableCell className="min-w-0">
                    <div className="flex items-center gap-2">
                      <span
                        className="block truncate"
                        title={applicationDisplayName(
                          point.applicationId,
                          point.displayName,
                        )}
                      >
                        {applicationDisplayName(
                          point.applicationId,
                          point.displayName,
                        )}
                      </span>
                      {point.includesUnknown ? (
                        <Badge variant="secondary">
                          {t("quality.includesUnknown")}
                        </Badge>
                      ) : null}
                    </div>
                  </TableCell>
                  <TableCell className="text-right">
                    {formatDataSize(point.downloadBytes, locale)}
                  </TableCell>
                  <TableCell className="text-right">
                    {formatDataSize(point.uploadBytes, locale)}
                  </TableCell>
                  <TableCell className="text-right">
                    {formatDataSize(point.totalBytes, locale)}
                  </TableCell>
                </TableRow>
              ))}
              {!history?.points.length ? (
                <TableRow className="h-100">
                  <TableCell colSpan={4}>
                    <Empty className="h-full border-0">
                      <EmptyHeader>
                        <EmptyTitle>
                          {historyLoading
                            ? t("history.loading")
                            : t("history.empty")}
                        </EmptyTitle>
                      </EmptyHeader>
                    </Empty>
                  </TableCell>
                </TableRow>
              ) : null}
            </TableBody>
          </Table>
          <NetworkTablePagination
            onPageChange={setHistoryPageIndex}
            onPageSizeChange={(pageSize) => {
              setHistoryPageSize(pageSize);
              setHistoryPageIndex(0);
            }}
            pageIndex={historyPageIndex}
            pageSize={historyPageSize}
            totalCount={history?.totalCount ?? 0}
          />
        </CardContent>
      </Card>

      <AlertDialog onOpenChange={setClearOpen} open={clearOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogMedia>
              <Trash2Icon/>
            </AlertDialogMedia>
            <AlertDialogTitle>{t("clear.title")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("clear.description")}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common:actions.cancel")}</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                void clearUsage({scope: "all"}).then((result) => {
                  if (result) {
                    toast.add({title: t("clear.success"), type: "success"});
                  }
                  setClearOpen(false);
                });
              }}
              variant="destructive"
            >
              {t("clear.confirm")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
