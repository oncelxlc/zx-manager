import { useTranslation } from "react-i18next";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { LanguageSwitcher } from "./LanguageSwitcher";
import { NetworkMonitorLaunchSwitcher } from "./NetworkMonitorLaunchSwitcher";
import { ThemeSwitcher } from "./ThemeSwitcher";

export function PreferencesDialog({ onOpenChange, open }: { onOpenChange: (open: boolean) => void; open: boolean }) {
  const { t } = useTranslation("settings");
  return (
    <Dialog onOpenChange={onOpenChange} open={open}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{t("title")}</DialogTitle>
          <DialogDescription>{t("storageDescription")}</DialogDescription>
        </DialogHeader>
        <div className="grid gap-5">
          <section className="grid gap-2">
            <h2 className="text-sm font-medium">{t("appearance.title")}</h2>
            <ThemeSwitcher />
          </section>
          <section className="grid gap-2">
            <h2 className="text-sm font-medium">{t("language.title")}</h2>
            <LanguageSwitcher />
          </section>
          <section className="grid gap-2">
            <h2 className="text-sm font-medium">{t("networkMonitor.title")}</h2>
            <NetworkMonitorLaunchSwitcher />
          </section>
        </div>
      </DialogContent>
    </Dialog>
  );
}
