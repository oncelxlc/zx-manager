import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router";
import {
  ActivityIcon,
  BoxIcon,
  CableIcon,
  CircleHelpIcon,
  FileCodeIcon,
  FileKeyIcon,
  LayoutDashboardIcon,
  SearchIcon,
  ServerCogIcon,
  SettingsIcon,
  TerminalSquareIcon,
} from "lucide-react";

import {
  Sidebar,
  SidebarContent,
  SidebarHeader,
} from "@/components/ui/sidebar";
import { toast } from "@/components/ui/toast";
import { restartApplication } from "src/services/tauri/application";
import { writeDiagnosticText } from "src/services/tauri/system-information";
import { useSystemInformationStore } from "src/stores/system-information-store";
import {
  formatMachinePlatform,
  getErrorTranslationKey,
  serializeDiagnosticReport,
} from "src/utils/system-information";
import { PreferencesDialog } from "./preferences/PreferencesDialog";
import {
  NavigationGroup,
  type NavigationItem,
} from "./navigation/NavigationGroup";
import { SidebarFooterContent } from "./sidebar/SidebarFooterContent";

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

function showMockAction(label: string, description: string) {
  toast.add({
    title: label,
    description,
    type: "info",
  });
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

      <SidebarFooterContent
        footerItems={[
          {labelKey: "items.settings", icon: SettingsIcon},
          {labelKey: "items.help", icon: CircleHelpIcon},
          {labelKey: "items.search", icon: SearchIcon},
        ]}
        onCopyDiagnostics={() => void handleCopyDiagnostics()}
        onItemSelect={handleItemSelect}
        onOpenGuide={() => showMockAction(t("navigation:machine.openGuide"), t("navigation:toast.futureAction"))}
        onOpenSystemInformation={openSystemInformation}
        onRefreshSummary={() => void handleRefreshSummary()}
        onRestart={() => void handleRestartApplication()}
        onRestartConfirmationChange={(open) => {
          if (!open && !restarting) {
            setRestartConfirmationOpen(false);
          }
        }}
        onRestartRequest={() => setRestartConfirmationOpen(true)}
        restartConfirmationOpen={restartConfirmationOpen}
        restarting={restarting}
        summary={summary}
        summaryLoading={summaryLoading}
      />
      <PreferencesDialog onOpenChange={setSettingsOpen} open={settingsOpen}/>
    </Sidebar>
  );
}
