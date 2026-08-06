import { useTranslation } from "react-i18next";
import { ScrollTextIcon } from "lucide-react";

import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { useMainLayoutHeader } from "src/layouts/MainLayout";
import { NginxInstanceGate } from "src/components/nginx/instance/NginxInstanceGate";

export function NginxLogsPage() {
  const { t } = useTranslation("nginx");
  useMainLayoutHeader({ title: t("logs.title") });

  return (
    <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4">
      <div className="flex flex-col gap-1">
        <h1 className="text-2xl font-semibold tracking-tight">{t("logs.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("logs.description")}</p>
      </div>
      <NginxInstanceGate>
        <Empty className="min-h-80 border">
          <EmptyHeader>
            <EmptyMedia variant="icon"><ScrollTextIcon /></EmptyMedia>
            <EmptyTitle>{t("logs.emptyTitle")}</EmptyTitle>
            <EmptyDescription>{t("logs.emptyDescription")}</EmptyDescription>
          </EmptyHeader>
        </Empty>
      </NginxInstanceGate>
    </div>
  );
}
