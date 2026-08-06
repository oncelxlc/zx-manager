import { useEffect } from "react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
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
  return (
    <div className="flex min-w-0 flex-col gap-4">
      <div className="flex flex-wrap items-center gap-2">
        <Select
          items={items}
          onValueChange={(value) => { if (value) void selectSource(value); }}
          value={selectedSourceId}
        >
          <SelectTrigger className="min-w-56" aria-label={t("logs.source")}><SelectValue /></SelectTrigger>
          <SelectContent><SelectGroup>{sources.map((source) => (
            <SelectItem disabled={source.availability !== "available"} key={source.id} value={source.id}>
              {source.label}
            </SelectItem>
          ))}</SelectGroup></SelectContent>
        </Select>
        <Badge variant={following ? "default" : "secondary"}>
          {t(following ? "logs.following" : "logs.paused")}
        </Badge>
        <Button disabled={!nextCursor} onClick={() => void loadMore()} variant="outline">
          {t("logs.loadMore")}
        </Button>
        <Button disabled={!selectedSource?.managed} onClick={() => void rotate()} variant="outline">
          {t("logs.rotateNow")}
        </Button>
        <span className="text-xs text-muted-foreground">{t("logs.manualOnly")}</span>
      </div>
      <NginxLogViewer lines={lines} />
    </div>
  );
}

export function NginxLogsPage() {
  const { t } = useTranslation("nginx");
  useMainLayoutHeader({ title: t("logs.title") });
  return (
    <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4">
      <div className="flex flex-col gap-1">
        <h1 className="text-2xl font-semibold tracking-tight">{t("logs.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("logs.description")}</p>
      </div>
      <NginxInstanceGate><NginxLogsContent /></NginxInstanceGate>
    </div>
  );
}
