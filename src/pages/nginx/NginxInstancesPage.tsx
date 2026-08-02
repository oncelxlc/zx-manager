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
import { NginxInstanceCard } from "src/components/nginx/NginxInstanceCard";
import { NginxMigrationAlert } from "src/components/nginx/NginxMigrationAlert";
import { NginxRegistrationDialog } from "src/components/nginx/NginxRegistrationDialog";
import { useMainLayoutHeader } from "src/layouts/MainLayout";
import { useNginxStore } from "src/stores/nginx-store";

export function NginxInstancesPage() {
  const { t } = useTranslation(["nginx", "common"]);
  const store = useNginxStore();
  const busy = store.operationStatus === "loading";

  useEffect(() => {
    void store.loadRegistry();
    void store.ensureStatusSubscription();
  }, [store.loadRegistry, store.ensureStatusSubscription]);

  const showCurrentError = useCallback(() => {
    const error = useNginxStore.getState().error;
    toast.add({
      title: t("nginx:toast.errorTitle"),
      description: t(`nginx:errors.${error?.code ?? "NGINX_UNKNOWN"}`),
      type: "error",
    });
  }, [t]);

  const handleSelect = useCallback(async () => {
    const result = await store.selectAndInspect();
    if (!result && useNginxStore.getState().operationStatus === "error") {
      showCurrentError();
    }
  }, [showCurrentError, store]);

  const handleControl = useCallback(async (action: "start" | "stop" | "reload" | "restart") => {
    const succeeded = await store.controlInstance(action);
    const operation = useNginxStore.getState().lastOperation;
    toast.add({
      title: succeeded ? t("nginx:toast.operationSuccess") : t("nginx:toast.errorTitle"),
      description: succeeded
        ? t(operation?.outcome === "noop"
          ? "nginx:toast.operationNoop"
          : "nginx:toast.operationExecuted")
        : t(`nginx:errors.${useNginxStore.getState().error?.code ?? "NGINX_UNKNOWN"}`),
      type: succeeded ? "success" : "error",
    });
  }, [store, t]);

  const headerActions = useMemo(() => (
    <>
      <Button disabled={store.loadStatus === "loading"} onClick={() => void store.loadRegistry(true)} variant="outline">
        {store.loadStatus === "loading" ? <Spinner data-icon="inline-start" /> : <RefreshCwIcon data-icon="inline-start" />}
        {t("common:actions.refresh")}
      </Button>
      {store.registryState?.status === "empty" ? (
        <Button disabled={busy} onClick={() => void handleSelect()}>
          <PlusIcon data-icon="inline-start" />{t("nginx:instances.add")}
        </Button>
      ) : null}
    </>
  ), [busy, handleSelect, store.loadRegistry, store.loadStatus, store.registryState?.status, t]);

  useMainLayoutHeader({ actions: headerActions, title: t("nginx:instances.title") });

  return (
    <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4">
      <div className="flex flex-col gap-1">
        <h1 className="text-2xl font-semibold tracking-tight">{t("nginx:instances.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("nginx:instances.description")}</p>
      </div>

      {store.registryState?.status === "migrationRequired" ? (
        <NginxMigrationAlert
          candidates={store.registryState.migrationCandidates}
          loading={busy}
          onResolve={(id) => void store.resolveMigration(id)}
        />
      ) : null}

      {!store.registryState && store.loadStatus === "error" ? (
        <Empty className="min-h-80 border">
          <EmptyHeader>
            <EmptyMedia variant="icon"><ServerCrashIcon /></EmptyMedia>
            <EmptyTitle>{t("nginx:instances.errorTitle")}</EmptyTitle>
            <EmptyDescription>{t(`nginx:errors.${store.error?.code ?? "NGINX_UNKNOWN"}`)}</EmptyDescription>
          </EmptyHeader>
          <EmptyContent>
            <Button onClick={() => void store.loadRegistry(true)}>{t("nginx:instances.retry")}</Button>
          </EmptyContent>
        </Empty>
      ) : null}

      {store.registryState?.status === "empty" ? (
        <Empty className="min-h-80 border">
          <EmptyHeader>
            <EmptyMedia variant="icon"><ServerIcon /></EmptyMedia>
            <EmptyTitle>{t("nginx:instances.emptyTitle")}</EmptyTitle>
            <EmptyDescription>{t("nginx:instances.emptyDescription")}</EmptyDescription>
          </EmptyHeader>
          <EmptyContent>
            <Button onClick={() => void handleSelect()}><PlusIcon data-icon="inline-start" />{t("nginx:instances.add")}</Button>
          </EmptyContent>
        </Empty>
      ) : null}

      {store.instance ? (
        <NginxInstanceCard
          busy={busy}
          instance={store.instance}
          observedAt={store.observedAt}
          onControl={(action) => void handleControl(action)}
          onRefresh={() => void store.refreshInstance()}
          onUnregister={() => void store.unregisterInstance()}
          operationPhase={store.operationPhase}
        />
      ) : null}

      {store.inspection ? (
        <NginxRegistrationDialog
          inspection={store.inspection}
          key={store.inspection.inspectionId}
          loading={busy}
          onCancel={store.clearInspection}
          onRegister={(authorization) => void store.registerInspection(authorization).then((instance) => {
            if (instance) {
              toast.add({ title: t("nginx:toast.registered", { name: instance.name }), type: "success" });
            } else showCurrentError();
          })}
        />
      ) : null}
    </div>
  );
}
