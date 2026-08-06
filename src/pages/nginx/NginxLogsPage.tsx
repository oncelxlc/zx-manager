import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { CircleDotIcon, CirclePauseIcon, TriangleAlertIcon } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@/components/ui/empty";
import { Spinner } from "@/components/ui/spinner";
import { toast } from "@/components/ui/toast";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { NginxInstanceGate } from "src/components/nginx/instance/NginxInstanceGate";
import { NginxLogViewer } from "src/components/nginx/logs/NginxLogViewer";
import { NginxPageHeading } from "src/components/nginx/NginxPageHeading";
import { useMainLayoutHeader } from "src/layouts/MainLayout";
import { useNginxLogStore } from "src/stores/nginx-log-store";
import { useNginxStore } from "src/stores/nginx-store";

function NginxLogsContent() {
  const { t } = useTranslation("nginx");
  const instance = useNginxStore((state) => state.instance);
  const sources = useNginxLogStore((state) => state.sources);
  const selectedSourceId = useNginxLogStore((state) => state.selectedSourceId);
  const lines = useNginxLogStore((state) => state.lines);
  const nextCursor = useNginxLogStore((state) => state.nextCursor);
  const following = useNginxLogStore((state) => state.following);
  const status = useNginxLogStore((state) => state.status);
  const error = useNginxLogStore((state) => state.error);
  const loadSources = useNginxLogStore((state) => state.loadSources);
  const selectSource = useNginxLogStore((state) => state.selectSource);
  const loadMore = useNginxLogStore((state) => state.loadMore);
  const rotate = useNginxLogStore((state) => state.rotate);
  const clear = useNginxLogStore((state) => state.clear);
  useEffect(() => {
    if (instance) void loadSources(instance.id);
    return () => { void clear(); };
  }, [clear, instance, loadSources]);
  if (!instance) return null;
  const items = sources.map((source) => ({ value: source.id, label: source.label }));
  const selectedSource = sources.find((source) => source.id === selectedSourceId);
  if (status === "loading" && sources.length === 0) {
    return (
      <div className="flex min-h-48 items-center justify-center">
        <Spinner />
        <span className="sr-only">{t("logs.loading")}</span>
      </div>
    );
  }
  if (status === "error" && sources.length === 0) {
    return (
      <Empty className="border">
        <EmptyHeader>
          <EmptyTitle>{t("logs.errorTitle")}</EmptyTitle>
          <EmptyDescription>{t(`errors.${error?.code ?? "NGINX_UNKNOWN"}`)}</EmptyDescription>
        </EmptyHeader>
      </Empty>
    );
  }
  if (!selectedSource) {
    return (
      <Empty className="border">
        <EmptyHeader>
          <EmptyTitle>{t("logs.emptyTitle")}</EmptyTitle>
          <EmptyDescription>{t("logs.emptyDescription")}</EmptyDescription>
        </EmptyHeader>
      </Empty>
    );
  }
  return (
    <div className="flex min-w-0 flex-col gap-4">
      <div className="flex flex-wrap items-center gap-2">
        <Select
          items={items}
          onValueChange={(value) => { if (value) void selectSource(value); }}
          value={selectedSourceId}
        >
          <SelectTrigger className="w-full sm:w-72" aria-label={t("logs.source")}><SelectValue /></SelectTrigger>
          <SelectContent><SelectGroup>{sources.map((source) => (
            <SelectItem disabled={source.availability !== "available"} key={source.id} value={source.id}>
              {source.label}
            </SelectItem>
          ))}</SelectGroup></SelectContent>
        </Select>
        <Badge aria-label={t(following ? "logs.following" : "logs.paused")} variant={following ? "default" : "secondary"}>
          {following ? <CircleDotIcon /> : <CirclePauseIcon />}
          {t(following ? "logs.following" : "logs.paused")}
        </Badge>
        <Button disabled={!nextCursor} onClick={() => void loadMore()} size="sm" variant="outline">
          {t("logs.loadMore")}
        </Button>
        <Button
          disabled={!selectedSource.managed}
          onClick={() => void rotate().then((rotated) => toast.add({
            title: t(rotated ? "logs.rotateSucceeded" : "logs.rotateFailed"),
            type: rotated ? "success" : "error",
          }))}
          size="sm"
          variant="outline"
        >
          {t("logs.rotateNow")}
        </Button>
        <Badge variant="outline">{t(selectedSource.managed ? "logs.managed" : "logs.readOnly")}</Badge>
      </div>
      <Alert>
        <TriangleAlertIcon />
        <AlertTitle>{t("logs.rotationPolicyTitle")}</AlertTitle>
        <AlertDescription>{t("logs.manualOnly")}</AlertDescription>
      </Alert>
      <NginxLogViewer ariaLabel={t("logs.viewerLabel")} lines={lines} />
    </div>
  );
}

export function NginxLogsPage() {
  const { t } = useTranslation("nginx");
  useMainLayoutHeader({ title: t("logs.title") });
  return (
    <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4">
      <NginxPageHeading description={t("logs.description")} title={t("logs.title")} />
      <NginxInstanceGate><NginxLogsContent /></NginxInstanceGate>
    </div>
  );
}
