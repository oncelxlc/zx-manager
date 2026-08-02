import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import {
  Field,
  FieldContent,
  FieldDescription,
  FieldGroup,
  FieldTitle,
} from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import {
  getPreferences,
  setNginxPreferences,
} from "src/services/storage/preferences-storage";
import { useNginxReleaseStore } from "src/stores/nginx-release-store";
import { defaultNginxPreferences } from "src/types/preferences";
import type { NginxPreferences } from "src/types/preferences";

const intervals = [6, 12, 24, 48, 72];
const retentionCounts = [3, 5, 10, 20];
const bufferSizes = [10_000, 20_000, 50_000, 100_000];

export function NginxPreferencesSettings() {
  const { t } = useTranslation("settings");
  const [preferences, setPreferences] = useState(defaultNginxPreferences);

  useEffect(() => {
    let active = true;
    void getPreferences().then((loaded) => {
      if (active && loaded.nginx) {
        setPreferences(loaded.nginx);
      }
    });
    return () => {
      active = false;
    };
  }, []);

  function update(next: NginxPreferences) {
    setPreferences(next);
    useNginxReleaseStore.getState().configure(next);
    void setNginxPreferences(next);
  }

  function numberSelect(
    value: number,
    values: number[],
    label: (value: number) => string,
    onChange: (value: number) => void,
  ) {
    const items = values.map((candidate) => ({
      label: label(candidate),
      value: String(candidate),
    }));
    return (
      <Select
        items={items}
        onValueChange={(next) => next && onChange(Number(next))}
        value={String(value)}
      >
        <SelectTrigger><SelectValue /></SelectTrigger>
        <SelectContent alignItemWithTrigger={false}>
          <SelectGroup>
            {items.map((item) => (
              <SelectItem key={item.value} value={item.value}>
                {item.label}
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
    );
  }

  return (
    <FieldGroup>
      <Field orientation="responsive">
        <FieldContent>
          <FieldTitle>{t("nginx.releaseChannel")}</FieldTitle>
          <FieldDescription>{t("nginx.releaseChannelDescription")}</FieldDescription>
        </FieldContent>
        <Select
          items={[
            { label: t("nginx.channels.stable"), value: "stable" },
            { label: t("nginx.channels.mainline"), value: "mainline" },
          ]}
          onValueChange={(releaseChannel) => {
            if (releaseChannel === "stable" || releaseChannel === "mainline") {
              update({ ...preferences, releaseChannel });
            }
          }}
          value={preferences.releaseChannel}
        >
          <SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent alignItemWithTrigger={false}>
            <SelectGroup>
              <SelectItem value="stable">{t("nginx.channels.stable")}</SelectItem>
              <SelectItem value="mainline">{t("nginx.channels.mainline")}</SelectItem>
            </SelectGroup>
          </SelectContent>
        </Select>
      </Field>
      <Field orientation="responsive">
        <FieldContent><FieldTitle>{t("nginx.checkInterval")}</FieldTitle></FieldContent>
        {numberSelect(
          preferences.updateCheckIntervalHours,
          intervals,
          (value) => t("nginx.hours", { count: value }),
          (updateCheckIntervalHours) => update({ ...preferences, updateCheckIntervalHours }),
        )}
      </Field>
      <Field orientation="responsive">
        <FieldContent><FieldTitle>{t("nginx.backupRetention")}</FieldTitle></FieldContent>
        {numberSelect(
          preferences.backupRetentionCount,
          retentionCounts,
          (value) => t("nginx.backups", { count: value }),
          (backupRetentionCount) => update({ ...preferences, backupRetentionCount }),
        )}
      </Field>
      <Field orientation="responsive">
        <FieldContent><FieldTitle>{t("nginx.logBuffer")}</FieldTitle></FieldContent>
        {numberSelect(
          preferences.logBufferLines,
          bufferSizes,
          (value) => t("nginx.lines", { count: value }),
          (logBufferLines) => update({ ...preferences, logBufferLines }),
        )}
      </Field>
      <Field orientation="horizontal">
        <FieldContent>
          <FieldTitle>{t("nginx.logFollow")}</FieldTitle>
          <FieldDescription>{t("nginx.logFollowDescription")}</FieldDescription>
        </FieldContent>
        <Switch
          aria-label={t("nginx.logFollow")}
          checked={preferences.logFollow}
          onCheckedChange={(logFollow) => update({ ...preferences, logFollow })}
        />
      </Field>
    </FieldGroup>
  );
}
