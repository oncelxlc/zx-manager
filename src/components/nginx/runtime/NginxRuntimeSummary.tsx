import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { NginxInstance, NginxRuntimeDetails } from "src/types/nginx";

interface NginxRuntimeSummaryProps {
  instance: NginxInstance;
  runtime: NginxRuntimeDetails | null;
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`;
  if (value < 1024 ** 2) return `${(value / 1024).toFixed(1)} KiB`;
  return `${(value / 1024 ** 2).toFixed(1)} MiB`;
}

function formatDuration(value: number | null) {
  if (value === null) return "—";
  const hours = Math.floor(value / 3600);
  const minutes = Math.floor((value % 3600) / 60);
  return `${hours}h ${minutes}m`;
}

export function NginxRuntimeSummary({ instance, runtime }: NginxRuntimeSummaryProps) {
  const { t } = useTranslation("nginx");
  const values = [
    { label: t("runtimeDetails.masterPid"), value: runtime?.masterPid ?? "—" },
    { label: t("runtimeDetails.workers"), value: runtime?.workerCount ?? "—" },
    { label: t("runtimeDetails.cpu"), value: runtime ? `${runtime.totalCpuUsage.toFixed(1)}%` : "—" },
    { label: t("runtimeDetails.memory"), value: runtime ? formatBytes(runtime.totalMemoryBytes) : "—" },
    { label: t("runtimeDetails.uptime"), value: formatDuration(runtime?.uptimeSeconds ?? null) },
    { label: t("runtimeDetails.version"), value: instance.version },
  ];

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          {t("runtimeDetails.summary")}
          <Badge variant={instance.runtimeStatus === "running" ? "success" : "secondary"}>
            {t(`runtime.${instance.runtimeStatus}`)}
          </Badge>
        </CardTitle>
      </CardHeader>
      <CardContent className="grid gap-3 sm:grid-cols-2 xl:grid-cols-6">
        {values.map((item) => (
          <div className="rounded-lg border p-3" key={item.label}>
            <p className="text-xs text-muted-foreground">{item.label}</p>
            <p className="mt-1 font-medium tabular-nums">{item.value}</p>
          </div>
        ))}
      </CardContent>
    </Card>
  );
}
