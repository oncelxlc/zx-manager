import { BoxesIcon, CpuIcon, MemoryStickIcon, ServerIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { MetricCard } from "./MetricCard";

export function DashboardMetricGrid() {
  const { t } = useTranslation(["dashboard", "common"]);

  return (
    <section
      aria-label={t("systemStatus")}
      className="dashboard-kpi-grid grid gap-4"
    >
      <MetricCard
        badge={t("common:status.healthy")}
        badgeVariant="success"
        description={t("cards.nginx.description")}
        detail={t("cards.nginx.detail")}
        healthy
        icon={ServerIcon}
        title={t("cards.nginx.title")}
        value={t("cards.nginx.value")}
      />
      <MetricCard
        badge="-3.2%"
        badgeVariant="success"
        description={t("cards.cpu.description")}
        detail={t("cards.cpu.detail")}
        icon={CpuIcon}
        title={t("cards.cpu.title")}
        value="24.8%"
      />
      <MetricCard
        badge="40%"
        badgeVariant="secondary"
        description={t("cards.memory.description")}
        detail={t("cards.memory.detail")}
        icon={MemoryStickIcon}
        progress={40}
        title={t("cards.memory.title")}
        value="6.4 GB"
      />
      <MetricCard
        badge="+2"
        badgeVariant="secondary"
        description={t("cards.services.description")}
        detail={t("cards.services.detail")}
        icon={BoxesIcon}
        title={t("cards.services.title")}
        value="12"
      />
    </section>
  );
}
