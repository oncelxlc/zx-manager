import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ArrowDownIcon,
  ArrowUpIcon,
  DatabaseIcon,
  NetworkIcon,
  RefreshCwIcon,
  ShieldAlertIcon,
  Trash2Icon,
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
  CardAction,
  Card,
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
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import {
  Empty,
  EmptyHeader,
  EmptyTitle,
} from "@/components/ui/empty";
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
  getPreferences,
  setNetworkMonitorConfigured,
  setNetworkMonitorSampleInterval as persistNetworkMonitorSampleInterval,
} from "src/services/storage/preferences-storage";
import {
  NetworkTablePagination,
  type NetworkTablePageSize,
} from "src/components/network-monitor/NetworkTablePagination";
import { useMainLayoutHeader } from "src/layouts/MainLayout";
import { useNetworkMonitorStore } from "src/stores/network-monitor-store";
import type {
  QueryGroupBy,
  TrafficLayer,
} from "src/types/network-monitor";
import {
  networkMonitorSampleIntervals,
  type NetworkMonitorSampleInterval,
} from "src/types/preferences";
import { formatDataRate, formatDataSize } from "src/utils/format-data-size";

type RangePreset = "10m" | "1h" | "24h" | "7d" | "custom";

const rangeDurations: Record<Exclude<RangePreset, "custom">, number> = {
  "10m": 10 * 60 * 1_000,
  "1h": 60 * 60 * 1_000,
  "24h": 24 * 60 * 60 * 1_000,
  "7d": 7 * 24 * 60 * 60 * 1_000,
};

const groupItems: { label: string; value: QueryGroupBy }[] = [
  {label: "time", value: "time"},
  {label: "interface", value: "interface"},
  {label: "application", value: "application"},
  {label: "proxySession", value: "proxySession"},
];

const defaultPageSize: NetworkTablePageSize = 10;

function toLocalInputValue(timestamp: number) {
  const date = new Date(timestamp - new Date(timestamp).getTimezoneOffset() * 60_000);
  return date.toISOString().slice(0, 16);
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
  const [configured, setConfigured] = useState<boolean | null>(null);
  const [range, setRange] = useState<RangePreset>("1h");
  const [groupBy, setGroupBy] = useState<QueryGroupBy>("time");
  const [layers, setLayers] = useState<TrafficLayer[]>(["physical", "tunnel"]);
  const [interfaceFilter, setInterfaceFilter] = useState("all");
  const [applicationFilter, setApplicationFilter] = useState("all");
  const [proxyFilter, setProxyFilter] = useState("all");
  const [customFrom, setCustomFrom] = useState(() =>
    toLocalInputValue(Date.now() - 60 * 60 * 1_000),
  );
  const [customTo, setCustomTo] = useState(() => toLocalInputValue(Date.now()));
  const [clearOpen, setClearOpen] = useState(false);
  const [interfacePageIndex, setInterfacePageIndex] = useState(0);
  const [interfacePageSize, setInterfacePageSize] =
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
      groupBy,
      applicationIds:
        applicationFilter === "all" ? [] : [applicationFilter],
      interfaceIds: interfaceFilter === "all" ? [] : [interfaceFilter],
      interval: "auto",
      layers,
      limit: historyPageSize,
      cursor: String(historyPageIndex * historyPageSize),
      proxySessionIds: proxyFilter === "all" ? [] : [proxyFilter],
      timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC",
    });
  }, [
    applicationFilter,
    customFrom,
    customTo,
    groupBy,
    historyPageIndex,
    historyPageSize,
    interfaceFilter,
    layers,
    proxyFilter,
    queryHistory,
    range,
    t,
  ]);

  useEffect(() => {
    if (configured && status) {
      runHistoryQuery();
    }
  }, [configured, runHistoryQuery, status?.generation]);

  const latest = realtime[realtime.length - 1];
  const interfaces = latest?.interfaces ?? [];
  const interfacePageCount = Math.max(
    1,
    Math.ceil(interfaces.length / interfacePageSize),
  );
  const visibleInterfaces = interfaces.slice(
    interfacePageIndex * interfacePageSize,
    (interfacePageIndex + 1) * interfacePageSize,
  );

  useEffect(() => {
    if (interfacePageIndex >= interfacePageCount) {
      setInterfacePageIndex(interfacePageCount - 1);
    }
  }, [interfacePageCount, interfacePageIndex]);

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
    setInterfacePageIndex(0);
    setHistoryPageIndex(0);
  }, [status?.generation]);

  const chartData = useMemo(
    () => {
      const latestSampledAt =
        realtime[realtime.length - 1]?.sampledAt ?? Date.now();
      const chartFrom = latestSampledAt - rangeDurations["10m"];
      return realtime
        .filter((event) => event.sampledAt >= chartFrom)
        .map((event) => ({
        sampledAt: event.sampledAt,
        download:
          event.sampleState === "sample"
            ? event.device.downloadBytesPerSecond
            : null,
        upload:
          event.sampleState === "sample"
            ? event.device.uploadBytesPerSecond
            : null,
        }));
    },
    [realtime],
  );
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
        title: t(status?.enabled ? "toast.stopped" : "toast.started"),
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
    () => (
      <>
        <Badge variant={status?.enabled ? "default" : "secondary"}>
          {t(status?.enabled ? "status.running" : "status.disabled")}
        </Badge>
        <Button
          disabled={loading}
          onClick={() => void handleCurrentSession()}
          variant="outline"
        >
          {loading ? <Spinner data-icon="inline-start"/> : <NetworkIcon data-icon="inline-start"/>}
          {t(status?.enabled ? "actions.stopSession" : "actions.startSession")}
        </Button>
        <Button disabled={loading} onClick={() => void refresh()} variant="outline">
          <RefreshCwIcon data-icon="inline-start"/>
          {t("common:actions.refresh")}
        </Button>
        <Button onClick={() => setClearOpen(true)} variant="destructive">
          <Trash2Icon data-icon="inline-start"/>
          {t("actions.clear")}
        </Button>
      </>
    ),
    [handleCurrentSession, loading, refresh, status?.enabled, t],
  );

  useMainLayoutHeader({
    actions: headerActions,
    title: t("title"),
  });

  const summaryCards = [
    {
      label: t("metrics.downloadRate"),
      value: formatDataRate(latest?.device.downloadBytesPerSecond ?? null, locale),
    },
    {
      label: t("metrics.uploadRate"),
      value: formatDataRate(latest?.device.uploadBytesPerSecond ?? null, locale),
    },
    {
      label: t("metrics.sessionDownload"),
      value: formatDataSize(latest?.device.sessionDownloadBytes ?? 0, locale),
    },
    {
      label: t("metrics.sessionUpload"),
      value: formatDataSize(latest?.device.sessionUploadBytes ?? 0, locale),
    },
    {
      label: t("metrics.activeInterfaces"),
      value: String(
        latest?.interfaces.filter((item) => item.state === "up").length ?? 0,
      ),
    },
    {
      label: t("metrics.proxyVpn"),
      value: latest?.proxyVpn.vpnConnected
        ? t("proxy.vpnConnected")
        : latest?.proxyVpn.proxyConfigured
          ? t("proxy.proxyConfigured")
          : t("proxy.none"),
    },
  ];
  const sampleIntervalItems = networkMonitorSampleIntervals.map((interval) => ({
    label: t("sampling.option", { count: interval }),
    value: String(interval),
  }));

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
              <AlertDescription>{t("onboarding.storageDescription")}</AlertDescription>
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

      {error ? (
        <Alert variant="destructive">
          <ShieldAlertIcon/>
          <AlertTitle>{t("errors.title")}</AlertTitle>
          <AlertDescription>
            {t(`errors.codes.${error.code}`, {defaultValue: error.message})}
          </AlertDescription>
        </Alert>
      ) : null}

      {capabilities && !capabilities.applicationTraffic.available ? (
        <Alert>
          <ShieldAlertIcon/>
          <AlertTitle>{t("capabilities.degradedTitle")}</AlertTitle>
          <AlertDescription>{t("capabilities.applicationUnavailable")}</AlertDescription>
        </Alert>
      ) : null}

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-6">
        {summaryCards.map((card, index) => (
          <Card key={card.label}>
            <CardHeader>
              <CardDescription>{card.label}</CardDescription>
              <CardTitle className="flex items-center gap-2 text-xl">
                {index === 0 ? <ArrowDownIcon aria-hidden="true"/> : null}
                {index === 1 ? <ArrowUpIcon aria-hidden="true"/> : null}
                {card.value}
              </CardTitle>
            </CardHeader>
          </Card>
        ))}
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
                value={String(status?.sampleIntervalSeconds ?? 5)}
              >
                <SelectTrigger
                  className="w-32"
                  id="network-sample-interval"
                >
                  <SelectValue />
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
                tickFormatter={(value: number) => formatDataSize(value, locale)}
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

      <div className="grid gap-4 xl:grid-cols-3">
        <Card className="xl:col-span-2">
          <CardHeader>
            <CardTitle>{t("interfaces.title")}</CardTitle>
            <CardDescription>{t("interfaces.description")}</CardDescription>
          </CardHeader>
          <CardContent>
            <Table
              className="min-w-[52rem] table-fixed"
              containerClassName="max-h-[52.5rem] overflow-auto rounded-md border"
            >
              <TableHeader className="[&_th]:sticky [&_th]:top-0 [&_th]:bg-card">
                <TableRow>
                  <TableHead>{t("interfaces.columns.name")}</TableHead>
                  <TableHead className="w-32">{t("interfaces.columns.kind")}</TableHead>
                  <TableHead className="w-28">{t("interfaces.columns.layer")}</TableHead>
                  <TableHead className="w-28">{t("interfaces.columns.state")}</TableHead>
                  <TableHead className="w-40 text-right">
                    {t("interfaces.columns.download")}
                  </TableHead>
                  <TableHead className="w-40 text-right">
                    {t("interfaces.columns.upload")}
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {visibleInterfaces.map((networkInterface) => (
                  <TableRow className="h-10" key={networkInterface.id}>
                    <TableCell className="min-w-0 font-medium">
                      <span
                        className="block truncate"
                        title={networkInterface.name}
                      >
                        {networkInterface.name}
                      </span>
                    </TableCell>
                    <TableCell>{t(`interfaces.kinds.${networkInterface.kind}`)}</TableCell>
                    <TableCell>{t(`layers.${networkInterface.layer}`)}</TableCell>
                    <TableCell>
                      <Badge variant="secondary">
                        {t(`interfaces.states.${networkInterface.state}`)}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      {formatDataRate(
                        networkInterface.traffic.downloadBytesPerSecond,
                        locale,
                      )}
                    </TableCell>
                    <TableCell className="text-right">
                      {formatDataRate(
                        networkInterface.traffic.uploadBytesPerSecond,
                        locale,
                      )}
                    </TableCell>
                  </TableRow>
                ))}
                {interfaces.length === 0 ? (
                  <TableRow className="h-[25rem]">
                    <TableCell colSpan={6}>
                      <Empty className="h-full border-0">
                        <EmptyHeader>
                          <EmptyTitle>
                            {t(
                              status?.enabled
                                ? "interfaces.waiting"
                                : "interfaces.disabled",
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
              onPageChange={setInterfacePageIndex}
              onPageSizeChange={(pageSize) => {
                setInterfacePageSize(pageSize);
                setInterfacePageIndex(0);
              }}
              pageIndex={interfacePageIndex}
              pageSize={interfacePageSize}
              totalCount={interfaces.length}
            />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t("proxy.title")}</CardTitle>
            <CardDescription>{t("proxy.description")}</CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-3 text-sm">
            <div className="flex items-center justify-between gap-4">
              <span className="text-muted-foreground">{t("proxy.systemProxy")}</span>
              <Badge variant="secondary">
                {latest?.proxyVpn.proxyConfigured ? t("status.enabled") : t("status.disabled")}
              </Badge>
            </div>
            <div className="flex items-center justify-between gap-4">
              <span className="text-muted-foreground">{t("proxy.vpn")}</span>
              <Badge variant="secondary">
                {latest?.proxyVpn.vpnConnected ? t("status.connected") : t("status.disconnected")}
              </Badge>
            </div>
            <div className="flex items-center justify-between gap-4">
              <span className="text-muted-foreground">{t("proxy.tunnelDownload")}</span>
              <span>{formatDataRate(latest?.proxyVpn.traffic.downloadBytesPerSecond ?? null, locale)}</span>
            </div>
          </CardContent>
        </Card>
      </div>

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
                    setHistoryPageIndex(0);
                  }
                }}
                value={[range]}
                variant="outline"
              >
                {(["10m", "1h", "24h", "7d", "custom"] as const).map((value) => (
                  <ToggleGroupItem key={value} value={value}>
                    {t(`history.ranges.${value}`)}
                  </ToggleGroupItem>
                ))}
              </ToggleGroup>
            </Field>
            {range === "custom" ? (
              <div className="grid gap-4 md:grid-cols-2">
                <Field>
                  <FieldLabel htmlFor="network-history-from">{t("history.from")}</FieldLabel>
                  <Input
                    id="network-history-from"
                    onChange={(event) => {
                      setCustomFrom(event.target.value);
                      setHistoryPageIndex(0);
                    }}
                    type="datetime-local"
                    value={customFrom}
                  />
                </Field>
                <Field>
                  <FieldLabel htmlFor="network-history-to">{t("history.to")}</FieldLabel>
                  <Input
                    id="network-history-to"
                    onChange={(event) => {
                      setCustomTo(event.target.value);
                      setHistoryPageIndex(0);
                    }}
                    type="datetime-local"
                    value={customTo}
                  />
                </Field>
              </div>
            ) : null}
            <div className="grid gap-4 md:grid-cols-2">
              <Field>
                <FieldLabel>{t("history.groupBy")}</FieldLabel>
                <Select
                  items={groupItems.map((item) => ({
                    label: t(`history.groups.${item.label}`),
                    value: item.value,
                  }))}
                  onValueChange={(value) => {
                    setGroupBy(value as QueryGroupBy);
                    setHistoryPageIndex(0);
                  }}
                  value={groupBy}
                >
                  <SelectTrigger>
                    <SelectValue/>
                  </SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      {groupItems.map((item) => (
                        <SelectItem key={item.value} value={item.value}>
                          {t(`history.groups.${item.label}`)}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </Field>
              <Field>
                <FieldLabel>{t("history.layers")}</FieldLabel>
                <ToggleGroup
                  multiple
                  onValueChange={(values) => {
                    setLayers(values as TrafficLayer[]);
                    setHistoryPageIndex(0);
                  }}
                  value={layers}
                  variant="outline"
                >
                  {(["physical", "tunnel", "application"] as const).map((layer) => (
                    <ToggleGroupItem key={layer} value={layer}>
                      {t(`layers.${layer}`)}
                    </ToggleGroupItem>
                  ))}
                </ToggleGroup>
                <FieldDescription>{t("history.layerDescription")}</FieldDescription>
              </Field>
            </div>
            <div className="grid gap-4 md:grid-cols-3">
              <Field>
                <FieldLabel>{t("history.interfaceFilter")}</FieldLabel>
                <Select
                  items={[
                    {label: t("history.all"), value: "all"},
                    ...(latest?.interfaces ?? []).map((item) => ({
                      label: item.name,
                      value: item.id,
                    })),
                  ]}
                  onValueChange={(value) => {
                    setInterfaceFilter(String(value));
                    setHistoryPageIndex(0);
                  }}
                  value={interfaceFilter}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue/>
                  </SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      <SelectItem value="all">{t("history.all")}</SelectItem>
                      {(latest?.interfaces ?? []).map((item) => (
                        <SelectItem key={item.id} value={item.id}>
                          {item.name}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </Field>
              <Field data-disabled>
                <FieldLabel>{t("history.applicationFilter")}</FieldLabel>
                <Select
                  disabled
                  items={[
                    {
                      label: t("history.applicationUnavailable"),
                      value: "all",
                    },
                  ]}
                  onValueChange={(value) => {
                    setApplicationFilter(String(value));
                    setHistoryPageIndex(0);
                  }}
                  value={applicationFilter}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue/>
                  </SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      <SelectItem value="all">
                        {t("history.applicationUnavailable")}
                      </SelectItem>
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </Field>
              <Field>
                <FieldLabel>{t("history.proxyFilter")}</FieldLabel>
                <Select
                  items={[
                    {label: t("history.all"), value: "all"},
                    {label: t("history.vpn"), value: "vpn"},
                  ]}
                  onValueChange={(value) => {
                    setProxyFilter(String(value));
                    setHistoryPageIndex(0);
                  }}
                  value={proxyFilter}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue/>
                  </SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      <SelectItem value="all">{t("history.all")}</SelectItem>
                      <SelectItem value="vpn">{t("history.vpn")}</SelectItem>
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </Field>
            </div>
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
              {historyLoading ? <Spinner data-icon="inline-start"/> :
                <RefreshCwIcon data-icon="inline-start"/>}
              {t("history.query")}
            </Button>
          </div>
          <Table
            className="min-w-[50rem] table-fixed"
            containerClassName="max-h-[52.5rem] overflow-auto rounded-md border"
          >
            <TableHeader className="[&_th]:sticky [&_th]:top-0 [&_th]:bg-card">
              <TableRow>
                <TableHead className="w-48">{t("history.columns.time")}</TableHead>
                <TableHead>{t("history.columns.group")}</TableHead>
                <TableHead className="w-32">{t("history.columns.layer")}</TableHead>
                <TableHead className="w-40 text-right">
                  {t("history.columns.download")}
                </TableHead>
                <TableHead className="w-40 text-right">
                  {t("history.columns.upload")}
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(history?.points ?? []).map((point) => (
                <TableRow
                  className="h-10"
                  key={`${point.from}-${point.groupId}-${point.layer}`}
                >
                  <TableCell>{new Intl.DateTimeFormat(locale, {
                    dateStyle: "short",
                    timeStyle: "short",
                  }).format(point.from)}</TableCell>
                  <TableCell className="min-w-0">
                    <span className="block truncate" title={point.groupId}>
                      {point.groupId}
                    </span>
                  </TableCell>
                  <TableCell>{t(`layers.${point.layer}`)}</TableCell>
                  <TableCell className="text-right">
                    {formatDataSize(point.downloadBytes, locale)}
                  </TableCell>
                  <TableCell className="text-right">
                    {formatDataSize(point.uploadBytes, locale)}
                  </TableCell>
                </TableRow>
              ))}
              {!history?.points.length ? (
                <TableRow className="h-[25rem]">
                  <TableCell colSpan={5}>
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
            <AlertDialogDescription>{t("clear.description")}</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common:actions.cancel")}</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                void clearUsage({scope: "all"}).then((result) => {
                  if (result) {
                    toast.add({title: t("clear.success"), type: "success"});
                  }
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
