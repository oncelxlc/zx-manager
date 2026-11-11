import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import type { NginxConfigNodeDetail } from "src/types/nginx";
import type { NginxConfigCustomGroup } from "src/types/preferences";

interface Props {
  detail: NginxConfigNodeDetail | null;
  groupId: string | null;
  groups: NginxConfigCustomGroup[];
  onGroupChange: (groupId: string | null) => void;
  onOpenChange: (open: boolean) => void;
}

export function NginxConfigNodeSheet(props: Props) {
  const { t } = useTranslation("nginx");
  const items = [
    { value: "none", label: t("configuration.noGroup") },
    ...props.groups.map((group) => ({ value: group.id, label: group.name })),
  ];
  return (
    <Sheet open={Boolean(props.detail)} onOpenChange={props.onOpenChange}>
      <SheetContent className="sm:max-w-xl">
        <SheetHeader>
          <SheetTitle>{props.detail?.node.label ?? t("configuration.details")}</SheetTitle>
          <SheetDescription>{t("configuration.nodeDescription")}</SheetDescription>
        </SheetHeader>
        {props.detail ? (
          <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-auto px-4 pb-4">
            <div className="flex flex-wrap gap-2">
              <Badge>{props.detail.node.kind}</Badge>
              <Badge variant="outline">
                {t("configuration.line", { line: props.detail.node.location.line })}
              </Badge>
            </div>
            <Select
              items={items}
              onValueChange={(value) => props.onGroupChange(value === "none" ? null : value)}
              value={props.groupId ?? "none"}
            >
              <SelectTrigger className="w-full" aria-label={t("configuration.customGroupName")}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent><SelectGroup>{items.map((item) => (
                <SelectItem key={item.value} value={item.value}>{item.label}</SelectItem>
              ))}</SelectGroup></SelectContent>
            </Select>
            <div>
              <p className="mb-2 text-sm font-medium">{t("configuration.includeChain")}</p>
              <code className="block break-all rounded-lg bg-muted p-3 text-xs">
                {props.detail.node.includeChain.join(" → ") || t("configuration.entryFile")}
              </code>
            </div>
            <pre className="overflow-auto rounded-lg bg-muted p-3 text-xs"><code>{props.detail.raw}</code></pre>
          </div>
        ) : null}
      </SheetContent>
    </Sheet>
  );
}
