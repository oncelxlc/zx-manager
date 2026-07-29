import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router";
import {
  ActivityIcon,
  BookOpenIcon,
  BoxIcon,
  CableIcon,
  CircleHelpIcon,
  CopyIcon,
  FileCodeIcon,
  FileKeyIcon,
  GaugeIcon,
  LayoutDashboardIcon,
  MoreVerticalIcon,
  RefreshCwIcon,
  SearchIcon,
  ServerCogIcon,
  SettingsIcon,
  ShieldCheckIcon,
  TerminalSquareIcon,
  TriangleAlertIcon,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

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
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarSeparator,
} from "@/components/ui/sidebar";
import { Spinner } from "@/components/ui/spinner";
import { toast } from "@/components/ui/toast";
import { restartApplication } from "src/services/tauri/application";
import { writeDiagnosticText } from "src/services/tauri/system-information";
import { useSystemInformationStore } from "src/stores/system-information-store";
import {
  formatMachinePlatform,
  getErrorTranslationKey,
  serializeDiagnosticReport,
} from "src/utils/system-information";
import { LanguageSwitcher } from "./preferences/LanguageSwitcher";
import { PreferencesDialog } from "./preferences/PreferencesDialog";
import { ThemeSwitcher } from "./preferences/ThemeSwitcher";

interface NavigationItem {
  labelKey: string;
  icon: LucideIcon;
}

const managementItems: NavigationItem[] = [
  {labelKey: "items.dashboard", icon: LayoutDashboardIcon},
  {labelKey: "items.services", icon: ServerCogIcon},
  {labelKey: "items.nginx", icon: BoxIcon},
];

const resourceItems: NavigationItem[] = [
  {labelKey: "items.configuration", icon: FileCodeIcon},
  {labelKey: "items.logs", icon: TerminalSquareIcon},
  {labelKey: "items.certificates", icon: FileKeyIcon},
  {labelKey: "items.networkPorts", icon: CableIcon},
  {labelKey: "items.networkMonitor", icon: ActivityIcon},
];

const footerItems: NavigationItem[] = [
  {labelKey: "items.settings", icon: SettingsIcon},
  {labelKey: "items.help", icon: CircleHelpIcon},
  {labelKey: "items.search", icon: SearchIcon},
];

function showMockAction(label: string, description: string) {
  toast.add({
    title: label,
    description,
    type: "info",
  });
}

function NavigationGroup({
                           label,
                           items,
                           activeItem,
                           onItemSelect,
                         }: {
  label: string;
  items: NavigationItem[];
  activeItem: string;
  onItemSelect: (labelKey: string) => void;
}) {
  const {t} = useTranslation("navigation");
  return (
    <SidebarGroup>
      <SidebarGroupLabel>{label}</SidebarGroupLabel>
      <SidebarGroupContent>
        <SidebarMenu>
          {items.map((item) => (
            <SidebarMenuItem key={item.labelKey}>
              <SidebarMenuButton
                isActive={activeItem === item.labelKey}
                onClick={() => onItemSelect(item.labelKey)}
                tooltip={t(item.labelKey)}
              >
                <item.icon/>
                <span>{t(item.labelKey)}</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          ))}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  );
}

export function AppSidebar() {
  const {t} = useTranslation(["navigation", "common"]);
  const location = useLocation();
  const navigate = useNavigate();
  const [activeItem, setActiveItem] = useState("items.dashboard");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [restartConfirmationOpen, setRestartConfirmationOpen] = useState(false);
  const [restarting, setRestarting] = useState(false);
  const summary = useSystemInformationStore((state) => state.summary);
  const information = useSystemInformationStore((state) => state.information);
  const loadSummary = useSystemInformationStore((state) => state.loadSummary);
  const loadInformation = useSystemInformationStore((state) => state.loadInformation);
  const summaryLoading = useSystemInformationStore(
    (state) => state.summaryStatus === "loading",
  );
  const activeNavigationItem =
    location.pathname === "/"
      ? "items.dashboard"
      : location.pathname === "/system-information"
        ? ""
        : location.pathname === "/network-monitor"
          ? "items.networkMonitor"
        : activeItem;

  useEffect(() => {
    void loadSummary();
  }, [loadSummary]);

  function handleItemSelect(labelKey: string) {
    setActiveItem(labelKey);
    if (labelKey === "items.settings") {
      setSettingsOpen(true);
      return;
    }
    if (labelKey === "items.dashboard") {
      void navigate("/");
      return;
    }
    if (labelKey === "items.networkMonitor") {
      void navigate("/network-monitor");
      return;
    }
    if (labelKey !== "items.dashboard") {
      toast.add({
        title: t("navigation:toast.selected", {label: t(`navigation:${labelKey}`)}),
        description: t("navigation:toast.moduleMock"),
        type: "info",
      });
    }
  }

  function openSystemInformation() {
    setActiveItem("");
    void navigate("/system-information");
  }

  async function handleRefreshSummary() {
    const refreshed = await loadSummary({force: true});
    const error = useSystemInformationStore.getState().summaryError;
    toast.add({
      title: refreshed
        ? t("navigation:toast.summaryRefreshSuccess")
        : t("navigation:toast.summaryRefreshError"),
      description: refreshed
        ? formatMachinePlatform(
          refreshed,
          t("navigation:machine.platformUnavailable"),
        )
        : t(
          `systemInformation:${getErrorTranslationKey(error)}`,
          error?.message || t("systemInformation:errors.unknown"),
        ),
      type: refreshed ? "success" : "error",
    });
  }

  async function handleCopyDiagnostics() {
    try {
      const snapshot = information ?? await loadInformation();
      if (!snapshot) {
        const error = useSystemInformationStore.getState().informationError;
        throw error ?? new Error(t("systemInformation:errors.unknown"));
      }
      await writeDiagnosticText(serializeDiagnosticReport(snapshot));
      toast.add({
        title: t("navigation:toast.copyDiagnosticsSuccess"),
        description: t("navigation:toast.copyDiagnosticsSuccessDescription"),
        type: "success",
      });
    } catch (error) {
      const message =
        typeof error === "object"
        && error !== null
        && "message" in error
        && typeof error.message === "string"
          ? error.message
          : t("systemInformation:errors.unknown");
      toast.add({
        title: t("navigation:toast.copyDiagnosticsError"),
        description: message,
        type: "error",
      });
    }
  }

  async function handleRestartApplication() {
    setRestarting(true);
    try {
      await restartApplication();
    } catch (error) {
      const message = error instanceof Error
        ? error.message
        : typeof error === "string"
          ? error
          : t("systemInformation:errors.unknown");
      setRestarting(false);
      toast.add({
        title: t("navigation:machine.restartError"),
        description: message,
        type: "error",
      });
    }
  }

  return (
    <Sidebar
      collapsible="icon"
      className="w-60 group-data-[side=left]:border-r-0 group-data-[side=right]:border-l-0"
    >
      <SidebarHeader className="gap-3 p-3 group-data-[collapsible=icon]:p-2">
        <div
          className="flex items-center gap-3 px-1 py-1.5 group-data-[collapsible=icon]:justify-center group-data-[collapsible=icon]:px-0"
        >
          <div
            className="flex size-8 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground">
            <TerminalSquareIcon className="size-4" aria-hidden="true"/>
          </div>
          <div className="min-w-0 group-data-[collapsible=icon]:hidden">
            <p className="truncate text-sm font-semibold">{t("common:app.name")}</p>
            <p className="truncate text-xs text-muted-foreground">
              {t("common:app.description")}
            </p>
          </div>
        </div>
      </SidebarHeader>

      <SidebarContent>
        <NavigationGroup
          label={t("navigation:groups.management")}
          items={managementItems}
          activeItem={activeNavigationItem}
          onItemSelect={handleItemSelect}
        />
        <NavigationGroup
          label={t("navigation:groups.resources")}
          items={resourceItems}
          activeItem={activeNavigationItem}
          onItemSelect={handleItemSelect}
        />
      </SidebarContent>

      <SidebarFooter className="gap-2 p-3 group-data-[collapsible=icon]:p-2">
        <SidebarMenu>
          {footerItems.map((item) => (
            <SidebarMenuItem key={item.labelKey}>
              <SidebarMenuButton
                onClick={() => handleItemSelect(item.labelKey)}
                tooltip={t(`navigation:${item.labelKey}`)}
              >
                <item.icon/>
                <span>{t(`navigation:${item.labelKey}`)}</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          ))}
        </SidebarMenu>

        <SidebarSeparator className="m-0"/>

        <div
          data-slot="sidebar-preference-actions"
          className="flex items-center gap-2 group-data-[collapsible=icon]:flex-col group-data-[collapsible=icon]:justify-center"
        >
          <LanguageSwitcher compact/>
          <ThemeSwitcher compact/>
        </div>

        <SidebarSeparator className="m-0"/>

        <SidebarMenu>
          <SidebarMenuItem>
            <DropdownMenu orientation="vertical">
              <DropdownMenuTrigger
                render={
                  <SidebarMenuButton
                    aria-label={t("navigation:labels.openMachineActions")}
                    size="lg"
                  />
                }
              >
                <Avatar className="size-8">
                  <AvatarFallback>
                    <GaugeIcon aria-hidden="true"/>
                  </AvatarFallback>
                </Avatar>
                <div className="min-w-0 flex-auto text-left group-data-[collapsible=icon]:hidden">
                  <p className="truncate text-xs font-medium">{t("navigation:machine.name")}</p>
                  <p className="truncate text-xs text-muted-foreground">
                    {summaryLoading && !summary
                      ? t("navigation:machine.platformUnavailable")
                      : formatMachinePlatform(
                        summary,
                        t("navigation:machine.platformUnavailable"),
                      )}
                  </p>
                </div>
                <MoreVerticalIcon
                  aria-hidden="true"
                  className="ml-auto group-data-[collapsible=icon]:hidden"
                />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" side="left">
                <DropdownMenuGroup>
                  <DropdownMenuItem onClick={openSystemInformation}>
                    <ShieldCheckIcon/>
                    {t("navigation:machine.systemInfo")}
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    disabled={summaryLoading}
                    onClick={() => void handleRefreshSummary()}
                  >
                    <RefreshCwIcon/>
                    {t("navigation:machine.refreshSummary")}
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => void handleCopyDiagnostics()}>
                    <CopyIcon/>
                    {t("navigation:machine.copyDiagnostics")}
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    onClick={() => showMockAction(t("navigation:machine.openGuide"), t("navigation:toast.futureAction"))}>
                    <BookOpenIcon/>
                    {t("navigation:machine.openGuide")}
                  </DropdownMenuItem>
                </DropdownMenuGroup>
                <DropdownMenuSeparator/>
                <DropdownMenuGroup>
                  <DropdownMenuItem
                    onClick={() => setRestartConfirmationOpen(true)}
                  >
                    {t("navigation:machine.restartApp")}
                  </DropdownMenuItem>
                </DropdownMenuGroup>
              </DropdownMenuContent>
            </DropdownMenu>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>
      <PreferencesDialog onOpenChange={setSettingsOpen} open={settingsOpen}/>
      <AlertDialog
        onOpenChange={(open) => {
          if (!open && !restarting) {
            setRestartConfirmationOpen(false);
          }
        }}
        open={restartConfirmationOpen}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogMedia>
              <TriangleAlertIcon aria-hidden="true" />
            </AlertDialogMedia>
            <AlertDialogTitle>
              {t("navigation:machine.restartConfirmationTitle")}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t("navigation:machine.restartConfirmationDescription")}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={restarting}>
              {t("common:actions.cancel")}
            </AlertDialogCancel>
            <AlertDialogAction
              disabled={restarting}
              onClick={() => void handleRestartApplication()}
              variant="destructive"
            >
              {restarting ? <Spinner data-icon="inline-start" /> : null}
              {t("navigation:machine.restartApp")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </Sidebar>
  );
}
