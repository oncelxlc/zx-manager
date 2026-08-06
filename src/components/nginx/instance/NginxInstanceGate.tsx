import { useCallback, useEffect, type ReactNode } from "react";
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
import { NginxMigrationAlert } from "src/components/nginx/NginxMigrationAlert";
import { NginxRegistrationDialog } from "src/components/nginx/NginxRegistrationDialog";
import { useNginxStore } from "src/stores/nginx-store";

interface NginxInstanceGateProps {
  children: ReactNode;
}

export function NginxInstanceGate({ children }: NginxInstanceGateProps) {
  const { t } = useTranslation(["nginx", "common"]);
  const store = useNginxStore();
  const busy = store.operationStatus === "loading";

  useEffect(() => {
    void store.loadRegistry();
    void store.ensureStatusSubscription();
  }, [store.ensureStatusSubscription, store.loadRegistry]);

  const showError = useCallback(() => {
    const error = useNginxStore.getState().error;
    toast.add({
      title: t("nginx:toast.errorTitle"),
      description: t(`nginx:errors.${error?.code ?? "NGINX_UNKNOWN"}`),
      type: "error",
    });
  }, [t]);

  const selectExisting = useCallback(async () => {
    const result = await store.selectAndInspect();
    if (!result && useNginxStore.getState().operationStatus === "error") {
      showError();
    }
  }, [showError, store]);

  if (!store.registryState && ["idle", "loading"].includes(store.loadStatus)) {
    return (
      <div className="flex min-h-80 items-center justify-center">
        <Spinner />
        <span className="sr-only">{t("common:status.loading")}</span>
      </div>
    );
  }

  if (!store.registryState) {
    return (
      <Empty className="min-h-80 border">
        <EmptyHeader>
          <EmptyMedia variant="icon"><ServerCrashIcon /></EmptyMedia>
          <EmptyTitle>{t("nginx:instanceGate.errorTitle")}</EmptyTitle>
          <EmptyDescription>
            {t(`nginx:errors.${store.error?.code ?? "NGINX_UNKNOWN"}`)}
          </EmptyDescription>
        </EmptyHeader>
        <EmptyContent>
          <Button onClick={() => void store.loadRegistry(true)}>
            <RefreshCwIcon data-icon="inline-start" />
            {t("nginx:instanceGate.retry")}
          </Button>
        </EmptyContent>
      </Empty>
    );
  }

  if (store.registryState.status === "migrationRequired") {
    return (
      <NginxMigrationAlert
        candidates={store.registryState.migrationCandidates}
        loading={busy}
        onResolve={(id) => void store.resolveMigration(id)}
      />
    );
  }

  if (store.registryState.status === "empty") {
    return (
      <>
        <Empty className="min-h-80 border">
          <EmptyHeader>
            <EmptyMedia variant="icon"><ServerIcon /></EmptyMedia>
            <EmptyTitle>{t("nginx:instanceGate.emptyTitle")}</EmptyTitle>
            <EmptyDescription>{t("nginx:instanceGate.emptyDescription")}</EmptyDescription>
          </EmptyHeader>
          <EmptyContent className="flex-row justify-center">
            <Button disabled={busy} onClick={() => void selectExisting()}>
              <PlusIcon data-icon="inline-start" />
              {t("nginx:instanceGate.selectExisting")}
            </Button>
            <Button disabled variant="outline">
              {t("nginx:instanceGate.install")}
            </Button>
          </EmptyContent>
          <p className="text-xs text-muted-foreground">
            {t("nginx:instanceGate.installUnavailable")}
          </p>
        </Empty>
        {store.inspection ? (
          <NginxRegistrationDialog
            inspection={store.inspection}
            key={store.inspection.inspectionId}
            loading={busy}
            onCancel={store.clearInspection}
            onRegister={(authorization) => void store.registerInspection(authorization).then(
              (instance) => {
                if (instance) {
                  toast.add({
                    title: t("nginx:toast.registered", { name: instance.name }),
                    type: "success",
                  });
                } else showError();
              },
            )}
          />
        ) : null}
      </>
    );
  }

  const instance = store.registryState.instance;
  if (instance.lifecycleState !== "available") {
    return (
      <Empty className="min-h-80 border">
        <EmptyHeader>
          <EmptyMedia variant="icon"><ServerCrashIcon /></EmptyMedia>
          <EmptyTitle>{t("nginx:instanceGate.invalidTitle")}</EmptyTitle>
          <EmptyDescription>
            {t("nginx:instanceGate.invalidDescription", {
              state: t(`nginx:lifecycle.${instance.lifecycleState}`),
            })}
          </EmptyDescription>
        </EmptyHeader>
        <EmptyContent className="flex-row justify-center">
          <Button onClick={() => void store.refreshInstance()} variant="outline">
            <RefreshCwIcon data-icon="inline-start" />
            {t("nginx:instanceGate.retry")}
          </Button>
          <Button disabled={busy} onClick={() => void store.unregisterInstance()}>
            {t("nginx:instanceGate.clearInvalid")}
          </Button>
        </EmptyContent>
      </Empty>
    );
  }

  return children;
}
