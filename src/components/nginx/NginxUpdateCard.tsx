import { useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { RefreshCwIcon } from "lucide-react";

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

export function NginxUpdateCard() {
  const { i18n, t } = useTranslation("nginx");
  const status = useNginxReleaseStore((state) => state.status);
  const loading = useNginxReleaseStore((state) => state.loading);
  const checking = useNginxReleaseStore((state) => state.checking);
  const error = useNginxReleaseStore((state) => state.error);
  const preferences = useNginxReleaseStore((state) => state.preferences);
  const loadCached = useNginxReleaseStore((state) => state.loadCached);
  const check = useNginxReleaseStore((state) => state.check);

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

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          {t("updates.title")}
          {status && status.updateAvailableCount > 0 ? (
            <Badge variant="warning">
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
          <span className="text-muted-foreground">{t("updates.checkedAt")}</span>
          <span>{loading ? t("updates.loadingCache") : checkedAt}</span>
        </div>
        {error && !status ? (
          <p className="text-destructive">{t(`errors.${error.code}`)}</p>
        ) : null}
      </CardContent>
    </Card>
  );
}
