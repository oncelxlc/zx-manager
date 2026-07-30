import { DatabaseIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

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
import type { SystemInformation } from "src/types/system-information";
import { formatBytes } from "src/utils/system-information";

export function SystemGpuSection({ information, locale, unavailable }: {
  information: SystemInformation;
  locale: string;
  unavailable: string;
}) {
  const { t } = useTranslation("systemInformation");
  return (
    <Card>
      <CardHeader><div className="flex items-start gap-3"><div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground"><DatabaseIcon aria-hidden="true" /></div><div><CardTitle>{t("sections.gpu.title")}</CardTitle><CardDescription>{t("sections.gpu.description")}</CardDescription></div></div></CardHeader>
      <CardContent>{information.gpus.length === 0 ? (
        <Empty className="border"><EmptyHeader><EmptyMedia variant="icon"><DatabaseIcon /></EmptyMedia><EmptyTitle>{t("empty.gpuTitle")}</EmptyTitle><EmptyDescription>{t("empty.gpuDescription")}</EmptyDescription></EmptyHeader></Empty>
      ) : <div className="grid gap-4 lg:grid-cols-2">{information.gpus.map((gpu, index) => (
        <Card key={`${gpu.name}-${gpu.backend}-${index}`} size="sm"><CardHeader><CardTitle>{gpu.name}</CardTitle><CardDescription>{t(`gpuTypes.${gpu.deviceType}`)} · {t(`gpuBackends.${gpu.backend}`)}</CardDescription></CardHeader><CardContent><dl className="grid gap-2 text-sm"><div>{t("fields.driver")}: {gpu.driver ?? unavailable}</div><div>{t("fields.vendorId")}: {gpu.vendorId ?? unavailable}</div><div>{t("fields.deviceId")}: {gpu.deviceId ?? unavailable}</div><div>{t("fields.dedicatedMemory")}: {formatBytes(gpu.dedicatedMemoryBytes, locale, unavailable)}</div></dl></CardContent></Card>
      ))}</div>}</CardContent>
    </Card>
  );
}
