import { useTranslation } from "react-i18next";
import type { LucideIcon } from "lucide-react";

import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuBadge,
  SidebarMenuItem,
} from "@/components/ui/sidebar";

export interface NavigationItem {
  labelKey: string;
  icon: LucideIcon;
}

interface NavigationGroupProps {
  label: string;
  items: NavigationItem[];
  activeItem: string;
  onItemSelect: (labelKey: string) => void;
  badges?: Partial<Record<string, number>>;
}

export function NavigationGroup({
  label,
  items,
  activeItem,
  onItemSelect,
  badges,
}: NavigationGroupProps) {
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
              {(badges?.[item.labelKey] ?? 0) > 0 ? (
                <SidebarMenuBadge>
                  {badges?.[item.labelKey]}
                </SidebarMenuBadge>
              ) : null}
            </SidebarMenuItem>
          ))}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  );
}
