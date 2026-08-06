import {
  CircleCheckIcon,
  CircleHelpIcon,
  CircleStopIcon,
  TriangleAlertIcon,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import type { NginxRuntimeStatus } from "src/types/nginx";

const statusIcons = {
  conflict: TriangleAlertIcon,
  running: CircleCheckIcon,
  stopped: CircleStopIcon,
  unknown: CircleHelpIcon,
} satisfies Record<NginxRuntimeStatus, typeof CircleCheckIcon>;

function statusVariant(status: NginxRuntimeStatus) {
  if (status === "running") return "success" as const;
  if (status === "conflict") return "destructive" as const;
  if (status === "unknown") return "warning" as const;
  return "secondary" as const;
}

export function NginxRuntimeBadge({ status }: { status: NginxRuntimeStatus }) {
  const { t } = useTranslation("nginx");
  const label = t(`runtime.${status}`);
  const Icon = statusIcons[status];
  return (
    <Badge aria-label={label} variant={statusVariant(status)}>
      <Icon />
      {label}
    </Badge>
  );
}
