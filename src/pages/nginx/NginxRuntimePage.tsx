import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { RefreshCwIcon } from "lucide-react";

import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb";
import { Button } from "@/components/ui/button";
import { NginxInstanceGate } from "src/components/nginx/instance/NginxInstanceGate";
import { NginxControlActions } from "src/components/nginx/runtime/NginxControlActions";
import { NginxMetricsUnavailable } from "src/components/nginx/runtime/NginxMetricsUnavailable";
import { NginxProcessTable } from "src/components/nginx/runtime/NginxProcessTable";
import { NginxRuntimeSummary } from "src/components/nginx/runtime/NginxRuntimeSummary";
import { useMainLayoutHeader } from "src/layouts/MainLayout";
import { useNginxStore } from "src/stores/nginx-store";

function NginxRuntimeContent() {
  const instance = useNginxStore((state) => state.instance);
  const runtime = useNginxStore((state) => state.runtimeDetails);
  if (!instance) return null;
  return (
    <div className="flex flex-col gap-5">
      <NginxRuntimeSummary instance={instance} runtime={runtime} />
      <NginxProcessTable processes={runtime?.processes ?? []} />
      {runtime?.listenerMetrics !== "available" ? <NginxMetricsUnavailable /> : null}
    </div>
  );
}

export function NginxRuntimePage() {
  const { t } = useTranslation("nginx");
  const refresh = useNginxStore((state) => state.refreshRuntimeDetails);
  const actions = useMemo(() => (
    <div className="flex flex-wrap items-center gap-2">
      <Button onClick={() => void refresh()} size="sm" variant="outline">
        <RefreshCwIcon data-icon="inline-start" />{t("common:actions.refresh")}
      </Button>
      <NginxControlActions />
    </div>
  ), [refresh, t]);
  useMainLayoutHeader({ actions, title: t("runtimePage.title") });

  return (
    <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4">
      <Breadcrumb>
        <BreadcrumbList>
          <BreadcrumbItem>Nginx</BreadcrumbItem>
          <BreadcrumbSeparator />
          <BreadcrumbItem><BreadcrumbPage>{t("runtimePage.title")}</BreadcrumbPage></BreadcrumbItem>
        </BreadcrumbList>
      </Breadcrumb>
      <div className="flex flex-col gap-1">
        <h1 className="text-2xl font-semibold tracking-tight">{t("runtimePage.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("runtimePage.description")}</p>
      </div>
      <NginxInstanceGate><NginxRuntimeContent /></NginxInstanceGate>
    </div>
  );
}
