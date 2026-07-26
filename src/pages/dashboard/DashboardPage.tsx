import { useRef, useState } from "react";
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

import { MetricCard } from "@/components/dashboard/MetricCard";
import { ResourceChart } from "@/components/dashboard/ResourceChart";
import { ServiceWorkspace } from "@/components/dashboard/ServiceWorkspace";
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
import { initialServices } from "@/data/dashboard-mock-data";
import {
  restartService,
  startService,
  stopService,
} from "@/services/tauri/service-manager";
import type {
  AddServiceInput,
  LocalService,
  ServiceOperation,
} from "@/types/service";

const operationHandlers = {
  start: startService,
  stop: stopService,
  restart: restartService,
} satisfies Record<ServiceOperation, (serviceId: string) => Promise<void>>;

const operationLabels: Record<ServiceOperation, string> = {
  start: "started",
  stop: "stopped",
  restart: "restarted",
};

function createServiceId(name: string, sequence: number) {
  const slug = name
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
  return `${slug || "service"}-${sequence}`;
}

export function DashboardPage() {
  const [services, setServices] = useState<LocalService[]>(initialServices);
  const [pendingIds, setPendingIds] = useState<Set<string>>(new Set());
  const [refreshing, setRefreshing] = useState(false);
  const [lastRefreshed, setLastRefreshed] = useState("Updated just now");
  const nextServiceSequence = useRef(initialServices.length + 1);

  async function handleRefresh() {
    setRefreshing(true);
    await new Promise((resolve) => setTimeout(resolve, 650));
    setRefreshing(false);
    setLastRefreshed("Updated a few seconds ago");
    toast.add({
      title: "Dashboard refreshed",
      description: "Local service and resource snapshots are up to date.",
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
        title: `${service.name} ${operationLabels[operation]}`,
        description: "The dashboard mock state was updated successfully.",
        type: "success",
      });
    } catch (error) {
      toast.add({
        title: `Unable to ${operation} ${service.name}`,
        description:
          error instanceof Error ? error.message : "An unknown error occurred.",
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
      description: `${input.startupMode} service · ${input.executablePath}`,
      type: input.type,
      status: "stopped",
      version: "—",
      port: input.port,
      cpu: 0,
      memory: "0 MB",
    };

    setServices((current) => [...current, newService]);
    toast.add({
      title: `${input.name} added`,
      description: "The service is registered locally in a stopped state.",
      type: "success",
    });
  }

  function handleRemove(service: LocalService) {
    setServices((current) =>
      current.filter((item) => item.id !== service.id),
    );
    toast.add({
      title: `${service.name} removed`,
      description: "Only the local dashboard entry was removed.",
      type: "success",
    });
  }

  function handleLocalAction(title: string, service: LocalService) {
    toast.add({
      title,
      description: `${service.name} is ready for a future Tauri integration.`,
      type: "info",
    });
  }

  function showHeaderAction(title: string) {
    toast.add({
      title,
      description: "This dashboard action is currently running in mock mode.",
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
                Dashboard
              </h1>
              <p className="truncate text-sm text-muted-foreground">
                Monitor and manage local infrastructure services ·{" "}
                {lastRefreshed}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Badge variant="success">
              <span className="size-1.5 rounded-full bg-success" />
              System Healthy
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
              {refreshing ? "Refreshing" : "Refresh"}
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger
                render={
                  <Button
                    aria-label="Open dashboard actions"
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
                    onClick={() => showHeaderAction("Open system report")}
                  >
                    <HardDriveIcon />
                    Open system report
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    onClick={() => showHeaderAction("Dashboard settings")}
                  >
                    <Settings2Icon />
                    Dashboard settings
                  </DropdownMenuItem>
                </DropdownMenuGroup>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  onClick={() => showHeaderAction("Export diagnostics")}
                >
                  Export diagnostics
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </header>

        <section
          aria-label="System status"
          className="dashboard-kpi-grid grid gap-4"
        >
          <MetricCard
            badge="Healthy"
            badgeVariant="success"
            description="Nginx 1.26.2 is running"
            detail="Listening on ports 80 and 443"
            healthy
            icon={ServerIcon}
            title="Nginx Status"
            value="Running"
          />
          <MetricCard
            badge="-3.2%"
            badgeVariant="success"
            description="Normal system load"
            detail="8 cores · 2.4 GHz average"
            icon={CpuIcon}
            title="CPU Usage"
            value="24.8%"
          />
          <MetricCard
            badge="40%"
            badgeVariant="secondary"
            description="6.4 GB of 16 GB used"
            detail="9.6 GB available"
            icon={MemoryStickIcon}
            progress={40}
            title="Memory Usage"
            value="6.4 GB"
          />
          <MetricCard
            badge="+2"
            badgeVariant="secondary"
            description="12 of 15 services running"
            detail="3 services stopped or need attention"
            icon={BoxesIcon}
            title="Active Services"
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
