import { useTranslation } from "react-i18next";

import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@/components/ui/empty";
import { Spinner } from "@/components/ui/spinner";
import { NginxDiagnostics } from "src/components/nginx/NginxDiagnostics";
import { NginxSitesTable } from "src/components/nginx/NginxSitesTable";
import { NginxTopology } from "src/components/nginx/NginxTopology";
import { useMainLayoutHeader } from "src/layouts/MainLayout";
import { useNginxConfigurationStore } from "src/stores/nginx-configuration-store";
import { useNginxConfigurationPage } from "./useNginxConfigurationPage";
import { nginxErrorTranslationKey } from "src/utils/nginx-error";

export function NginxSitesPage() {
  const { t } = useTranslation("nginx");
  useNginxConfigurationPage();
  const configuration = useNginxConfigurationStore((state) => state.configuration);
  const loadStatus = useNginxConfigurationStore((state) => state.loadStatus);
  const error = useNginxConfigurationStore((state) => state.error);
  useMainLayoutHeader({ title: t("sites.title") });

  return (
    <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">{t("sites.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("sites.description")}</p>
      </div>
      {loadStatus === "loading" ? (
        <div className="flex min-h-48 items-center justify-center"><Spinner /></div>
      ) : null}
      {loadStatus === "error" ? (
        <Empty className="border">
          <EmptyHeader>
            <EmptyTitle>{t("configuration.errorTitle")}</EmptyTitle>
            <EmptyDescription>{t(nginxErrorTranslationKey(error?.code))}</EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : null}
      {configuration ? (
        <>
          <NginxDiagnostics diagnostics={configuration.diagnostics} />
          {configuration.sites.length > 0 ? (
            <NginxSitesTable instanceId={configuration.instanceId} sites={configuration.sites} />
          ) : (
            <Empty className="border">
              <EmptyHeader>
                <EmptyTitle>{t("sites.emptyTitle")}</EmptyTitle>
                <EmptyDescription>{t("sites.emptyDescription")}</EmptyDescription>
              </EmptyHeader>
            </Empty>
          )}
          <NginxTopology
            edges={configuration.topologyEdges}
            nodes={configuration.topologyNodes}
          />
        </>
      ) : null}
    </div>
  );
}
