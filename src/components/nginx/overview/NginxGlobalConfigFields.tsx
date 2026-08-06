import { useTranslation } from "react-i18next";

import {
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import type {
  NginxGlobalConfigFieldError,
  NginxGlobalConfigurationPatch,
} from "src/types/nginx";

const fields = [
  "workerProcesses",
  "workerRlimitNofile",
  "workerConnections",
  "multiAccept",
  "acceptMutex",
  "acceptMutexDelay",
  "pid",
  "errorLog",
] as const;

export function NginxGlobalConfigFields({
  draft,
  errors,
  onChange,
}: {
  draft: NginxGlobalConfigurationPatch;
  errors: NginxGlobalConfigFieldError[];
  onChange: <K extends keyof NginxGlobalConfigurationPatch>(
    field: K,
    value: NginxGlobalConfigurationPatch[K],
  ) => void;
}) {
  const { t } = useTranslation("nginx");
  return (
    <FieldGroup>
      {fields.map((field) => {
        const error = errors.find((item) => item.field === field);
        return (
          <Field data-invalid={Boolean(error)} key={field}>
            <FieldLabel htmlFor={`nginx-${field}`}>
              {t(`globalConfig.fields.${field}.label`)}
            </FieldLabel>
            <Input
              aria-invalid={Boolean(error)}
              id={`nginx-${field}`}
              onChange={(event) => onChange(field, event.target.value || null)}
              value={draft[field] ?? ""}
            />
            <FieldDescription>{t(`globalConfig.fields.${field}.description`)}</FieldDescription>
            {error ? <FieldError>{t(`globalConfig.errors.${error.code}`)}</FieldError> : null}
          </Field>
        );
      })}
      <Field data-invalid={errors.some((item) => item.field === "topLevelIncludes")}>
        <FieldLabel htmlFor="nginx-top-level-includes">
          {t("globalConfig.fields.topLevelIncludes.label")}
        </FieldLabel>
        <Textarea
          aria-invalid={errors.some((item) => item.field === "topLevelIncludes")}
          id="nginx-top-level-includes"
          onChange={(event) => onChange(
            "topLevelIncludes",
            event.target.value.split("\n").map((value) => value.trim()).filter(Boolean),
          )}
          value={draft.topLevelIncludes.join("\n")}
        />
        <FieldDescription>{t("globalConfig.fields.topLevelIncludes.description")}</FieldDescription>
      </Field>
    </FieldGroup>
  );
}
