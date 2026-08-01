import { useTranslation } from "react-i18next";

import { Field, FieldDescription, FieldLabel } from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { NginxInstance } from "src/types/nginx";

interface NginxConfigurationPickerProps {
  instances: NginxInstance[];
  value: string | null;
  onChange: (instanceId: string) => void;
}

export function NginxConfigurationPicker({
  instances,
  value,
  onChange,
}: NginxConfigurationPickerProps) {
  const { t } = useTranslation("nginx");
  const readable = instances.filter((instance) => instance.capabilities.canRead);
  const items = readable.map((instance) => ({
    label: instance.name,
    value: instance.id,
  }));

  return (
    <Field className="max-w-xl">
      <FieldLabel>{t("configuration.instance")}</FieldLabel>
      <Select
        items={items}
        onValueChange={(next) => next && onChange(next)}
        value={value}
      >
        <SelectTrigger className="w-full">
          <SelectValue placeholder={t("configuration.selectInstance")} />
        </SelectTrigger>
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
      <FieldDescription>{t("configuration.instanceDescription")}</FieldDescription>
    </Field>
  );
}
