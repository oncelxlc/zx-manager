import { useCallback, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
  HardDriveIcon,
  MoreHorizontalIcon,
  RefreshCwIcon,
  Settings2Icon,
} from "lucide-react";

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
import { Spinner } from "@/components/ui/spinner";
import { toast } from "@/components/ui/toast";

interface DashboardHeaderActionsProps {
  onActionsChange: (actions: ReactNode) => void;
}

/** Owns dashboard-only header feedback, keeping the route page as an assembler. */
export function DashboardHeaderActions({
  onActionsChange,
}: DashboardHeaderActionsProps) {
  const { t } = useTranslation(["dashboard", "common"]);
  const [refreshing, setRefreshing] = useState(false);

  const showMockAction = useCallback((title: string) => {
    toast.add({
      title,
      description: t("headerActions.mockDescription"),
      type: "info",
    });
  }, [t]);

  const handleRefresh = useCallback(async () => {
    setRefreshing(true);
    await new Promise((resolve) => setTimeout(resolve, 650));
    setRefreshing(false);
    toast.add({
      title: t("refreshSuccessTitle"),
      description: t("refreshSuccessDescription"),
      type: "success",
    });
  }, [t]);

  const actions = useMemo(() => (
    <>
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
              onClick={() => showMockAction(t("headerActions.openSystemReport"))}
            >
              <HardDriveIcon />
              {t("headerActions.openSystemReport")}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => showMockAction(t("headerActions.dashboardSettings"))}
            >
              <Settings2Icon />
              {t("headerActions.dashboardSettings")}
            </DropdownMenuItem>
          </DropdownMenuGroup>
          <DropdownMenuSeparator />
          <DropdownMenuGroup>
            <DropdownMenuItem
              onClick={() => showMockAction(t("headerActions.exportDiagnostics"))}
            >
              {t("headerActions.exportDiagnostics")}
            </DropdownMenuItem>
          </DropdownMenuGroup>
        </DropdownMenuContent>
      </DropdownMenu>
    </>
  ), [handleRefresh, refreshing, showMockAction, t]);

  useEffect(() => {
    onActionsChange(actions);
  }, [actions, onActionsChange]);

  return null;
}
