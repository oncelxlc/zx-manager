import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
  AppWindowIcon,
  CheckCircle2Icon,
  CpuIcon,
  DatabaseIcon,
  HardDriveIcon,
  MemoryStickIcon,
  MonitorCogIcon,
  TriangleAlertIcon,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Progress } from "@/components/ui/progress";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  formatBytes,
  formatDuration,
  formatFrequency,
  formatPercent,
} from "src/utils/system-information";
import type { SystemInformation } from "src/types/system-information";

function DetailRow({
  label,
  value,
}: {
  label: string;
  value: ReactNode;
}) {
  return (
    <div className="grid gap-1 border-b py-2 last:border-b-0 sm:grid-cols-[minmax(8rem,0.7fr)_minmax(0,1fr)] sm:gap-4">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0 break-words font-medium sm:text-right">{value}</dd>
    </div>
  );
}

function SectionCard({
  children,
  description,
  icon,
  title,
}: {
  children: ReactNode;
  description: string;
  icon: ReactNode;
  title: string;
}) {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-start gap-3">
          <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
            {icon}
          </div>
          <div className="min-w-0">
            <CardTitle>{title}</CardTitle>
            <CardDescription>{description}</CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent>{children}</CardContent>
    </Card>
  );
}

function SummaryCard({
  detail,
  icon,
  label,
  progress,
  value,
}: {
  detail: string;
  icon: ReactNode;
  label: string;
  progress?: number | null;
  value: string;
}) {
  return (
    <Card>
      <CardHeader className="flex-row items-center justify-between">
        <CardDescription>{label}</CardDescription>
        <span className="text-muted-foreground">{icon}</span>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <p className="truncate text-2xl font-semibold tracking-tight">{value}</p>
        {progress !== undefined && progress !== null ? (
          <Progress aria-label={label} value={progress} />
        ) : null}
        <p className="truncate text-xs text-muted-foreground">{detail}</p>
      </CardContent>
    </Card>
  );
}

export function SystemInformationContent({
  information,
}: {
  information: SystemInformation;
}) {
  const { i18n, t } = useTranslation(["systemInformation", "common"]);
  const locale = i18n.resolvedLanguage ?? i18n.language;
  const unavailable = t("unavailable");
  const durationLabels = {
    day: t("units.day"),
    hour: t("units.hour"),
    minute: t("units.minute"),
  };
  const storageTotal = information.disks.reduce(
    (total, disk) => total + (disk.totalBytes ?? 0),
    0,
  );
  const storageUsed = information.disks.reduce(
    (total, disk) => total + (disk.usedBytes ?? 0),
    0,
  );
  const storageUsage =
    storageTotal > 0 ? (storageUsed / storageTotal) * 100 : null;

  return (
    <div className="flex flex-col gap-5">
      <section
        aria-label={t("summary.title")}
        className="grid gap-4 md:grid-cols-2 xl:grid-cols-4"
      >
        <SummaryCard
          detail={information.system.architecture ?? unavailable}
          icon={<MonitorCogIcon aria-hidden="true" />}
          label={t("summary.operatingSystem")}
          value={
            information.system.osLongVersion
            ?? information.system.osName
            ?? unavailable
          }
        />
        <SummaryCard
          detail={information.cpu.model ?? unavailable}
          icon={<CpuIcon aria-hidden="true" />}
          label={t("summary.cpuUsage")}
          progress={information.cpu.usagePercent}
          value={formatPercent(information.cpu.usagePercent, locale, unavailable)}
        />
        <SummaryCard
          detail={t("summary.memoryDetail", {
            total: formatBytes(
              information.memory.totalBytes,
              locale,
              unavailable,
            ),
          })}
          icon={<MemoryStickIcon aria-hidden="true" />}
          label={t("summary.memoryUsage")}
          progress={information.memory.usagePercent}
          value={formatBytes(
            information.memory.usedBytes,
            locale,
            unavailable,
          )}
        />
        <SummaryCard
          detail={t("summary.diskCount", { count: information.disks.length })}
          icon={<HardDriveIcon aria-hidden="true" />}
          label={t("summary.storageUsage")}
          progress={storageUsage}
          value={formatBytes(
            information.disks.length > 0 ? storageUsed : null,
            locale,
            unavailable,
          )}
        />
      </section>

      <div className="grid gap-5 xl:grid-cols-2">
        <SectionCard
          description={t("sections.system.description")}
          icon={<MonitorCogIcon aria-hidden="true" />}
          title={t("sections.system.title")}
        >
          <dl>
            <DetailRow label={t("fields.platform")} value={t(`platforms.${information.system.platform}`)} />
            <DetailRow label={t("fields.osVersion")} value={information.system.osLongVersion ?? information.system.osVersion ?? unavailable} />
            <DetailRow label={t("fields.kernelVersion")} value={information.system.kernelVersion ?? unavailable} />
            <DetailRow label={t("fields.architecture")} value={information.system.architecture ?? unavailable} />
            <DetailRow label={t("fields.hostName")} value={information.system.hostName ?? unavailable} />
            <DetailRow
              label={t("fields.uptime")}
              value={formatDuration(
                information.system.uptimeSeconds,
                locale,
                unavailable,
                durationLabels,
              )}
            />
          </dl>
        </SectionCard>

        <SectionCard
          description={t("sections.cpu.description")}
          icon={<CpuIcon aria-hidden="true" />}
          title={t("sections.cpu.title")}
        >
          <dl>
            <DetailRow label={t("fields.model")} value={information.cpu.model ?? unavailable} />
            <DetailRow label={t("fields.vendor")} value={information.cpu.vendor ?? unavailable} />
            <DetailRow label={t("fields.usage")} value={formatPercent(information.cpu.usagePercent, locale, unavailable)} />
            <DetailRow label={t("fields.physicalCores")} value={information.cpu.physicalCoreCount ?? unavailable} />
            <DetailRow label={t("fields.logicalCores")} value={information.cpu.logicalCoreCount ?? unavailable} />
            <DetailRow label={t("fields.frequency")} value={formatFrequency(information.cpu.frequencyMhz, locale, unavailable)} />
          </dl>
        </SectionCard>

        <SectionCard
          description={t("sections.memory.description")}
          icon={<MemoryStickIcon aria-hidden="true" />}
          title={t("sections.memory.title")}
        >
          <div className="flex flex-col gap-5">
            <Progress
              aria-label={t("fields.memoryUsage")}
              value={information.memory.usagePercent ?? 0}
            />
            <dl>
              <DetailRow label={t("fields.total")} value={formatBytes(information.memory.totalBytes, locale, unavailable)} />
              <DetailRow label={t("fields.used")} value={formatBytes(information.memory.usedBytes, locale, unavailable)} />
              <DetailRow label={t("fields.available")} value={formatBytes(information.memory.availableBytes, locale, unavailable)} />
              <DetailRow label={t("fields.swapTotal")} value={formatBytes(information.memory.swapTotalBytes, locale, unavailable)} />
              <DetailRow label={t("fields.swapUsed")} value={formatBytes(information.memory.swapUsedBytes, locale, unavailable)} />
            </dl>
          </div>
        </SectionCard>

        <SectionCard
          description={t("sections.runtime.description")}
          icon={<AppWindowIcon aria-hidden="true" />}
          title={t("sections.runtime.title")}
        >
          <dl>
            <DetailRow label={t("fields.application")} value={`${information.runtime.appName} ${information.runtime.appVersion}`} />
            <DetailRow label={t("fields.tauriVersion")} value={information.runtime.tauriVersion} />
            <DetailRow label={t("fields.targetTriple")} value={information.runtime.targetTriple ?? unavailable} />
            <DetailRow label={t("fields.webviewVersion")} value={information.runtime.webviewVersion ?? unavailable} />
          </dl>
        </SectionCard>
      </div>

      <SectionCard
        description={t("sections.gpu.description")}
        icon={<DatabaseIcon aria-hidden="true" />}
        title={t("sections.gpu.title")}
      >
        {information.gpus.length === 0 ? (
          <Empty className="border">
            <EmptyHeader>
              <EmptyMedia variant="icon"><DatabaseIcon /></EmptyMedia>
              <EmptyTitle>{t("empty.gpuTitle")}</EmptyTitle>
              <EmptyDescription>{t("empty.gpuDescription")}</EmptyDescription>
            </EmptyHeader>
          </Empty>
        ) : (
          <div className="grid gap-4 lg:grid-cols-2">
            {information.gpus.map((gpu, index) => (
              <Card key={`${gpu.name}-${gpu.backend}-${index}`} size="sm">
                <CardHeader>
                  <CardTitle>{gpu.name}</CardTitle>
                  <CardDescription>
                    {t(`gpuTypes.${gpu.deviceType}`)} · {t(`gpuBackends.${gpu.backend}`)}
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <dl>
                    <DetailRow label={t("fields.driver")} value={gpu.driver ?? unavailable} />
                    <DetailRow label={t("fields.vendorId")} value={gpu.vendorId ?? unavailable} />
                    <DetailRow label={t("fields.deviceId")} value={gpu.deviceId ?? unavailable} />
                    <DetailRow label={t("fields.dedicatedMemory")} value={formatBytes(gpu.dedicatedMemoryBytes, locale, unavailable)} />
                  </dl>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </SectionCard>

      <SectionCard
        description={t("sections.storage.description")}
        icon={<HardDriveIcon aria-hidden="true" />}
        title={t("sections.storage.title")}
      >
        {information.disks.length === 0 ? (
          <Empty className="border">
            <EmptyHeader>
              <EmptyMedia variant="icon"><HardDriveIcon /></EmptyMedia>
              <EmptyTitle>{t("empty.storageTitle")}</EmptyTitle>
              <EmptyDescription>{t("empty.storageDescription")}</EmptyDescription>
            </EmptyHeader>
          </Empty>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("fields.disk")}</TableHead>
                <TableHead>{t("fields.mountPoint")}</TableHead>
                <TableHead>{t("fields.fileSystem")}</TableHead>
                <TableHead>{t("fields.kind")}</TableHead>
                <TableHead>{t("fields.used")}</TableHead>
                <TableHead>{t("fields.available")}</TableHead>
                <TableHead>{t("fields.total")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {information.disks.map((disk, index) => (
                <TableRow key={`${disk.mountPoint ?? disk.name}-${index}`}>
                  <TableCell className="font-medium">{disk.name ?? unavailable}</TableCell>
                  <TableCell>{disk.mountPoint ?? unavailable}</TableCell>
                  <TableCell>{disk.fileSystem ?? unavailable}</TableCell>
                  <TableCell>{disk.kind}</TableCell>
                  <TableCell>{formatBytes(disk.usedBytes, locale, unavailable)}</TableCell>
                  <TableCell>{formatBytes(disk.availableBytes, locale, unavailable)}</TableCell>
                  <TableCell>{formatBytes(disk.totalBytes, locale, unavailable)}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </SectionCard>

      <SectionCard
        description={t("sections.availability.description")}
        icon={<CheckCircle2Icon aria-hidden="true" />}
        title={t("sections.availability.title")}
      >
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          {Object.entries(information.availability).map(([key, available]) => (
            <div key={key} className="flex items-center justify-between gap-3 rounded-lg border p-3">
              <span>{t(`availability.${key}`)}</span>
              <Badge variant={available ? "success" : "muted"}>
                {available ? t("available") : t("unavailable")}
              </Badge>
            </div>
          ))}
        </div>
        {information.warnings.length > 0 ? (
          <div className="mt-4 flex flex-col gap-2 rounded-lg border border-warning/30 bg-warning/5 p-4">
            <div className="flex items-center gap-2 font-medium text-warning">
              <TriangleAlertIcon aria-hidden="true" />
              {t("warnings.title")}
            </div>
            <ul className="list-disc space-y-1 pl-5 text-sm text-muted-foreground">
              {information.warnings.map((warning) => (
                <li key={warning}>{t(`warnings.codes.${warning}`)}</li>
              ))}
            </ul>
          </div>
        ) : null}
      </SectionCard>
    </div>
  );
}
