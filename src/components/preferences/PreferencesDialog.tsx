import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldGroup,
  FieldTitle,
} from "@/components/ui/field";
import { Switch } from "@/components/ui/switch";
import {
  getPreferences,
  setNetworkMonitorStartOnLaunch,
} from "src/services/storage/preferences-storage";
import { LanguageSwitcher } from "./LanguageSwitcher";
import { ThemeSwitcher } from "./ThemeSwitcher";

export function PreferencesDialog({ onOpenChange, open }: { onOpenChange: (open: boolean) => void; open: boolean }) {
  const { t } = useTranslation("settings");
  const [networkMonitorStartOnLaunch, setStartOnLaunch] = useState(false);

  useEffect(() => {
    if (!open) {
      return;
    }
    let active = true;
    void getPreferences().then((preferences) => {
      if (active) {
        setStartOnLaunch(preferences.networkMonitorStartOnLaunch ?? false);
      }
    });
    return () => {
      active = false;
    };
  }, [open]);

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
            <FieldGroup>
              <Field orientation="horizontal">
                <FieldContent>
                  <FieldTitle>{t("networkMonitor.startOnLaunch")}</FieldTitle>
                  <FieldDescription>
                    {t("networkMonitor.startOnLaunchDescription")}
                  </FieldDescription>
                </FieldContent>
                <Switch
                  aria-label={t("networkMonitor.startOnLaunch")}
                  checked={networkMonitorStartOnLaunch}
                  onCheckedChange={(checked) => {
                    setStartOnLaunch(checked);
                    void setNetworkMonitorStartOnLaunch(checked);
                  }}
                />
              </Field>
            </FieldGroup>
          </section>
        </div>
      </DialogContent>
    </Dialog>
  );
}
