import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import {
  ChevronRightIcon,
  RefreshCwIcon,
  ServerCrashIcon,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { SidebarTrigger } from "@/components/ui/sidebar";
import { Spinner } from "@/components/ui/spinner";
import { toast } from "@/components/ui/toast";
import { SystemInformationContent } from "src/components/system-information/SystemInformationContent";
import { SystemInformationSkeleton } from "src/components/system-information/SystemInformationSkeleton";
import { useSystemInformationStore } from "src/stores/system-information-store";
import {
  formatCollectedAt,
  getErrorTranslationKey,
} from "src/utils/system-information";

export function SystemInformationPage() {
  const { i18n, t } = useTranslation(["systemInformation", "common"]);
  const information = useSystemInformationStore((state) => state.information);
  const status = useSystemInformationStore((state) => state.informationStatus);
  const error = useSystemInformationStore((state) => state.informationError);
  const loadInformation = useSystemInformationStore((state) => state.loadInformation);
  const refreshInformation = useSystemInformationStore((state) => state.refreshInformation);
  const loading = status === "loading";
  const locale = i18n.resolvedLanguage ?? i18n.language;

  useEffect(() => {
    void loadInformation();
  }, [loadInformation]);

  async function handleRefresh() {
    const result = await refreshInformation();
    if (result) {
      toast.add({
        title: t("toast.refreshSuccessTitle"),
        description: t("toast.refreshSuccessDescription"),
        type: "success",
      });
      return;
    }

    const currentError = useSystemInformationStore.getState().informationError;
    toast.add({
      title: t("toast.refreshErrorTitle"),
      description: t(
        getErrorTranslationKey(currentError),
        currentError?.message || t("errors.unknown"),
      ),
      type: "error",
    });
  }

  return (
    <div className="min-w-0 mt-14.25">
      <header className="fixed top-0 z-20 w-[stretch] border-b bg-background/95 backdrop-blur supports-backdrop-filter:bg-background/80">
        <div className="mx-auto flex w-full max-w-[1800px] flex-wrap items-center justify-between gap-4 px-4 py-3 lg:px-6">
          <div className="flex min-w-0 items-center gap-3">
            <SidebarTrigger />
            <span aria-hidden="true" className="h-4 w-px bg-border" />
            <span className="truncate text-sm text-muted-foreground">
              {t("common:app.name")}
            </span>
            <ChevronRightIcon aria-hidden="true" className="shrink-0 text-muted-foreground" />
            <span className="truncate text-sm font-medium">{t("title")}</span>
          </div>
          <Button
            disabled={loading}
            onClick={() => void handleRefresh()}
            variant="outline"
          >
            {loading ? (
              <Spinner
                aria-hidden="true"
                data-icon="inline-start"
                role="presentation"
              />
            ) : (
              <RefreshCwIcon data-icon="inline-start" />
            )}
            {loading ? t("refreshing") : t("common:actions.refresh")}
          </Button>
        </div>
      </header>

      <main className="mx-auto flex w-full max-w-[1800px] flex-col gap-5 p-4 lg:p-6">
        <div className="flex flex-col gap-1">
          <h1 className="text-2xl font-semibold tracking-tight">{t("title")}</h1>
          <p className="text-sm text-muted-foreground">
            {information
              ? t("collectedAt", {
                value: formatCollectedAt(
                  information.collectedAt,
                  locale,
                  t("unavailable"),
                ),
              })
              : t("description")}
          </p>
        </div>

        {!information && loading ? <SystemInformationSkeleton /> : null}

        {!information && status === "error" ? (
          <Empty className="min-h-96 border">
            <EmptyHeader>
              <EmptyMedia variant="icon"><ServerCrashIcon /></EmptyMedia>
              <EmptyTitle>{t("errorState.title")}</EmptyTitle>
              <EmptyDescription>
                {t(
                  getErrorTranslationKey(error),
                  error?.message || t("errors.unknown"),
                )}
              </EmptyDescription>
            </EmptyHeader>
            <EmptyContent>
              <Button onClick={() => void handleRefresh()}>
                <RefreshCwIcon data-icon="inline-start" />
                {t("errorState.retry")}
              </Button>
            </EmptyContent>
          </Empty>
        ) : null}

        {information ? (
          <SystemInformationContent information={information} />
        ) : null}
      </main>
    </div>
  );
}
