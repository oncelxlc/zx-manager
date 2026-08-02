import { useState, type ComponentProps } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { GripVerticalIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

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
} from "src/types/service";
import { ServiceActions } from "./ServiceActions";

const statusConfig: Record<ServiceStatus, { variant: ComponentProps<typeof Badge>["variant"]; dotClassName: string }> = {
  running: {
    variant: "success",
    dotClassName: "bg-success",
  },
  stopped: {
    variant: "muted",
    dotClassName: "bg-muted-foreground",
  },
  warning: {
    variant: "warning",
    dotClassName: "bg-warning",
  },
  error: {
    variant: "destructive",
    dotClassName: "bg-destructive",
  },
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
  const { t } = useTranslation(["services", "dashboard"]);
  const [scrollElement, setScrollElement] = useState<HTMLDivElement | null>(null);
  const rowVirtualizer = useVirtualizer({
    count: services.length,
    estimateSize: () => 56,
    getItemKey: (index) => services[index]?.id ?? index,
    getScrollElement: () => scrollElement,
    initialRect: { height: 560, width: 980 },
    overscan: 5,
  });
  const virtualRows = rowVirtualizer.getVirtualItems();
  const renderedRows = virtualRows.length > 0
    ? virtualRows
    : services.slice(0, 15).map((_, index) => ({
      end: (index + 1) * 56,
      index,
      size: 56,
      start: index * 56,
    }));
  const firstVirtualRow = renderedRows[0];
  const lastVirtualRow = renderedRows[renderedRows.length - 1];
  const columnCount = 2 + [...visibleColumns].length;
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
      <div className="max-h-[52.5rem] overflow-auto" ref={setScrollElement}>
        <Table className="min-w-[980px]" containerClassName="overflow-visible">
          <TableHeader className="bg-muted/40">
            <TableRow>
              <TableHead className="w-11">
                <Checkbox
                  aria-label={t("table.selectAll")}
                  checked={allSelected}
                  indeterminate={!allSelected && someSelected}
                  onCheckedChange={toggleAll}
                />
              </TableHead>
              <TableHead className="min-w-64">{t("table.service")}</TableHead>
              {visibleColumns.has("type") ? <TableHead>{t("dashboard:columns.type")}</TableHead> : null}
              {visibleColumns.has("status") ? (
                <TableHead>{t("dashboard:columns.status")}</TableHead>
              ) : null}
              {visibleColumns.has("version") ? (
                <TableHead>{t("dashboard:columns.version")}</TableHead>
              ) : null}
              {visibleColumns.has("port") ? <TableHead>{t("dashboard:columns.port")}</TableHead> : null}
              {visibleColumns.has("cpu") ? (
                <TableHead className="text-right">{t("dashboard:columns.cpu")}</TableHead>
              ) : null}
              {visibleColumns.has("memory") ? (
                <TableHead className="text-right">{t("dashboard:columns.memory")}</TableHead>
              ) : null}
              {visibleColumns.has("actions") ? (
                <TableHead className="w-16 text-right">{t("dashboard:columns.actions")}</TableHead>
              ) : null}
            </TableRow>
          </TableHeader>
          <TableBody>
            {firstVirtualRow?.start ? (
              <TableRow aria-hidden="true" style={{ height: firstVirtualRow.start }}>
                <TableCell colSpan={columnCount} className="p-0" />
              </TableRow>
            ) : null}
            {renderedRows.map((virtualRow) => {
              const service = services[virtualRow.index];
              if (!service) {
                return null;
              }
              const status = statusConfig[service.status];
              return (
                <TableRow
                  data-state={selectedIds.has(service.id) ? "selected" : undefined}
                  key={service.id}
                  style={{ height: virtualRow.size }}
                >
                  <TableCell>
                    <Checkbox
                      aria-label={t("table.select", { name: service.name })}
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
                          {t(`descriptions.${service.descriptionKey}`, service.descriptionValues)}
                        </p>
                      </div>
                    </div>
                  </TableCell>
                  {visibleColumns.has("type") ? (
                    <TableCell>
                      <Badge variant="outline">{t(`types.${service.type}`)}</Badge>
                    </TableCell>
                  ) : null}
                  {visibleColumns.has("status") ? (
                    <TableCell>
                      <Badge variant={status.variant}>
                        <span
                          className={cn("size-1.5 rounded-full", status.dotClassName)}
                        />
                        {t(`status.${service.status}`)}
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
            {lastVirtualRow ? (
              <TableRow
                aria-hidden="true"
                style={{
                  height: Math.max(
                    0,
                    (rowVirtualizer.getTotalSize() || services.length * 56)
                    - lastVirtualRow.end,
                  ),
                }}
              >
                <TableCell colSpan={columnCount} className="p-0" />
              </TableRow>
            ) : null}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
