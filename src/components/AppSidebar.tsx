import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ActivityIcon,
  BellIcon,
  BookOpenIcon,
  BoxIcon,
  CableIcon,
  ChevronDownIcon,
  CircleHelpIcon,
  FileCodeIcon,
  FileKeyIcon,
  GaugeIcon,
  LayoutDashboardIcon,
  MoreHorizontalIcon,
  PlusIcon,
  SearchIcon,
  ServerCogIcon,
  SettingsIcon,
  ShieldCheckIcon,
  TerminalSquareIcon,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
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
  SidebarRail,
  SidebarSeparator,
} from "@/components/ui/sidebar";
import { toast } from "@/components/ui/toast";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { LanguageSwitcher } from "./preferences/LanguageSwitcher";
import { PreferencesDialog } from "./preferences/PreferencesDialog";
import { ThemeSwitcher } from "./preferences/ThemeSwitcher";

interface NavigationItem {
  labelKey: string;
  icon: LucideIcon;
}

const managementItems: NavigationItem[] = [
  { labelKey: "items.dashboard", icon: LayoutDashboardIcon },
  { labelKey: "items.services", icon: ServerCogIcon },
  { labelKey: "items.nginx", icon: BoxIcon },
];

const resourceItems: NavigationItem[] = [
  { labelKey: "items.configuration", icon: FileCodeIcon },
  { labelKey: "items.logs", icon: TerminalSquareIcon },
  { labelKey: "items.certificates", icon: FileKeyIcon },
  { labelKey: "items.networkPorts", icon: CableIcon },
  { labelKey: "items.systemMonitor", icon: ActivityIcon },
];

const footerItems: NavigationItem[] = [
  { labelKey: "items.settings", icon: SettingsIcon },
  { labelKey: "items.help", icon: CircleHelpIcon },
  { labelKey: "items.search", icon: SearchIcon },
];

const quickActions = [
  "quickActions.installNginx",
  "quickActions.addService",
  "quickActions.createProxy",
  "quickActions.openConfiguration",
  "quickActions.importCertificate",
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
  const { t } = useTranslation("navigation");
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
                <item.icon />
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
  const { t } = useTranslation(["navigation", "common"]);
  const [activeItem, setActiveItem] = useState("items.dashboard");
  const [settingsOpen, setSettingsOpen] = useState(false);

  function handleItemSelect(labelKey: string) {
    setActiveItem(labelKey);
    if (labelKey === "items.settings") {
      setSettingsOpen(true);
      return;
    }
    if (labelKey !== "items.dashboard") {
      toast.add({
        title: t("navigation:toast.selected", { label: t(`navigation:${labelKey}`) }),
        description: t("navigation:toast.moduleMock"),
        type: "info",
      });
    }
  }

  return (
    <Sidebar collapsible="icon">
      <SidebarHeader className="gap-3 p-3">
        <div className="flex items-center gap-3 px-1 py-1.5">
          <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground">
            <TerminalSquareIcon className="size-4" aria-hidden="true" />
          </div>
          <div className="min-w-0 group-data-[collapsible=icon]:hidden">
            <p className="truncate text-sm font-semibold">{t("common:app.name")}</p>
            <p className="truncate text-xs text-muted-foreground">
            {t("common:app.description")}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2 group-data-[collapsible=icon]:hidden">
          <DropdownMenu>
            <DropdownMenuTrigger
              render={<Button className="flex-1 justify-between" />}
            >
              <span className="flex items-center gap-1.5">
                <PlusIcon data-icon="inline-start" />
                {t("navigation:labels.quickAction")}
              </span>
              <ChevronDownIcon data-icon="inline-end" />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start" className="w-56">
              <DropdownMenuGroup>
              <DropdownMenuLabel>{t("navigation:labels.localActions")}</DropdownMenuLabel>
              {quickActions.map((action) => (
                <DropdownMenuItem
                  key={action}
                    onClick={() => showMockAction(t(`navigation:${action}`), t("navigation:toast.futureAction"))}
                  >
                    {t(`navigation:${action}`)}
                  </DropdownMenuItem>
                ))}
              </DropdownMenuGroup>
            </DropdownMenuContent>
          </DropdownMenu>

          <Tooltip>
            <TooltipTrigger
              render={
                <Button
                  aria-label={t("navigation:labels.notifications")}
                  size="icon"
                  variant="outline"
                />
              }
            >
              <BellIcon />
            </TooltipTrigger>
            <TooltipContent>{t("navigation:labels.notifications")}</TooltipContent>
          </Tooltip>
        </div>
      </SidebarHeader>

      <SidebarSeparator />

      <SidebarContent>
        <NavigationGroup
          label={t("navigation:groups.management")}
          items={managementItems}
          activeItem={activeItem}
          onItemSelect={handleItemSelect}
        />
        <NavigationGroup
          label={t("navigation:groups.resources")}
          items={resourceItems}
          activeItem={activeItem}
          onItemSelect={handleItemSelect}
        />
      </SidebarContent>

      <SidebarFooter className="gap-2 p-3">
        <SidebarMenu>
          {footerItems.map((item) => (
            <SidebarMenuItem key={item.labelKey}>
              <SidebarMenuButton
                onClick={() => handleItemSelect(item.labelKey)}
                tooltip={t(`navigation:${item.labelKey}`)}
              >
                <item.icon />
                <span>{t(`navigation:${item.labelKey}`)}</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          ))}
        </SidebarMenu>

        <SidebarSeparator />

        <div className="flex items-center gap-1 group-data-[collapsible=icon]:justify-center">
          <LanguageSwitcher compact />
          <ThemeSwitcher compact />
        </div>

        <SidebarSeparator />

        <div className="flex items-center gap-2 group-data-[collapsible=icon]:justify-center">
          <Avatar className="size-8">
            <AvatarFallback>
              <GaugeIcon aria-hidden="true" />
            </AvatarFallback>
          </Avatar>
          <div className="min-w-0 flex-1 group-data-[collapsible=icon]:hidden">
            <p className="truncate text-xs font-medium">{t("navigation:machine.name")}</p>
            <p className="truncate text-xs text-muted-foreground">
              {t("navigation:machine.platform")}
            </p>
          </div>
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button
                  aria-label={t("navigation:labels.openMachineActions")}
                  className="group-data-[collapsible=icon]:hidden"
                  size="icon-sm"
                  variant="ghost"
                />
              }
            >
              <MoreHorizontalIcon />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuGroup>
                <DropdownMenuItem onClick={() => showMockAction(t("navigation:machine.systemInfo"), t("navigation:toast.futureAction"))}>
                  <ShieldCheckIcon />
                  {t("navigation:machine.systemInfo")}
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => showMockAction(t("navigation:machine.openGuide"), t("navigation:toast.futureAction"))}>
                  <BookOpenIcon />
                  {t("navigation:machine.openGuide")}
                </DropdownMenuItem>
              </DropdownMenuGroup>
              <DropdownMenuSeparator />
              <DropdownMenuItem onClick={() => showMockAction(t("navigation:machine.restartApp"), t("navigation:toast.futureAction"))}>
                {t("navigation:machine.restartApp")}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </SidebarFooter>
      <SidebarRail />
      <PreferencesDialog onOpenChange={setSettingsOpen} open={settingsOpen} />
    </Sidebar>
  );
}
