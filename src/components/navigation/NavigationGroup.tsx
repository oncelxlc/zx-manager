import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { LucideIcon } from "lucide-react";
import { ChevronRightIcon } from "lucide-react";

import {
  Collapsible,
  CollapsibleContent,
} from "@/components/ui/collapsible";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuAction,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  useSidebar,
} from "@/components/ui/sidebar";

export interface NavigationItem {
  labelKey: string;
  icon: LucideIcon;
}

interface NavigationLink extends NavigationItem {
  type: "link";
  to: string | null;
}

interface NavigationCollapsible extends NavigationItem {
  type: "collapsible";
  defaultTo: string;
  children: Array<{ labelKey: string; to: string }>;
}

export type NavigationEntry = NavigationLink | NavigationCollapsible;

interface NavigationGroupProps {
  entries: NavigationEntry[];
  pathname: string;
  activeMockItem: string;
  onItemSelect: (labelKey: string, to: string | null) => void;
}

export function NavigationGroup({
  entries,
  pathname,
  activeMockItem,
  onItemSelect,
}: NavigationGroupProps) {
  const { t } = useTranslation("navigation");
  const { state } = useSidebar();
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  function setEntryExpanded(labelKey: string, open: boolean) {
    setExpanded((current) => {
      const next = new Set(current);
      if (open) next.add(labelKey);
      else next.delete(labelKey);
      return next;
    });
  }

  return (
    <SidebarGroup>
      <SidebarGroupContent>
        <SidebarMenu>
          {entries.map((entry) => {
            if (entry.type === "link") {
              const active = entry.to
                ? pathname === entry.to
                : activeMockItem === entry.labelKey;
              return (
                <SidebarMenuItem key={entry.labelKey}>
                  <SidebarMenuButton
                    isActive={active}
                    onClick={() => onItemSelect(entry.labelKey, entry.to)}
                    tooltip={t(entry.labelKey)}
                  >
                    <entry.icon />
                    <span>{t(entry.labelKey)}</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              );
            }

            const active = pathname.startsWith("/nginx/");
            const open = active || expanded.has(entry.labelKey);
            if (state === "collapsed") {
              return (
                <SidebarMenuItem key={entry.labelKey}>
                  <DropdownMenu>
                    <DropdownMenuTrigger
                      render={(
                        <SidebarMenuButton
                          isActive={active}
                          tooltip={t(entry.labelKey)}
                        />
                      )}
                    >
                      <entry.icon />
                      <span>{t(entry.labelKey)}</span>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="start" side="right">
                      <DropdownMenuGroup>
                        {entry.children.map((child) => (
                          <DropdownMenuItem
                            key={child.to}
                            onClick={() => onItemSelect(child.labelKey, child.to)}
                          >
                            {t(child.labelKey)}
                          </DropdownMenuItem>
                        ))}
                      </DropdownMenuGroup>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </SidebarMenuItem>
              );
            }

            return (
              <Collapsible
                key={entry.labelKey}
                onOpenChange={(next) => setEntryExpanded(entry.labelKey, next)}
                open={open}
              >
                <SidebarMenuItem>
                  <SidebarMenuButton
                    isActive={active}
                    onClick={() => onItemSelect(entry.labelKey, entry.defaultTo)}
                    tooltip={t(entry.labelKey)}
                  >
                    <entry.icon />
                    <span>{t(entry.labelKey)}</span>
                  </SidebarMenuButton>
                  <SidebarMenuAction
                    aria-expanded={open}
                    aria-label={t(open ? "labels.collapseNginx" : "labels.expandNginx")}
                    onClick={() => setEntryExpanded(entry.labelKey, !open)}
                  >
                    <ChevronRightIcon
                      className={open ? "rotate-90 transition-transform" : "transition-transform"}
                    />
                  </SidebarMenuAction>
                  <CollapsibleContent>
                    <SidebarMenuSub>
                      {entry.children.map((child) => (
                        <SidebarMenuSubItem key={child.to}>
                          <SidebarMenuSubButton
                            isActive={pathname === child.to}
                            onClick={() => onItemSelect(child.labelKey, child.to)}
                            render={<button type="button" />}
                          >
                            <span>{t(child.labelKey)}</span>
                          </SidebarMenuSubButton>
                        </SidebarMenuSubItem>
                      ))}
                    </SidebarMenuSub>
                  </CollapsibleContent>
                </SidebarMenuItem>
              </Collapsible>
            );
          })}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  );
}
