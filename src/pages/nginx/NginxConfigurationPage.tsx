import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router";

import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@/components/ui/empty";
import { Spinner } from "@/components/ui/spinner";
import { NginxDiagnostics } from "src/components/nginx/NginxDiagnostics";
import { NginxSourceViewer } from "src/components/nginx/NginxSourceViewer";
import { useMainLayoutHeader } from "src/layouts/MainLayout";
import { useNginxConfigurationStore } from "src/stores/nginx-configuration-store";
import { useNginxConfigurationPage } from "./useNginxConfigurationPage";
import { nginxErrorTranslationKey } from "src/utils/nginx-error";

export function NginxConfigurationPage() {
  const { t } = useTranslation("nginx");
  const [params] = useSearchParams();
  const initialInstanceId = params.get("instance");
  const requestedSource = params.get("source");
  const requestedLine = Number(params.get("line")) || null;
  const [chosenSource, setChosenSource] = useState<string | null>(null);
  useNginxConfigurationPage(initialInstanceId);
  const configuration = useNginxConfigurationStore((state) => state.configuration);
  const loadStatus = useNginxConfigurationStore((state) => state.loadStatus);
  const error = useNginxConfigurationStore((state) => state.error);
  const sourceId = chosenSource
    ?? requestedSource
    ?? configuration?.entrySourceId
    ?? "";
  useMainLayoutHeader({ title: t("configuration.title") });

  return (
    <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">{t("configuration.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("configuration.description")}</p>
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
          <NginxSourceViewer
            focusLine={requestedSource === sourceId ? requestedLine : null}
            onSourceChange={setChosenSource}
            sourceId={sourceId}
            sources={configuration.sources}
          />
        </>
      ) : null}
    </div>
  );
}
