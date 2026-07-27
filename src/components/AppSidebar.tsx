import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ActivityIcon,
  BookOpenIcon,
  BoxIcon,
  CableIcon,
  CircleHelpIcon,
  FileCodeIcon,
  FileKeyIcon,
  GaugeIcon,
  LayoutDashboardIcon,
  MoreVerticalIcon,
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
import { toast } from "@/components/ui/toast";
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
  {labelKey: "items.systemMonitor", icon: ActivityIcon},
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
        title: t("navigation:toast.selected", {label: t(`navigation:${labelKey}`)}),
        description: t("navigation:toast.moduleMock"),
        type: "info",
      });
    }
  }

  return (
    <Sidebar collapsible="icon" className="w-70">
      <SidebarHeader className="gap-3 p-3">
        <div className="flex items-center gap-3 px-1 py-1.5">
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
                <item.icon/>
                <span>{t(`navigation:${item.labelKey}`)}</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          ))}
        </SidebarMenu>

        <SidebarSeparator className="m-0"/>

        <div className="flex items-center gap-2 group-data-[collapsible=icon]:justify-center">
          <LanguageSwitcher compact/>
          <ThemeSwitcher compact/>
        </div>

        <SidebarSeparator className="m-0"/>

        <DropdownMenu orientation="vertical">
          <DropdownMenuTrigger
            render={
              <Button
                aria-label={t("navigation:labels.openMachineActions")}
                className="w-full h-auto p-2 group-data-[collapsible=icon]:hidden"
                size="icon-sm"
                variant="ghost"
              />
            }
          >
            <div
              className="w-full flex items-center gap-2 group-data-[collapsible=icon]:justify-center">
              <Avatar className="size-8">
                <AvatarFallback>
                  <GaugeIcon aria-hidden="true"/>
                </AvatarFallback>
              </Avatar>
              <div className="min-w-0 flex-auto text-left group-data-[collapsible=icon]:hidden">
                <p className="truncate text-xs font-medium">{t("navigation:machine.name")}</p>
                <p className="truncate text-xs text-muted-foreground">
                  {t("navigation:machine.platform")}
                </p>
              </div>
              <MoreVerticalIcon/>
            </div>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" side="left">
            <DropdownMenuGroup>
              <DropdownMenuItem
                onClick={() => showMockAction(t("navigation:machine.systemInfo"), t("navigation:toast.futureAction"))}>
                <ShieldCheckIcon/>
                {t("navigation:machine.systemInfo")}
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={() => showMockAction(t("navigation:machine.openGuide"), t("navigation:toast.futureAction"))}>
                <BookOpenIcon/>
                {t("navigation:machine.openGuide")}
              </DropdownMenuItem>
            </DropdownMenuGroup>
            <DropdownMenuSeparator/>
            <DropdownMenuItem
              onClick={() => showMockAction(t("navigation:machine.restartApp"), t("navigation:toast.futureAction"))}>
              {t("navigation:machine.restartApp")}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </SidebarFooter>
      <PreferencesDialog onOpenChange={setSettingsOpen} open={settingsOpen}/>
    </Sidebar>
  );
}
