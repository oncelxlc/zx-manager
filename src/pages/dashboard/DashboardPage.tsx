import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  BoxesIcon,
  CpuIcon,
  HardDriveIcon,
  MemoryStickIcon,
  MoreHorizontalIcon,
  RefreshCwIcon,
  ServerIcon,
  Settings2Icon,
} from "lucide-react";

import { MetricCard } from "src/components/dashboard/MetricCard";
import { ResourceChart } from "src/components/dashboard/ResourceChart";
import { ServiceWorkspace } from "src/components/dashboard/ServiceWorkspace";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { SidebarTrigger } from "@/components/ui/sidebar";
import { Spinner } from "@/components/ui/spinner";
import { toast } from "@/components/ui/toast";
import { initialServices } from "src/data/dashboard-mock-data";
import {
  restartService,
  startService,
  stopService,
} from "src/services/tauri/service-manager";
import type {
  AddServiceInput,
  LocalService,
  ServiceOperation,
} from "src/types/service";

const operationHandlers = {
  start: startService,
  stop: stopService,
  restart: restartService,
} satisfies Record<ServiceOperation, (serviceId: string) => Promise<void>>;

function createServiceId(name: string, sequence: number) {
  const slug = name
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
  return `${slug || "service"}-${sequence}`;
}

export function DashboardPage() {
  const { t } = useTranslation(["dashboard", "services", "common"]);
  const [services, setServices] = useState<LocalService[]>(initialServices);
  const [pendingIds, setPendingIds] = useState<Set<string>>(new Set());
  const [refreshing, setRefreshing] = useState(false);
  const [lastRefreshed, setLastRefreshed] = useState("updatedJustNow");
  const nextServiceSequence = useRef(initialServices.length + 1);

  async function handleRefresh() {
    setRefreshing(true);
    await new Promise((resolve) => setTimeout(resolve, 650));
    setRefreshing(false);
    setLastRefreshed("updatedSecondsAgo");
    toast.add({
      title: t("refreshSuccessTitle"),
      description: t("refreshSuccessDescription"),
      type: "success",
    });
  }

  async function handleOperation(
    service: LocalService,
    operation: ServiceOperation,
  ) {
    setPendingIds((current) => new Set(current).add(service.id));

    try {
      await operationHandlers[operation](service.id);
      setServices((current) =>
        current.map((item) => {
          if (item.id !== service.id) {
            return item;
          }

          if (operation === "stop") {
            return {
              ...item,
              status: "stopped",
              cpu: 0,
              memory: "0 MB",
            };
          }

          return {
            ...item,
            status: "running",
          };
        }),
      );
      toast.add({
        title: t("services:toast.operationSuccess", { name: service.name, operation: t(`services:toast.operations.${operation}`) }),
        description: t("services:toast.operationSuccessDescription"),
        type: "success",
      });
    } catch (error) {
      toast.add({
        title: t("services:toast.operationError", { name: service.name, operation: t(`services:actions.${operation}`) }),
        description:
          error instanceof Error ? error.message : t("common:status.error"),
        type: "error",
      });
    } finally {
      setPendingIds((current) => {
        const nextPending = new Set(current);
        nextPending.delete(service.id);
        return nextPending;
      });
    }
  }

  function handleAddService(input: AddServiceInput) {
    const sequence = nextServiceSequence.current;
    nextServiceSequence.current += 1;

    const newService: LocalService = {
      id: createServiceId(input.name, sequence),
      name: input.name,
      descriptionKey: "custom",
      descriptionValues: { path: input.executablePath, startupMode: t(`services:startupModes.${input.startupMode}`) },
      type: input.type,
      status: "stopped",
      version: "—",
      port: input.port,
      cpu: 0,
      memory: "0 MB",
    };

    setServices((current) => [...current, newService]);
    toast.add({
      title: t("services:toast.added", { name: input.name }),
      description: t("services:toast.addedDescription"),
      type: "success",
    });
  }

  function handleRemove(service: LocalService) {
    setServices((current) =>
      current.filter((item) => item.id !== service.id),
    );
    toast.add({
      title: t("services:toast.removed", { name: service.name }),
      description: t("services:toast.removedDescription"),
      type: "success",
    });
  }

  function handleLocalAction(title: string, service: LocalService) {
    toast.add({
      title,
      description: t("services:toast.localAction", { name: service.name }),
      type: "info",
    });
  }

  function showHeaderAction(title: string) {
    toast.add({
      title,
      description: t("headerActions.mockDescription"),
      type: "info",
    });
  }

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto flex w-full max-w-[1800px] flex-col gap-5 p-4 lg:p-6">
        <header className="flex flex-wrap items-center justify-between gap-4">
          <div className="flex min-w-0 items-center gap-3">
            <SidebarTrigger />
            <div className="min-w-0">
              <h1 className="truncate text-xl font-semibold tracking-tight">
                {t("title")}
              </h1>
              <p className="truncate text-sm text-muted-foreground">
                {t("subtitle", { updated: t(lastRefreshed) })}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Badge variant="success">
              <span className="size-1.5 rounded-full bg-success" />
              {t("systemHealthy")}
            </Badge>
            <Button
              disabled={refreshing}
              onClick={() => void handleRefresh()}
              variant="outline"
            >
              {refreshing ? (
                <Spinner data-icon="inline-start" />
              ) : (
                <RefreshCwIcon data-icon="inline-start" />
              )}
              {refreshing ? t("refreshing") : t("common:actions.refresh")}
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger
                render={
                  <Button
                    aria-label={t("headerActions.openActions")}
                    size="icon"
                    variant="outline"
                  />
                }
              >
                <MoreHorizontalIcon />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuGroup>
                  <DropdownMenuItem
                    onClick={() => showHeaderAction(t("headerActions.openSystemReport"))}
                  >
                    <HardDriveIcon />
                    {t("headerActions.openSystemReport")}
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    onClick={() => showHeaderAction(t("headerActions.dashboardSettings"))}
                  >
                    <Settings2Icon />
                    {t("headerActions.dashboardSettings")}
                  </DropdownMenuItem>
                </DropdownMenuGroup>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  onClick={() => showHeaderAction(t("headerActions.exportDiagnostics"))}
                >
                  {t("headerActions.exportDiagnostics")}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </header>

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

        <ResourceChart />

        <ServiceWorkspace
          onAddService={handleAddService}
          onLocalAction={handleLocalAction}
          onOperation={handleOperation}
          onRemove={handleRemove}
          pendingIds={pendingIds}
          services={services}
        />
      </div>
    </div>
  );
}
