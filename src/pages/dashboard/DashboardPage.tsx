import { lazy, Suspense, useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { DashboardHeaderActions } from "src/components/dashboard/DashboardHeaderActions";
import { DashboardMetricGrid } from "src/components/dashboard/DashboardMetricGrid";
import { ResourceChartSkeleton } from "src/components/dashboard/ResourceChartSkeleton";
import { ServiceWorkspace } from "src/components/dashboard/ServiceWorkspace";
import { useMainLayoutHeader } from "src/layouts/MainLayout";
import { useDashboardServices } from "./useDashboardServices";

const ResourceChart = lazy(() =>
  import("src/components/dashboard/ResourceChart").then((module) => ({
    default: module.ResourceChart,
  })),
);

export function DashboardPage() {
  const { t } = useTranslation("dashboard");
  const [headerActions, setHeaderActions] = useState<ReactNode>();
  const services = useDashboardServices();
  useMainLayoutHeader({
    actions: headerActions,
    title: t("title"),
  });

  return (
    <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4">
      <DashboardHeaderActions onActionsChange={setHeaderActions} />
      <DashboardMetricGrid />

      <Suspense fallback={<ResourceChartSkeleton />}>
        <ResourceChart />
      </Suspense>

      <ServiceWorkspace
        onAddService={services.handleAddService}
        onLocalAction={services.handleLocalAction}
        onOperation={services.handleOperation}
        onRemove={services.handleRemove}
        pendingIds={services.pendingIds}
        services={services.services}
      />
    </div>
  );
}
