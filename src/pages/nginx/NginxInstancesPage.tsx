import { useCallback, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { PlusIcon, RefreshCwIcon, ServerCrashIcon, ServerIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Spinner } from "@/components/ui/spinner";
import { toast } from "@/components/ui/toast";
import { NginxInstanceTable } from "src/components/nginx/NginxInstanceTable";
import { NginxRegistrationDialog } from "src/components/nginx/NginxRegistrationDialog";
import { useMainLayoutHeader } from "src/layouts/MainLayout";
import { useNginxStore } from "src/stores/nginx-store";

export function NginxInstancesPage() {
  const { t } = useTranslation(["nginx", "common"]);
  const instances = useNginxStore((state) => state.instances);
  const inspection = useNginxStore((state) => state.inspection);
  const loadStatus = useNginxStore((state) => state.loadStatus);
  const operationStatus = useNginxStore((state) => state.operationStatus);
  const error = useNginxStore((state) => state.error);
  const loadInstances = useNginxStore((state) => state.loadInstances);
  const selectAndInspect = useNginxStore((state) => state.selectAndInspect);
  const registerInspection = useNginxStore((state) => state.registerInspection);
  const refreshInstance = useNginxStore((state) => state.refreshInstance);
  const unregisterInstance = useNginxStore((state) => state.unregisterInstance);
  const clearInspection = useNginxStore((state) => state.clearInspection);
  const loading = loadStatus === "loading" || operationStatus === "loading";

  useEffect(() => {
    void loadInstances();
  }, [loadInstances]);

  const showCurrentError = useCallback(() => {
    const error = useNginxStore.getState().error;
    toast.add({
      title: t("nginx:toast.errorTitle"),
      description: t(`nginx:errors.${error?.code ?? "NGINX_UNKNOWN"}`),
      type: "error",
    });
  }, [t]);

  const handleSelect = useCallback(async () => {
    const result = await selectAndInspect();
    if (!result && useNginxStore.getState().operationStatus === "error") {
      showCurrentError();
    }
  }, [selectAndInspect, showCurrentError]);

  const headerActions = useMemo(() => (
    <>
      <Button
        disabled={loading}
        onClick={() => void loadInstances(true)}
        variant="outline"
      >
        {loadStatus === "loading" ? (
          <Spinner data-icon="inline-start" />
        ) : (
          <RefreshCwIcon data-icon="inline-start" />
        )}
        {t("common:actions.refresh")}
      </Button>
      <Button disabled={loading} onClick={() => void handleSelect()}>
        <PlusIcon data-icon="inline-start" />
        {t("nginx:instances.add")}
      </Button>
    </>
  ), [handleSelect, loadInstances, loadStatus, loading, t]);

  useMainLayoutHeader({ actions: headerActions, title: t("nginx:instances.title") });

  return (
    <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4">
      <div className="flex flex-col gap-1">
        <h1 className="text-2xl font-semibold tracking-tight">
          {t("nginx:instances.title")}
        </h1>
        <p className="text-sm text-muted-foreground">
          {t("nginx:instances.description")}
        </p>
      </div>

      {instances.length === 0 && loadStatus === "error" ? (
        <Empty className="min-h-80 border">
          <EmptyHeader>
            <EmptyMedia variant="icon"><ServerCrashIcon /></EmptyMedia>
            <EmptyTitle>{t("nginx:instances.errorTitle")}</EmptyTitle>
            <EmptyDescription>
              {t(`nginx:errors.${error?.code ?? "NGINX_UNKNOWN"}`)}
            </EmptyDescription>
          </EmptyHeader>
          <EmptyContent>
            <Button onClick={() => void loadInstances(true)}>
              <RefreshCwIcon data-icon="inline-start" />
              {t("nginx:instances.retry")}
            </Button>
          </EmptyContent>
        </Empty>
      ) : null}

      {instances.length === 0 && loadStatus !== "loading" && loadStatus !== "error" ? (
        <Empty className="min-h-80 border">
          <EmptyHeader>
            <EmptyMedia variant="icon"><ServerIcon /></EmptyMedia>
            <EmptyTitle>{t("nginx:instances.emptyTitle")}</EmptyTitle>
            <EmptyDescription>{t("nginx:instances.emptyDescription")}</EmptyDescription>
          </EmptyHeader>
          <EmptyContent>
            <Button onClick={() => void handleSelect()}>
              <PlusIcon data-icon="inline-start" />
              {t("nginx:instances.add")}
            </Button>
          </EmptyContent>
        </Empty>
      ) : null}

      {instances.length > 0 ? (
        <NginxInstanceTable
          instances={instances}
          onRefresh={(id) => void refreshInstance(id)}
          onUnregister={(id) => void unregisterInstance(id)}
        />
      ) : null}

      {inspection ? (
        <NginxRegistrationDialog
          inspection={inspection}
          key={inspection.inspectionId}
          loading={operationStatus === "loading"}
          onCancel={clearInspection}
          onRegister={(name, authorization) => {
            void registerInspection(name, authorization).then((instance) => {
              if (instance) {
                toast.add({
                  title: t("nginx:toast.registered", { name: instance.name }),
                  description: t("nginx:toast.registeredDescription"),
                  type: "success",
                });
              } else {
                showCurrentError();
              }
            });
          }}
        />
      ) : null}
    </div>
  );
}
