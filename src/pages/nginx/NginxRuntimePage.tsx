import { useTranslation } from "react-i18next";
import { ActivityIcon } from "lucide-react";

import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { useMainLayoutHeader } from "src/layouts/MainLayout";

export function NginxRuntimePage() {
  const { t } = useTranslation("nginx");
  useMainLayoutHeader({ title: t("runtimePage.title") });

  return (
    <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4">
      <div className="flex flex-col gap-1">
        <h1 className="text-2xl font-semibold tracking-tight">
          {t("runtimePage.title")}
        </h1>
        <p className="text-sm text-muted-foreground">
          {t("runtimePage.description")}
        </p>
      </div>
      <Empty className="min-h-80 border">
        <EmptyHeader>
          <EmptyMedia variant="icon"><ActivityIcon /></EmptyMedia>
          <EmptyTitle>{t("runtimePage.emptyTitle")}</EmptyTitle>
          <EmptyDescription>{t("runtimePage.emptyDescription")}</EmptyDescription>
        </EmptyHeader>
      </Empty>
    </div>
  );
}
