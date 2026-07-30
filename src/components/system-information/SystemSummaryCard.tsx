import type { ReactNode } from "react";

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
} from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";

interface SystemSummaryCardProps {
  detail: string;
  icon: ReactNode;
  label: string;
  progress?: number | null;
  value: string;
}

export function SystemSummaryCard({
  detail,
  icon,
  label,
  progress,
  value,
}: SystemSummaryCardProps) {
  return (
    <Card>
      <CardHeader className="flex-row items-center justify-between">
        <CardDescription>{label}</CardDescription>
        <span className="text-muted-foreground">{icon}</span>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <p className="truncate text-2xl font-semibold tracking-tight">{value}</p>
        {progress !== undefined && progress !== null ? (
          <Progress aria-label={label} value={progress} />
        ) : null}
        <p className="truncate text-xs text-muted-foreground">{detail}</p>
      </CardContent>
    </Card>
  );
}
