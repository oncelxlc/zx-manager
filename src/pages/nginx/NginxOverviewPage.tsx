import { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { CircleCheckIcon, TriangleAlertIcon } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { NginxInstanceGate } from "src/components/nginx/instance/NginxInstanceGate";
import { NginxPageHeading } from "src/components/nginx/NginxPageHeading";
import { NginxGlobalConfigCard } from "src/components/nginx/overview/NginxGlobalConfigCard";
import { NginxUpdateCard } from "src/components/nginx/NginxUpdateCard";
import { NginxControlActions } from "src/components/nginx/runtime/NginxControlActions";
import { NginxMetricsUnavailable } from "src/components/nginx/runtime/NginxMetricsUnavailable";
import { NginxRuntimeSummary } from "src/components/nginx/runtime/NginxRuntimeSummary";
import { useMainLayoutHeader } from "src/layouts/MainLayout";
import { useNginxStore } from "src/stores/nginx-store";

function NginxOverviewContent() {
  const { t } = useTranslation("nginx");
  const instance = useNginxStore((state) => state.instance);
  const runtime = useNginxStore((state) => state.runtimeDetails);
  const operations = useNginxStore((state) => state.operationHistory);
  const loadOperations = useNginxStore((state) => state.loadOperationHistory);

  useEffect(() => {
    void loadOperations();
  }, [loadOperations]);

  if (!instance) return null;
  return (
    <div className="flex flex-col gap-5">
      <NginxRuntimeSummary instance={instance} runtime={runtime} />
      <NginxGlobalConfigCard instance={instance} />
      <div className="grid gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>{t("overview.configurationHealth")}</CardTitle>
            <CardDescription>{t("overview.configurationHealthDescription")}</CardDescription>
          </CardHeader>
          <CardContent className="grid gap-3 text-sm">
            <div className="flex justify-between gap-3">
              <span className="text-muted-foreground">{t("overview.configPath")}</span>
              <span className="max-w-[70%] truncate font-mono text-xs" title={instance.configPath ?? undefined}>
                {instance.configPath ?? t("overview.unavailable")}
              </span>
            </div>
            <div className="flex justify-between gap-3">
              <span className="text-muted-foreground">{t("overview.executableIdentity")}</span>
              <span>{t(`lifecycle.${instance.lifecycleState}`)}</span>
            </div>
            <div className="flex justify-between gap-3">
              <span className="text-muted-foreground">{t("overview.authorization")}</span>
              <span>{t(`authorization.${instance.authorizationLevel}`)}</span>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>{t("overview.recentOperations")}</CardTitle>
            <CardDescription>{t("overview.recentOperationsDescription")}</CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-2 text-sm">
            {operations.slice(0, 5).map((operation) => (
              <div className="flex items-center justify-between gap-3 rounded-lg border p-3" key={operation.id}>
                <span>{t(`control.${operation.action}Label`)}</span>
                <Badge
                  aria-label={t(operation.success ? "overview.operationSucceeded" : "overview.operationFailed")}
                  variant={operation.success ? "success" : "destructive"}
                >
                  {operation.success ? <CircleCheckIcon /> : <TriangleAlertIcon />}
                  {t(operation.success ? "overview.operationSucceeded" : "overview.operationFailed")}
                </Badge>
              </div>
            ))}
            {operations.length === 0 ? (
              <p className="text-muted-foreground">{t("overview.noOperations")}</p>
            ) : null}
          </CardContent>
        </Card>
      </div>
      {runtime?.connectionMetrics !== "available" ? <NginxMetricsUnavailable /> : null}
      <NginxUpdateCard />
    </div>
  );
}

export function NginxOverviewPage() {
  const { t } = useTranslation("nginx");
  const actions = useMemo(() => <NginxControlActions />, []);
  useMainLayoutHeader({ actions, title: t("overview.title") });

  return (
    <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4">
      <NginxPageHeading description={t("overview.description")} title={t("overview.title")} />
      <NginxInstanceGate><NginxOverviewContent /></NginxInstanceGate>
    </div>
  );
}
