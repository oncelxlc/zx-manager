import { LanguagesIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuLabel, DropdownMenuRadioGroup, DropdownMenuRadioItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { toast } from "@/components/ui/toast";
import { changeLocale } from "src/i18n";
import { setLocale } from "src/services/storage/preferences-storage";
import type { SupportedLocale } from "src/types/preferences";

const locales: SupportedLocale[] = ["zh-CN", "en-US"];

export function LanguageSwitcher({ compact = false }: { compact?: boolean }) {
  const { i18n, t } = useTranslation("settings");
  const locale = i18n.language === "en-US" ? "en-US" : "zh-CN";

  async function handleLocaleChange(value: string) {
    if (value !== "zh-CN" && value !== "en-US") {
      return;
    }

    await changeLocale(value);
    await setLocale(value);
    toast.add({
      title: t("toast.languageChanged", { language: t(`language.${value === "zh-CN" ? "zhCN" : "enUS"}`) }),
      type: "success",
    });
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger render={<Button aria-label={t("labels.changeLanguage")} size={compact ? "icon-sm" : "sm"} variant="outline" />}>
        <LanguagesIcon aria-hidden="true" />
        {compact ? null : <span>{t(`language.${locale === "zh-CN" ? "zhCN" : "enUS"}`)}</span>}
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuRadioGroup onValueChange={(value) => void handleLocaleChange(value)} value={locale}>
          <DropdownMenuLabel>{t("language.label")}</DropdownMenuLabel>
          {locales.map((value) => (
            <DropdownMenuRadioItem key={value} value={value}>
              {t(`language.${value === "zh-CN" ? "zhCN" : "enUS"}`)}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
