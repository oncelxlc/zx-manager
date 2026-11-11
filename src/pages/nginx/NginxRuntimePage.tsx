import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { RefreshCwIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { NginxInstanceGate } from "src/components/nginx/instance/NginxInstanceGate";
import { NginxPageHeading } from "src/components/nginx/NginxPageHeading";
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
      <NginxPageHeading
        description={t("runtimePage.description")}
        title={t("runtimePage.title")}
      />
      <NginxInstanceGate><NginxRuntimeContent /></NginxInstanceGate>
    </div>
  );
}
