import type { ComponentProps } from "react";
import { GripVerticalIcon } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { cn } from "@/lib/utils";
import type {
  LocalService,
  ServiceColumn,
  ServiceOperation,
  ServiceStatus,
  ServiceType,
} from "@/types/service";
import { ServiceActions } from "./ServiceActions";

const statusConfig: Record<
  ServiceStatus,
  {
    label: string;
    variant: ComponentProps<typeof Badge>["variant"];
    dotClassName: string;
  }
> = {
  running: {
    label: "Running",
    variant: "success",
    dotClassName: "bg-success",
  },
  stopped: {
    label: "Stopped",
    variant: "muted",
    dotClassName: "bg-muted-foreground",
  },
  warning: {
    label: "Warning",
    variant: "warning",
    dotClassName: "bg-warning",
  },
  error: {
    label: "Error",
    variant: "destructive",
    dotClassName: "bg-destructive",
  },
};

const typeLabels: Record<ServiceType, string> = {
  nginx: "Nginx",
  database: "Database",
  cache: "Cache",
  node: "Node",
  application: "Application",
  system: "System",
};

interface ServiceTableProps {
  services: LocalService[];
  selectedIds: Set<string>;
  visibleColumns: Set<ServiceColumn>;
  pendingIds: Set<string>;
  onSelectionChange: (selectedIds: Set<string>) => void;
  onOperation: (
    service: LocalService,
    operation: ServiceOperation,
  ) => Promise<void>;
  onRemove: (service: LocalService) => void;
  onLocalAction: (title: string, service: LocalService) => void;
}

export function ServiceTable({
  services,
  selectedIds,
  visibleColumns,
  pendingIds,
  onSelectionChange,
  onOperation,
  onRemove,
  onLocalAction,
}: ServiceTableProps) {
  const allSelected =
    services.length > 0 && services.every((service) => selectedIds.has(service.id));
  const someSelected = services.some((service) => selectedIds.has(service.id));

  function toggleAll(checked: boolean) {
    onSelectionChange(
      checked ? new Set(services.map((service) => service.id)) : new Set(),
    );
  }

  function toggleService(serviceId: string, checked: boolean) {
    const nextSelected = new Set(selectedIds);
    if (checked) {
      nextSelected.add(serviceId);
    } else {
      nextSelected.delete(serviceId);
    }
    onSelectionChange(nextSelected);
  }

  return (
    <div className="overflow-hidden rounded-xl border">
      <div className="overflow-x-auto">
        <Table className="min-w-[980px]">
          <TableHeader className="bg-muted/40">
            <TableRow>
              <TableHead className="w-11">
                <Checkbox
                  aria-label="Select all services"
                  checked={allSelected}
                  indeterminate={!allSelected && someSelected}
                  onCheckedChange={toggleAll}
                />
              </TableHead>
              <TableHead className="min-w-64">Service</TableHead>
              {visibleColumns.has("type") ? <TableHead>Type</TableHead> : null}
              {visibleColumns.has("status") ? (
                <TableHead>Status</TableHead>
              ) : null}
              {visibleColumns.has("version") ? (
                <TableHead>Version</TableHead>
              ) : null}
              {visibleColumns.has("port") ? <TableHead>Port</TableHead> : null}
              {visibleColumns.has("cpu") ? (
                <TableHead className="text-right">CPU</TableHead>
              ) : null}
              {visibleColumns.has("memory") ? (
                <TableHead className="text-right">Memory</TableHead>
              ) : null}
              {visibleColumns.has("actions") ? (
                <TableHead className="w-16 text-right">Actions</TableHead>
              ) : null}
            </TableRow>
          </TableHeader>
          <TableBody>
            {services.map((service) => {
              const status = statusConfig[service.status];
              return (
                <TableRow
                  data-state={selectedIds.has(service.id) ? "selected" : undefined}
                  key={service.id}
                >
                  <TableCell>
                    <Checkbox
                      aria-label={`Select ${service.name}`}
                      checked={selectedIds.has(service.id)}
                      onCheckedChange={(checked) =>
                        toggleService(service.id, checked)
                      }
                    />
                  </TableCell>
                  <TableCell>
                    <div className="flex min-w-0 items-center gap-2">
                      <GripVerticalIcon
                        aria-hidden="true"
                        className="size-4 shrink-0 text-muted-foreground/50"
                      />
                      <div className="min-w-0">
                        <p className="truncate text-sm font-medium">
                          {service.name}
                        </p>
                        <p className="max-w-60 truncate text-xs text-muted-foreground">
                          {service.description}
                        </p>
                      </div>
                    </div>
                  </TableCell>
                  {visibleColumns.has("type") ? (
                    <TableCell>
                      <Badge variant="outline">{typeLabels[service.type]}</Badge>
                    </TableCell>
                  ) : null}
                  {visibleColumns.has("status") ? (
                    <TableCell>
                      <Badge variant={status.variant}>
                        <span
                          className={cn("size-1.5 rounded-full", status.dotClassName)}
                        />
                        {status.label}
                      </Badge>
                    </TableCell>
                  ) : null}
                  {visibleColumns.has("version") ? (
                    <TableCell className="font-mono text-xs text-muted-foreground">
                      {service.version}
                    </TableCell>
                  ) : null}
                  {visibleColumns.has("port") ? (
                    <TableCell className="font-mono text-xs">
                      {service.port ?? "—"}
                    </TableCell>
                  ) : null}
                  {visibleColumns.has("cpu") ? (
                    <TableCell className="text-right font-mono text-xs">
                      {service.cpu.toFixed(1)}%
                    </TableCell>
                  ) : null}
                  {visibleColumns.has("memory") ? (
                    <TableCell className="text-right font-mono text-xs">
                      {service.memory}
                    </TableCell>
                  ) : null}
                  {visibleColumns.has("actions") ? (
                    <TableCell className="text-right">
                      <ServiceActions
                        onLocalAction={onLocalAction}
                        onOperation={onOperation}
                        onRemove={onRemove}
                        pending={pendingIds.has(service.id)}
                        service={service}
                      />
                    </TableCell>
                  ) : null}
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
