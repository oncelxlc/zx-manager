import { PlusIcon, ShieldCheckIcon } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { NginxConfigCustomGroup } from "src/types/preferences";

interface Props {
  groupFilter: string;
  groups: NginxConfigCustomGroup[];
  kindFilter: string;
  onAddGroup: (name: string) => void;
  onGroupFilterChange: (value: string) => void;
  onKindFilterChange: (value: string) => void;
  onSearchChange: (value: string) => void;
  onValidate: () => void;
  search: string;
}

export function NginxConfigGraphToolbar(props: Props) {
  const { t } = useTranslation("nginx");
  const [groupName, setGroupName] = useState("");
  const kinds = ["all", "block", "directive", "unknown"];
  const groupItems = [
    { value: "all", label: t("configuration.filters.allGroups") },
    { value: "unassigned", label: t("configuration.filters.unassigned") },
    ...props.groups.map((group) => ({ value: group.id, label: group.name })),
  ];
  return (
    <div className="grid gap-3 rounded-xl border p-3 lg:grid-cols-[minmax(12rem,1fr)_11rem_12rem_auto]">
      <Input
        aria-label={t("configuration.search")}
        onChange={(event) => props.onSearchChange(event.target.value)}
        placeholder={t("configuration.searchPlaceholder")}
        value={props.search}
      />
      <Select
        items={kinds.map((value) => ({ value, label: t(`configuration.filters.${value}`) }))}
        onValueChange={(value) => props.onKindFilterChange(value ?? "all")}
        value={props.kindFilter}
      >
        <SelectTrigger className="w-full" aria-label={t("configuration.kindFilter")}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent><SelectGroup>{kinds.map((value) => (
          <SelectItem key={value} value={value}>{t(`configuration.filters.${value}`)}</SelectItem>
        ))}</SelectGroup></SelectContent>
      </Select>
      <Select
        items={groupItems}
        onValueChange={(value) => props.onGroupFilterChange(value ?? "all")}
        value={props.groupFilter}
      >
        <SelectTrigger className="w-full" aria-label={t("configuration.groupFilter")}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent><SelectGroup>{groupItems.map((item) => (
          <SelectItem key={item.value} value={item.value}>{item.label}</SelectItem>
        ))}</SelectGroup></SelectContent>
      </Select>
      <Button onClick={props.onValidate} variant="outline">
        <ShieldCheckIcon data-icon="inline-start" />{t("configuration.validate")}
      </Button>
      <div className="flex gap-2 lg:col-span-4">
        <Input
          aria-label={t("configuration.customGroupName")}
          onChange={(event) => setGroupName(event.target.value)}
          placeholder={t("configuration.customGroupPlaceholder")}
          value={groupName}
        />
        <Button
          disabled={!groupName.trim()}
          onClick={() => { props.onAddGroup(groupName); setGroupName(""); }}
          variant="secondary"
        >
          <PlusIcon data-icon="inline-start" />{t("configuration.addGroup")}
        </Button>
      </div>
    </div>
  );
}
