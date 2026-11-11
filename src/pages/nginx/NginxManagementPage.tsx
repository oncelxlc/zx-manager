import { useTranslation } from "react-i18next";
import { FileCodeIcon, FileTextIcon, NetworkIcon, ServerIcon } from "lucide-react";
import { Link } from "react-router";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { useMainLayoutHeader } from "src/layouts/MainLayout";
import { useNginxStore } from "src/stores/nginx-store";
import { NginxUpdateCard } from "src/components/nginx/NginxUpdateCard";
import { NginxInstanceGate } from "src/components/nginx/instance/NginxInstanceGate";
import { NginxRuntimeBadge } from "src/components/nginx/NginxRuntimeBadge";

export function NginxManagementPage() {
  const { t } = useTranslation("nginx");
  const instance = useNginxStore((state) => state.instance);

  useMainLayoutHeader({ title: t("management.title") });

  return (
    <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4">
      <div className="flex flex-col gap-1">
        <h1 className="text-2xl font-semibold tracking-tight">
          {t("management.title")}
        </h1>
        <p className="text-sm text-muted-foreground">
          {t("management.description")}
        </p>
      </div>

      <NginxInstanceGate>
        <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <ServerIcon className="text-muted-foreground" />
            <CardTitle>{t("management.instancesTitle")}</CardTitle>
            <CardDescription>
              {instance
                ? t("management.instanceReady", { version: instance.version })
                : t("management.instanceEmpty")}
            </CardDescription>
            <CardAction>
              <Button
                nativeButton={false}
                render={<Link to="/nginx/manage/instances" />}
                variant="outline"
              >
                {t("management.openInstances")}
              </Button>
            </CardAction>
          </CardHeader>
          <CardContent className="text-sm text-muted-foreground">
            {instance ? (
              <NginxRuntimeBadge status={instance.runtimeStatus} />
            ) : t("management.registryBoundary")}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <FileTextIcon className="text-muted-foreground" />
            <CardTitle>{t("management.logsTitle")}</CardTitle>
            <CardDescription>{t("management.logsDescription")}</CardDescription>
            <CardAction>
              <Button
                nativeButton={false}
                render={<Link to="/nginx/logs" />}
                variant="outline"
              >
                {t("management.openLogs")}
              </Button>
            </CardAction>
          </CardHeader>
          <CardContent className="text-sm text-muted-foreground">
            {t("management.noRealtimeClaim")}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <NetworkIcon className="text-muted-foreground" />
            <CardTitle>{t("management.sitesTitle")}</CardTitle>
            <CardDescription>{t("management.sitesDescription")}</CardDescription>
            <CardAction>
              <Button
                nativeButton={false}
                render={<Link to="/nginx/manage/sites" />}
                variant="outline"
              >
                {t("management.openSites")}
              </Button>
            </CardAction>
          </CardHeader>
          <CardContent className="text-sm text-muted-foreground">
            {t("management.topologyBoundary")}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <FileCodeIcon className="text-muted-foreground" />
            <CardTitle>{t("management.configurationTitle")}</CardTitle>
            <CardDescription>{t("management.configurationDescription")}</CardDescription>
            <CardAction>
              <Button
                nativeButton={false}
                render={<Link to="/nginx/manage/configuration" />}
                variant="outline"
              >
                {t("management.openConfiguration")}
              </Button>
            </CardAction>
          </CardHeader>
          <CardContent className="text-sm text-muted-foreground">
            {t("management.sourceBoundary")}
          </CardContent>
        </Card>
        </div>
        <NginxUpdateCard />
      </NginxInstanceGate>
    </div>
  );
}
