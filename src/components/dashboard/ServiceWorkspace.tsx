import { useMemo, useState, type ComponentProps } from "react";
import { Columns3Icon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardHeader,
} from "@/components/ui/card";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
import {
  configurationChanges,
  healthChecks,
  recentEvents,
} from "src/data/dashboard-mock-data";
import type {
  ActivityRecord,
  AddServiceInput,
  LocalService,
  ServiceColumn,
  ServiceOperation,
} from "src/types/service";
import { AddServiceDialog } from "./AddServiceDialog";
import { ServiceTable } from "./ServiceTable";

const allColumns: ServiceColumn[] = [
  "type",
  "status",
  "version",
  "port",
  "cpu",
  "memory",
  "actions",
];

const activityVariant: Record<
  ActivityRecord["status"],
  ComponentProps<typeof Badge>["variant"]
> = {
  info: "secondary",
  success: "success",
  warning: "warning",
};

type WorkspaceTab = "services" | "events" | "changes" | "health";

function isWorkspaceTab(value: string): value is WorkspaceTab {
  return (
    value === "services" ||
    value === "events" ||
    value === "changes" ||
    value === "health"
  );
}

function ActivityPanel({
  items,
  label,
}: {
  items: ActivityRecord[];
  label: string;
}) {
  const { t } = useTranslation("dashboard");
  return (
    <div aria-label={label} className="overflow-hidden rounded-xl border">
      {items.map((item) => (
        <div
          className="flex items-start justify-between gap-5 border-b px-4 py-3 last:border-b-0 hover:bg-muted/30"
          key={item.id}
        >
          <div className="min-w-0">
            <p className="truncate text-sm font-medium">{item.titleKey.includes(".") ? item.titleKey : t(`activity.titles.${item.titleKey}`)}</p>
            <p className="mt-0.5 truncate text-xs text-muted-foreground">
              {t(`activity.descriptions.${item.descriptionKey}`)}
            </p>
          </div>
          <div className="flex shrink-0 items-center gap-3">
            <Badge variant={activityVariant[item.status]}>{t(`activity.status.${item.status}`)}</Badge>
            <span className="min-w-28 text-right text-xs text-muted-foreground">
              {t(`activity.timestamps.${item.timestampKey}`)}
            </span>
          </div>
        </div>
      ))}
    </div>
  );
}

interface ServiceWorkspaceProps {
  services: LocalService[];
  pendingIds: Set<string>;
  onAddService: (input: AddServiceInput) => void;
  onOperation: (
    service: LocalService,
    operation: ServiceOperation,
  ) => Promise<void>;
  onRemove: (service: LocalService) => void;
  onLocalAction: (title: string, service: LocalService) => void;
}

export function ServiceWorkspace({
  services,
  pendingIds,
  onAddService,
  onOperation,
  onRemove,
  onLocalAction,
}: ServiceWorkspaceProps) {
  const { t } = useTranslation("dashboard");
  const [activeTab, setActiveTab] = useState<WorkspaceTab>("services");
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [visibleColumns, setVisibleColumns] = useState<Set<ServiceColumn>>(
    new Set(allColumns),
  );

  const activeSelectedIds = useMemo(() => {
    const serviceIds = new Set(services.map((service) => service.id));
    return new Set([...selectedIds].filter((id) => serviceIds.has(id)));
  }, [selectedIds, services]);

  function toggleColumn(column: ServiceColumn, checked: boolean) {
    setVisibleColumns((current) => {
      const nextColumns = new Set(current);
      if (checked) {
        nextColumns.add(column);
      } else {
        nextColumns.delete(column);
      }
      return nextColumns;
    });
  }

  return (
    <Tabs
      className="gap-0"
      onValueChange={(value) => {
        if (isWorkspaceTab(value)) {
          setActiveTab(value);
        }
      }}
      value={activeTab}
    >
      <Card>
        <CardHeader className="border-b">
          <TabsList aria-label={t("tabs.views")} variant="line">
            <TabsTrigger value="services">{t("tabs.services")} {services.length}</TabsTrigger>
            <TabsTrigger value="events">
              {t("tabs.events")} {recentEvents.length}
            </TabsTrigger>
            <TabsTrigger value="changes">
              {t("tabs.changes")} {configurationChanges.length}
            </TabsTrigger>
            <TabsTrigger value="health">
              {t("tabs.health")} {healthChecks.length}
            </TabsTrigger>
          </TabsList>
          <CardAction className="flex items-center gap-2">
            <DropdownMenu>
              <DropdownMenuTrigger render={<Button variant="outline" />}>
                <Columns3Icon data-icon="inline-start" />
                {t("columnSettings.customize")}
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-48">
                <DropdownMenuGroup>
                  <DropdownMenuLabel>{t("columnSettings.visible")}</DropdownMenuLabel>
                  <DropdownMenuSeparator />
                  {allColumns.map((column) => (
                    <DropdownMenuCheckboxItem
                      checked={visibleColumns.has(column)}
                      key={column}
                      onCheckedChange={(checked) =>
                        toggleColumn(column, checked)
                      }
                    >
                      {t(`columns.${column}`)}
                    </DropdownMenuCheckboxItem>
                  ))}
                </DropdownMenuGroup>
              </DropdownMenuContent>
            </DropdownMenu>
            <AddServiceDialog onAddService={onAddService} />
          </CardAction>
        </CardHeader>
        <CardContent className="pt-4">
          <TabsContent value="services">
            <ServiceTable
              onLocalAction={onLocalAction}
              onOperation={onOperation}
              onRemove={onRemove}
              onSelectionChange={setSelectedIds}
              pendingIds={pendingIds}
              selectedIds={activeSelectedIds}
              services={services}
              visibleColumns={visibleColumns}
            />
          </TabsContent>
          <TabsContent value="events">
            <ActivityPanel items={recentEvents} label={t("activity.events")} />
          </TabsContent>
          <TabsContent value="changes">
            <ActivityPanel
              items={configurationChanges}
              label={t("activity.changes")}
            />
          </TabsContent>
          <TabsContent value="health">
            <ActivityPanel items={healthChecks} label={t("activity.health")} />
          </TabsContent>
        </CardContent>
      </Card>
    </Tabs>
  );
}
