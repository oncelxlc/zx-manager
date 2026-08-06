import { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";

import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { NginxInstanceGate } from "src/components/nginx/instance/NginxInstanceGate";
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
                <span className={operation.success ? "text-muted-foreground" : "text-destructive"}>
                  {t(operation.success ? "overview.operationSucceeded" : "overview.operationFailed")}
                </span>
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
      <Breadcrumb>
        <BreadcrumbList>
          <BreadcrumbItem>Nginx</BreadcrumbItem>
          <BreadcrumbSeparator />
          <BreadcrumbItem><BreadcrumbPage>{t("overview.title")}</BreadcrumbPage></BreadcrumbItem>
        </BreadcrumbList>
      </Breadcrumb>
      <div className="flex flex-col gap-1">
        <h1 className="text-2xl font-semibold tracking-tight">{t("overview.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("overview.description")}</p>
      </div>
      <NginxInstanceGate><NginxOverviewContent /></NginxInstanceGate>
    </div>
  );
}
