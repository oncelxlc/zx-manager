import { MonitorIcon, MoonIcon, SunIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuLabel, DropdownMenuRadioGroup, DropdownMenuRadioItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { toast } from "@/components/ui/toast";
import { useTheme } from "@/components/theme-provider";
import type { ThemeMode } from "src/types/preferences";

const modes: ThemeMode[] = ["light", "dark", "system"];

export function ThemeSwitcher({ compact = false }: { compact?: boolean }) {
  const { resolvedTheme, setTheme, theme } = useTheme();
  const { t } = useTranslation("settings");
  const Icon = theme === "system" ? MonitorIcon : theme === "dark" ? MoonIcon : SunIcon;

  function getModeLabel(mode: ThemeMode) {
    if (mode === "system") {
      return t(resolvedTheme === "dark" ? "appearance.systemResolvedDark" : "appearance.systemResolvedLight");
    }
    return t(`appearance.${mode}`);
  }

  function handleThemeChange(value: string) {
    if (value !== "light" && value !== "dark" && value !== "system") {
      return;
    }

    setTheme(value);
    toast.add({ title: t("toast.themeChanged", { theme: getModeLabel(value) }), type: "success" });
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger render={<Button aria-label={t("labels.changeTheme")} size={compact ? "icon-sm" : "sm"} variant="outline" />}>
        <Icon aria-hidden="true" />
        {compact ? null : <span>{getModeLabel(theme)}</span>}
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuRadioGroup onValueChange={handleThemeChange} value={theme}>
          <DropdownMenuLabel>{t("appearance.theme")}</DropdownMenuLabel>
          {modes.map((mode) => (
            <DropdownMenuRadioItem key={mode} value={mode}>
              {getModeLabel(mode)}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
