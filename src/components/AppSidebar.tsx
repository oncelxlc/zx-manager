import { useState } from "react";
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

interface NavigationItem {
  label: string;
  icon: LucideIcon;
}

const managementItems: NavigationItem[] = [
  { label: "Dashboard", icon: LayoutDashboardIcon },
  { label: "Services", icon: ServerCogIcon },
  { label: "Nginx", icon: BoxIcon },
];

const resourceItems: NavigationItem[] = [
  { label: "Configuration", icon: FileCodeIcon },
  { label: "Logs", icon: TerminalSquareIcon },
  { label: "Certificates", icon: FileKeyIcon },
  { label: "Network Ports", icon: CableIcon },
  { label: "System Monitor", icon: ActivityIcon },
];

const footerItems: NavigationItem[] = [
  { label: "Settings", icon: SettingsIcon },
  { label: "Help", icon: CircleHelpIcon },
  { label: "Search", icon: SearchIcon },
];

const quickActions = [
  "Install Nginx",
  "Add Service",
  "Create Proxy",
  "Open Configuration",
  "Import Certificate",
];

function showMockAction(label: string) {
  toast.add({
    title: label,
    description: "This action is ready for a future Tauri command.",
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
  onItemSelect: (label: string) => void;
}) {
  return (
    <SidebarGroup>
      <SidebarGroupLabel>{label}</SidebarGroupLabel>
      <SidebarGroupContent>
        <SidebarMenu>
          {items.map((item) => (
            <SidebarMenuItem key={item.label}>
              <SidebarMenuButton
                isActive={activeItem === item.label}
                onClick={() => onItemSelect(item.label)}
                tooltip={item.label}
              >
                <item.icon />
                <span>{item.label}</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          ))}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  );
}

export function AppSidebar() {
  const [activeItem, setActiveItem] = useState("Dashboard");

  function handleItemSelect(label: string) {
    setActiveItem(label);
    if (label !== "Dashboard") {
      toast.add({
        title: `${label} selected`,
        description: "This module is represented by a local dashboard mock.",
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
            <p className="truncate text-sm font-semibold">Local Console</p>
            <p className="truncate text-xs text-muted-foreground">
              Infrastructure Manager
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
                Quick Action
              </span>
              <ChevronDownIcon data-icon="inline-end" />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start" className="w-56">
              <DropdownMenuGroup>
                <DropdownMenuLabel>Local actions</DropdownMenuLabel>
                {quickActions.map((action) => (
                  <DropdownMenuItem
                    key={action}
                    onClick={() => showMockAction(action)}
                  >
                    {action}
                  </DropdownMenuItem>
                ))}
              </DropdownMenuGroup>
            </DropdownMenuContent>
          </DropdownMenu>

          <Tooltip>
            <TooltipTrigger
              render={
                <Button
                  aria-label="Open notifications"
                  size="icon"
                  variant="outline"
                />
              }
            >
              <BellIcon />
            </TooltipTrigger>
            <TooltipContent>Notifications</TooltipContent>
          </Tooltip>
        </div>
      </SidebarHeader>

      <SidebarSeparator />

      <SidebarContent>
        <NavigationGroup
          label="Management"
          items={managementItems}
          activeItem={activeItem}
          onItemSelect={handleItemSelect}
        />
        <NavigationGroup
          label="Resources"
          items={resourceItems}
          activeItem={activeItem}
          onItemSelect={handleItemSelect}
        />
      </SidebarContent>

      <SidebarFooter className="gap-2 p-3">
        <SidebarMenu>
          {footerItems.map((item) => (
            <SidebarMenuItem key={item.label}>
              <SidebarMenuButton
                onClick={() => showMockAction(item.label)}
                tooltip={item.label}
              >
                <item.icon />
                <span>{item.label}</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          ))}
        </SidebarMenu>

        <SidebarSeparator />

        <div className="flex items-center gap-2 group-data-[collapsible=icon]:justify-center">
          <Avatar className="size-8">
            <AvatarFallback>
              <GaugeIcon aria-hidden="true" />
            </AvatarFallback>
          </Avatar>
          <div className="min-w-0 flex-1 group-data-[collapsible=icon]:hidden">
            <p className="truncate text-xs font-medium">Local Machine</p>
            <p className="truncate text-xs text-muted-foreground">
              Windows 11 · x64
            </p>
          </div>
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button
                  aria-label="Open machine actions"
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
                <DropdownMenuItem onClick={() => showMockAction("System info")}>
                  <ShieldCheckIcon />
                  System info
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => showMockAction("Open guide")}>
                  <BookOpenIcon />
                  Open guide
                </DropdownMenuItem>
              </DropdownMenuGroup>
              <DropdownMenuSeparator />
              <DropdownMenuItem onClick={() => showMockAction("Restart app")}>
                Restart app
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  );
}
