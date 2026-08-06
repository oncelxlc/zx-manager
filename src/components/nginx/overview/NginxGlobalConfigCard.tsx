import { Settings2Icon } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useNginxGlobalConfigStore } from "src/stores/nginx-global-config-store";
import type { NginxInstance } from "src/types/nginx";
import { NginxGlobalConfigSheet } from "./NginxGlobalConfigSheet";

export function NginxGlobalConfigCard({ instance }: { instance: NginxInstance }) {
  const { t } = useTranslation("nginx");
  const [open, setOpen] = useState(false);
  const source = useNginxGlobalConfigStore((state) => state.source);
  const load = useNginxGlobalConfigStore((state) => state.load);
  useEffect(() => { void load(instance.id); }, [instance.id, load]);
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("globalConfig.title")}</CardTitle>
        <CardDescription>{t("globalConfig.description")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div className="grid gap-2 text-sm sm:grid-cols-2">
          <span>{t("globalConfig.fields.workerProcesses.label")}: {source?.workerProcesses ?? "—"}</span>
          <span>{t("globalConfig.fields.workerConnections.label")}: {source?.workerConnections ?? "—"}</span>
          <span>{t("globalConfig.fields.multiAccept.label")}: {source?.multiAccept ?? "—"}</span>
          <span>{t("globalConfig.fields.errorLog.label")}: {source?.errorLog ?? "—"}</span>
        </div>
        <Button
          disabled={!instance.capabilities.canEdit || !source}
          onClick={() => setOpen(true)}
          variant="outline"
        >
          <Settings2Icon data-icon="inline-start" />{t("globalConfig.edit")}
        </Button>
      </CardContent>
      <NginxGlobalConfigSheet open={open} onOpenChange={setOpen} />
    </Card>
  );
}
