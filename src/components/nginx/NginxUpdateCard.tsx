import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { RefreshCwIcon, ShieldCheckIcon, TriangleAlertIcon } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Spinner } from "@/components/ui/spinner";
import { toast } from "@/components/ui/toast";
import { useNginxReleaseStore } from "src/stores/nginx-release-store";
import { useNginxStore } from "src/stores/nginx-store";
import { NginxUpgradeDialog } from "src/components/nginx/NginxUpgradeDialog";

export function NginxUpdateCard() {
  const { i18n, t } = useTranslation("nginx");
  const status = useNginxReleaseStore((state) => state.status);
  const loading = useNginxReleaseStore((state) => state.loading);
  const checking = useNginxReleaseStore((state) => state.checking);
  const error = useNginxReleaseStore((state) => state.error);
  const preferences = useNginxReleaseStore((state) => state.preferences);
  const loadCached = useNginxReleaseStore((state) => state.loadCached);
  const check = useNginxReleaseStore((state) => state.check);
  const instance = useNginxStore((state) => state.instance);
  const operationStatus = useNginxStore((state) => state.operationStatus);
  const upgradeProgress = useNginxStore((state) => state.upgradeProgress);
  const lastUpgradeResult = useNginxStore((state) => state.lastUpgradeResult);
  const upgradeInstance = useNginxStore((state) => state.upgradeInstance);
  const [confirmUpgrade, setConfirmUpgrade] = useState(false);

  useEffect(() => {
    void loadCached();
  }, [loadCached, preferences.releaseChannel]);

  const forceRefresh = useCallback(async () => {
    const result = await check(true);
    toast.add({
      title: result ? t("updates.refreshSuccess") : t("updates.refreshError"),
      description: result
        ? t("updates.refreshSuccessDescription", {
          version: result.latestRelease?.version ?? t("updates.unavailable"),
        })
        : t(`errors.${useNginxReleaseStore.getState().error?.code ?? "NGINX_UNKNOWN"}`),
      type: result ? "success" : "error",
    });
  }, [check, t]);

  const checkedAt = status?.checkedAt
    ? new Intl.DateTimeFormat(i18n.resolvedLanguage ?? i18n.language, {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(new Date(status.checkedAt))
    : t("updates.neverChecked");
  const targetVersion = status?.latestRelease?.version ?? null;
  const upgradeSupported = Boolean(
    instance
    && targetVersion
    && status?.outdatedInstanceIds.includes(instance.id)
    && instance.controlBackend === "portable"
    && instance.authorizationLevel === "full"
    && instance.lifecycleState === "available"
    && !["conflict", "unknown"].includes(instance.runtimeStatus)
    && navigator.userAgent.includes("Windows"),
  );
  const supportReason = !instance
    ? t("upgrade.reason.noInstance")
    : !navigator.userAgent.includes("Windows")
      ? t("upgrade.reason.platform")
      : instance.controlBackend !== "portable"
        ? t("upgrade.reason.backend")
        : instance.authorizationLevel !== "full"
          ? t("upgrade.reason.authorization")
          : instance.lifecycleState !== "available"
            ? t("upgrade.reason.lifecycle")
            : !targetVersion || !status?.outdatedInstanceIds.includes(instance.id)
              ? t("upgrade.reason.upToDate")
              : t("upgrade.reason.available");

  const runUpgrade = useCallback(async () => {
    if (!targetVersion) return;
    const result = await upgradeInstance({
      channel: preferences.releaseChannel,
      targetVersion,
      backupRetentionCount: preferences.backupRetentionCount,
    });
    if (result?.success) setConfirmUpgrade(false);
    toast.add({
      title: result?.success ? t("upgrade.success") : t("upgrade.failed"),
      description: result?.rolledBack ? t("upgrade.rolledBack") : supportReason,
      type: result?.success ? "success" : "error",
    });
  }, [preferences, supportReason, t, targetVersion, upgradeInstance]);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          {t("updates.title")}
          {status && status.updateAvailableCount > 0 ? (
            <Badge
              aria-label={t("updates.availableBadge", { count: status.updateAvailableCount })}
              variant="warning"
            >
              <TriangleAlertIcon />
              {t("updates.availableBadge", { count: status.updateAvailableCount })}
            </Badge>
          ) : null}
        </CardTitle>
        <CardDescription>
          {t("updates.description", {
            channel: t(`updates.channels.${preferences.releaseChannel}`),
          })}
        </CardDescription>
        <CardAction>
          <Button disabled={checking} onClick={() => void forceRefresh()} variant="outline">
            {checking ? (
              <Spinner data-icon="inline-start" />
            ) : (
              <RefreshCwIcon data-icon="inline-start" />
            )}
            {t("updates.checkNow")}
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent className="grid gap-2 text-sm">
        <div className="flex flex-wrap justify-between gap-2">
          <span className="text-muted-foreground">{t("updates.latest")}</span>
          <span>{status?.latestRelease?.version ?? t("updates.unavailable")}</span>
        </div>
        <div className="flex flex-wrap justify-between gap-2">
          <span className="text-muted-foreground">{t("upgrade.currentVersion")}</span>
          <span>{instance?.version ?? t("updates.unavailable")}</span>
        </div>
        <div className="flex flex-wrap justify-between gap-2">
          <span className="text-muted-foreground">{t("upgrade.support")}</span>
          <span>{supportReason}</span>
        </div>
        {error && !status ? (
          <p className="text-destructive">{t(`errors.${error.code}`)}</p>
        ) : null}
        <Button
          className="mt-2 justify-self-start"
          disabled={!upgradeSupported || operationStatus === "loading"}
          onClick={() => setConfirmUpgrade(true)}
        >
          <ShieldCheckIcon data-icon="inline-start" />{t("upgrade.action")}
        </Button>
      </CardContent>
      {instance && targetVersion ? (
        <NginxUpgradeDialog
          cacheSource={t(`updates.sources.${status?.source ?? "none"}`)}
          checkedAt={loading ? t("updates.loadingCache") : checkedAt}
          currentVersion={instance.version}
          onConfirm={() => void runUpgrade()}
          onOpenChange={setConfirmUpgrade}
          open={confirmUpgrade}
          progress={upgradeProgress}
          result={lastUpgradeResult}
          running={operationStatus === "loading"}
          supportReason={supportReason}
          targetVersion={targetVersion}
        />
      ) : null}
    </Card>
  );
}
