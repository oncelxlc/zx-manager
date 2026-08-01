import { BoxesIcon, CpuIcon, MemoryStickIcon, ServerIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useEffect } from "react";

import { MetricCard } from "./MetricCard";
import { useNginxStore } from "src/stores/nginx-store";

export function DashboardMetricGrid() {
  const { t } = useTranslation(["dashboard", "common", "nginx"]);
  const instance = useNginxStore((state) => state.instance);
  const loadRegistry = useNginxStore((state) => state.loadRegistry);
  const ensureStatusSubscription = useNginxStore((state) => state.ensureStatusSubscription);

  useEffect(() => {
    void loadRegistry();
    void ensureStatusSubscription();
  }, [ensureStatusSubscription, loadRegistry]);
  const runtimeStatus = instance?.runtimeStatus ?? "stopped";
  const badgeVariant = runtimeStatus === "running"
    ? "success"
    : runtimeStatus === "conflict"
      ? "destructive"
      : runtimeStatus === "unknown" ? "warning" : "secondary";

  return (
    <section
      aria-label={t("systemStatus")}
      className="dashboard-kpi-grid grid gap-4"
    >
      <MetricCard
        badge={instance ? t(`nginx:runtime.${runtimeStatus}`) : t("nginx:management.instanceEmpty")}
        badgeVariant={badgeVariant}
        description={instance ? t("cards.nginx.description") : t("nginx:instances.emptyDescription")}
        detail={instance ? t(`nginx:lifecycle.${instance.lifecycleState}`) : t("cards.nginx.detail")}
        healthy={runtimeStatus === "running"}
        icon={ServerIcon}
        title={t("cards.nginx.title")}
        value={instance ? `nginx/${instance.version}` : "—"}
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
