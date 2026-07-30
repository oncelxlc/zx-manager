import {
  BookOpenIcon,
  CopyIcon,
  GaugeIcon,
  MoreVerticalIcon,
  RefreshCwIcon,
  ShieldCheckIcon,
  TriangleAlertIcon,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogMedia,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  SidebarFooter,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarSeparator,
} from "@/components/ui/sidebar";
import { Spinner } from "@/components/ui/spinner";
import type { SystemSummary } from "src/types/system-information";
import { formatMachinePlatform } from "src/utils/system-information";
import type { NavigationItem } from "../navigation/NavigationGroup";
import { LanguageSwitcher } from "../preferences/LanguageSwitcher";
import { ThemeSwitcher } from "../preferences/ThemeSwitcher";

interface SidebarFooterContentProps {
  summary: SystemSummary | null;
  summaryLoading: boolean;
  restarting: boolean;
  restartConfirmationOpen: boolean;
  onItemSelect: (labelKey: string) => void;
  onOpenSystemInformation: () => void;
  onRefreshSummary: () => void;
  onCopyDiagnostics: () => void;
  onOpenGuide: () => void;
  onRestartRequest: () => void;
  onRestartConfirmationChange: (open: boolean) => void;
  onRestart: () => void;
  footerItems: NavigationItem[];
}

function MachineMenu({
  summary,
  summaryLoading,
  onOpenSystemInformation,
  onRefreshSummary,
  onCopyDiagnostics,
  onOpenGuide,
  onRestartRequest,
}: Pick<
  SidebarFooterContentProps,
  "summary" | "summaryLoading" | "onOpenSystemInformation" | "onRefreshSummary" | "onCopyDiagnostics" | "onOpenGuide" | "onRestartRequest"
>) {
  const { t } = useTranslation(["navigation", "common"]);
  const unavailable = t("navigation:machine.platformUnavailable");
  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <DropdownMenu orientation="vertical">
          <DropdownMenuTrigger render={<SidebarMenuButton aria-label={t("navigation:labels.openMachineActions")} size="lg" />}>
            <Avatar className="size-8"><AvatarFallback><GaugeIcon aria-hidden="true" /></AvatarFallback></Avatar>
            <div className="min-w-0 flex-auto text-left group-data-[collapsible=icon]:hidden">
              <p className="truncate text-xs font-medium">{t("navigation:machine.name")}</p>
              <p className="truncate text-xs text-muted-foreground">
                {summaryLoading && !summary ? unavailable : formatMachinePlatform(summary, unavailable)}
              </p>
            </div>
            <MoreVerticalIcon aria-hidden="true" className="ml-auto group-data-[collapsible=icon]:hidden" />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" side="left">
            <DropdownMenuGroup>
              <DropdownMenuItem onClick={onOpenSystemInformation}><ShieldCheckIcon />{t("navigation:machine.systemInfo")}</DropdownMenuItem>
              <DropdownMenuItem disabled={summaryLoading} onClick={onRefreshSummary}><RefreshCwIcon />{t("navigation:machine.refreshSummary")}</DropdownMenuItem>
              <DropdownMenuItem onClick={onCopyDiagnostics}><CopyIcon />{t("navigation:machine.copyDiagnostics")}</DropdownMenuItem>
              <DropdownMenuItem onClick={onOpenGuide}><BookOpenIcon />{t("navigation:machine.openGuide")}</DropdownMenuItem>
            </DropdownMenuGroup>
            <DropdownMenuSeparator />
            <DropdownMenuGroup><DropdownMenuItem onClick={onRestartRequest}>{t("navigation:machine.restartApp")}</DropdownMenuItem></DropdownMenuGroup>
          </DropdownMenuContent>
        </DropdownMenu>
      </SidebarMenuItem>
    </SidebarMenu>
  );
}

function RestartConfirmationDialog({
  open,
  restarting,
  onOpenChange,
  onRestart,
}: {
  open: boolean;
  restarting: boolean;
  onOpenChange: (open: boolean) => void;
  onRestart: () => void;
}) {
  const { t } = useTranslation(["navigation", "common"]);
  return (
    <AlertDialog onOpenChange={onOpenChange} open={open}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogMedia><TriangleAlertIcon aria-hidden="true" /></AlertDialogMedia>
          <AlertDialogTitle>{t("navigation:machine.restartConfirmationTitle")}</AlertDialogTitle>
          <AlertDialogDescription>{t("navigation:machine.restartConfirmationDescription")}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={restarting}>{t("common:actions.cancel")}</AlertDialogCancel>
          <AlertDialogAction disabled={restarting} onClick={onRestart} variant="destructive">
            {restarting ? <Spinner data-icon="inline-start" /> : null}
            {t("navigation:machine.restartApp")}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

export function SidebarFooterContent(props: SidebarFooterContentProps) {
  const { t } = useTranslation("navigation");
  return (
    <>
      <SidebarFooter className="gap-2 p-3 group-data-[collapsible=icon]:p-2">
        <SidebarMenu>{props.footerItems.map((item) => (
          <SidebarMenuItem key={item.labelKey}><SidebarMenuButton onClick={() => props.onItemSelect(item.labelKey)} tooltip={t(item.labelKey)}><item.icon /><span>{t(item.labelKey)}</span></SidebarMenuButton></SidebarMenuItem>
        ))}</SidebarMenu>
        <SidebarSeparator className="m-0" />
        <div data-slot="sidebar-preference-actions" className="flex items-center gap-2 group-data-[collapsible=icon]:flex-col group-data-[collapsible=icon]:justify-center"><LanguageSwitcher compact /><ThemeSwitcher compact /></div>
        <SidebarSeparator className="m-0" />
        <MachineMenu {...props} />
      </SidebarFooter>
      <RestartConfirmationDialog
        onOpenChange={props.onRestartConfirmationChange}
        onRestart={props.onRestart}
        open={props.restartConfirmationOpen}
        restarting={props.restarting}
      />
    </>
  );
}
