import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import {
  Field,
  FieldContent,
  FieldDescription,
  FieldTitle,
} from "@/components/ui/field";
import { Switch } from "@/components/ui/switch";
import {
  getPreferences,
  setNetworkMonitorStartOnLaunch,
} from "src/services/storage/preferences-storage";

export function NetworkMonitorLaunchSwitcher() {
  const { t } = useTranslation("settings");
  const [startOnLaunch, setStartOnLaunch] = useState(true);

  useEffect(() => {
    let active = true;
    void getPreferences().then((preferences) => {
      if (active) {
        setStartOnLaunch(preferences.networkMonitorStartOnLaunch ?? true);
      }
    });
    return () => {
      active = false;
    };
  }, []);

  function handleCheckedChange(checked: boolean) {
    setStartOnLaunch(checked);
    void setNetworkMonitorStartOnLaunch(checked);
  }

  return (
    <Field orientation="horizontal">
      <FieldContent>
        <FieldTitle>{t("networkMonitor.startOnLaunch")}</FieldTitle>
        <FieldDescription>
          {t("networkMonitor.startOnLaunchDescription")}
        </FieldDescription>
      </FieldContent>
      <Switch
        aria-label={t("networkMonitor.startOnLaunch")}
        checked={startOnLaunch}
        onCheckedChange={handleCheckedChange}
      />
    </Field>
  );
}
