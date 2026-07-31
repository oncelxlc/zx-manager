import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { FileTextIcon, ServerIcon } from "lucide-react";
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

export function NginxManagementPage() {
  const { t } = useTranslation("nginx");
  const instances = useNginxStore((state) => state.instances);
  const loadInstances = useNginxStore((state) => state.loadInstances);

  useEffect(() => {
    void loadInstances();
  }, [loadInstances]);

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

      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <ServerIcon className="size-5 text-muted-foreground" />
            <CardTitle>{t("management.instancesTitle")}</CardTitle>
            <CardDescription>
              {t("management.instancesDescription", { count: instances.length })}
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
            {t("management.registryBoundary")}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <FileTextIcon className="size-5 text-muted-foreground" />
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
      </div>
    </div>
  );
}
