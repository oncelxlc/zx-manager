import { useMemo } from "react";
import {
  NetworkIcon,
  RefreshCwIcon,
  ShieldAlertIcon,
  Trash2Icon,
  TriangleAlertIcon,
} from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogMedia,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { NetworkApplicationsCard } from "src/components/network-monitor/NetworkApplicationsCard";
import { NetworkHistoryCard } from "src/components/network-monitor/NetworkHistoryCard";
import { NetworkMonitorOnboarding } from "src/components/network-monitor/NetworkMonitorOnboarding";
import { Spinner } from "@/components/ui/spinner";
import { useMainLayoutHeader } from "src/layouts/MainLayout";
import { useNetworkMonitorController } from "./useNetworkMonitorController";

function NetworkMonitorNotices({controller}: {controller: ReturnType<typeof useNetworkMonitorController>}) {
  const {error, latest, status, t} = controller;

  return <>
    {error || status?.lastError ? <Alert variant="destructive">
      <ShieldAlertIcon />
      <AlertTitle>{t("errors.title")}</AlertTitle>
      <AlertDescription>{t(`errors.codes.${error?.code ?? status?.lastError?.code ?? "unknown"}`, {
        defaultValue: error?.message ?? status?.lastError?.message ?? "",
      })}</AlertDescription>
    </Alert> : null}
    {status?.partialData || latest?.sampleState === "gap" ? <Alert>
      <TriangleAlertIcon />
      <AlertTitle>{t("quality.partialTitle")}</AlertTitle>
      <AlertDescription>{t("quality.partialDescription", {
        lost: status?.lostEvents ?? latest?.lostEvents ?? 0,
        unresolved: status?.unresolvedEvents ?? latest?.unresolvedEvents ?? 0,
      })}</AlertDescription>
    </Alert> : null}
    <Alert>
      <NetworkIcon />
      <AlertTitle>{t("quality.aggregateTitle")}</AlertTitle>
      <AlertDescription>{t("quality.aggregateDescription")}</AlertDescription>
    </Alert>
  </>;
}

function ClearUsageDialog({controller}: {controller: ReturnType<typeof useNetworkMonitorController>}) {
  const {clearAllUsage, clearOpen, setClearOpen, t} = controller;

  return <AlertDialog onOpenChange={setClearOpen} open={clearOpen}>
    <AlertDialogContent>
      <AlertDialogHeader>
        <AlertDialogMedia><Trash2Icon /></AlertDialogMedia>
        <AlertDialogTitle>{t("clear.title")}</AlertDialogTitle>
        <AlertDialogDescription>{t("clear.description")}</AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel>{t("common:actions.cancel")}</AlertDialogCancel>
        <AlertDialogAction onClick={clearAllUsage} variant="destructive">
          {t("clear.confirm")}
        </AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>;
}

export function NetworkMonitorPage() {
  const controller = useNetworkMonitorController();
  const {
    handleCurrentSession,
    loading,
    locale,
    platformSupported,
    refresh,
    setClearOpen,
    stale,
    status,
    t,
  } = controller;
  const headerActions = useMemo(() => platformSupported === false ? null : <>
    <Badge variant={status?.enabled ? "default" : "secondary"}>
      {t(status?.enabled ? "status.running" : "status.disabled")}
    </Badge>
    <Button disabled={loading} onClick={() => void handleCurrentSession()} variant="outline">
      {loading ? <Spinner data-icon="inline-start" /> : <NetworkIcon data-icon="inline-start" />}
      {t(status?.enabled ? "actions.stopSession" : "actions.startSession")}
    </Button>
    <Button disabled={loading} onClick={() => void refresh()} variant="outline">
      <RefreshCwIcon data-icon="inline-start" />
      {t("common:actions.refresh")}
    </Button>
    <Button onClick={() => setClearOpen(true)} variant="destructive">
      <Trash2Icon data-icon="inline-start" />
      {t("actions.clear")}
    </Button>
  </>, [handleCurrentSession, loading, platformSupported, refresh, setClearOpen, status?.enabled, t]);
  useMainLayoutHeader({actions: headerActions, title: t("title")});

  if (platformSupported === false) return <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4">
    <div className="flex flex-col gap-1"><h1 className="text-2xl font-semibold tracking-tight">{t("title")}</h1><p className="text-sm text-muted-foreground">{t("description")}</p></div>
    <Alert><ShieldAlertIcon /><AlertTitle>{t("unsupported.title")}</AlertTitle><AlertDescription>{t("unsupported.description")}</AlertDescription></Alert>
  </div>;

  return <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4">
    <div className="flex flex-col gap-1">
      <h1 className="text-2xl font-semibold tracking-tight">{t("title")}</h1>
      <p className="text-sm text-muted-foreground">
        {status?.lastSampledAt ? t("lastUpdated", {value: new Intl.DateTimeFormat(locale, {dateStyle: "short", timeStyle: "medium"}).format(status.lastSampledAt)}) : t("description")}
        {stale ? ` · ${t("status.stale")}` : ""}
      </p>
    </div>
    {controller.configured === false && !status?.enabled ? <NetworkMonitorOnboarding
      description={t("onboarding.description")}
      notNowLabel={t("onboarding.notNow")}
      onChoice={(start) => void controller.handleFirstChoice(start)}
      startLabel={t("onboarding.start")}
      storageDescription={t("onboarding.storageDescription")}
      storageTitle={t("onboarding.storageTitle")}
      title={t("onboarding.title")}
    /> : null}
    <NetworkMonitorNotices controller={controller} />
    <NetworkApplicationsCard
      applicationDisplayName={controller.applicationDisplayName} applications={controller.applications}
      locale={locale} onPageChange={controller.setApplicationPageIndex}
      onPageSizeChange={(size) => {controller.setApplicationPageSize(size); controller.setApplicationPageIndex(0);}}
      onPathChange={(path) => {controller.setApplicationPath(path); controller.setApplicationPageIndex(0);}}
      onSampleIntervalChange={(value) => void controller.handleSampleIntervalChange(value)} onSort={controller.setApplicationSort}
      pageIndex={controller.safeApplicationPageIndex} pageSize={controller.applicationPageSize} path={controller.applicationPath}
      sampleIntervalItems={controller.sampleIntervalItems} sampleIntervalLoading={controller.sampleIntervalLoading}
      sortBy={controller.applicationSortBy} sortDirection={controller.applicationSortDirection} status={status}
      visibleApplications={controller.visibleApplications}
    />
    <NetworkHistoryCard
      applicationDisplayName={controller.applicationDisplayName} customFrom={controller.customFrom} customTo={controller.customTo}
      history={controller.history} loading={controller.historyLoading} locale={locale} onCustomFromChange={controller.setCustomFrom}
      onCustomToChange={controller.setCustomTo} onPageChange={controller.setHistoryPageIndex}
      onPageSizeChange={(size) => {controller.setHistoryPageSize(size); controller.setHistoryPageIndex(0);}}
      onPathChange={(path) => {controller.setHistoryPath(path); controller.setHistoryPageIndex(0);}}
      onQuery={() => controller.historyPageIndex === 0 ? controller.runHistoryQuery?.() : controller.setHistoryPageIndex(0)}
      onRangeChange={(range) => {controller.setRange(range); controller.setHistoryPageIndex(0);}}
      onSort={controller.setHistorySort} pageIndex={controller.safeHistoryPageIndex} pageSize={controller.historyPageSize}
      path={controller.historyPath} range={controller.range} sortBy={controller.historySortBy} sortDirection={controller.historySortDirection}
    />
    <ClearUsageDialog controller={controller} />
  </div>;
}
