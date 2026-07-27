import type { ComponentProps } from "react";
import type { LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";

interface MetricCardProps {
  title: string;
  value: string;
  badge: string;
  badgeVariant?: ComponentProps<typeof Badge>["variant"];
  description: string;
  detail: string;
  icon: LucideIcon;
  progress?: number;
  healthy?: boolean;
}

export function MetricCard({
  title,
  value,
  badge,
  badgeVariant = "secondary",
  description,
  detail,
  icon: Icon,
  progress,
  healthy = false,
}: MetricCardProps) {
  const { t } = useTranslation("common");

  return (
    <Card className="min-h-40 transition-shadow hover:ring-foreground/20">
      <CardHeader>
        <div className="flex items-center gap-2">
          <div className="flex size-8 items-center justify-center rounded-lg bg-muted text-muted-foreground">
            <Icon className="size-4" aria-hidden="true" />
          </div>
          <CardTitle className="text-xs font-medium text-muted-foreground">
            {title}
          </CardTitle>
        </div>
        <CardAction>
          <Badge variant={badgeVariant}>{badge}</Badge>
        </CardAction>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div className="flex items-center gap-2">
          {healthy ? (
            <span
              aria-label={t("status.healthy")}
              className="size-2 rounded-full bg-success"
            />
          ) : null}
          <p className="text-3xl font-semibold tracking-tight">{value}</p>
        </div>
        {typeof progress === "number" ? (
          <Progress aria-label={`${title}: ${progress}%`} value={progress} />
        ) : null}
        <div className="flex flex-col gap-0.5">
          <CardDescription className="text-xs text-foreground">
            {description}
          </CardDescription>
          <p className="text-xs text-muted-foreground">{detail}</p>
        </div>
      </CardContent>
    </Card>
  );
}
