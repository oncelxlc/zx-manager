import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Area,
  AreaChart,
  CartesianGrid,
  XAxis,
  YAxis,
} from "recharts";

import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Tabs,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
import { resourceMetrics } from "src/data/dashboard-mock-data";
import type { ResourceRange } from "src/types/service";
import { formatChartDate, formatPercent } from "src/i18n/formatters";
import type { SupportedLocale } from "src/types/preferences";

function isResourceRange(value: string): value is ResourceRange {
  return value === "24h" || value === "7d" || value === "30d";
}

export function ResourceChart() {
  const [range, setRange] = useState<ResourceRange>("7d");
  const { i18n, t } = useTranslation("dashboard");
  const locale: SupportedLocale = i18n.language === "en-US" ? "en-US" : "zh-CN";
  const chartConfig = {
    cpu: { label: t("chart.cpu"), color: "var(--chart-1)" },
    memory: { label: t("chart.memory"), color: "var(--chart-2)" },
  } satisfies ChartConfig;
  const ranges: ResourceRange[] = ["24h", "7d", "30d"];

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("chart.title")}</CardTitle>
        <CardDescription>{t("chart.description")}</CardDescription>
        <CardAction>
          <Tabs
            onValueChange={(value) => {
              if (isResourceRange(value)) {
                setRange(value);
              }
            }}
            value={range}
          >
            <TabsList aria-label={t("chart.rangeLabel")}>
              {ranges.map((item) => (
                <TabsTrigger key={item} value={item}>
                  {t(`chart.range.${item}`)}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>
        </CardAction>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="flex items-center gap-5 text-xs text-muted-foreground">
          <span className="flex items-center gap-2">
            <span className="size-2 rounded-full bg-chart-1" />
            {t("chart.cpu")}
          </span>
          <span className="flex items-center gap-2">
            <span className="size-2 rounded-full bg-chart-2" />
            {t("chart.memory")}
          </span>
        </div>
        <ChartContainer
          className="h-[260px] w-full aspect-auto"
          config={chartConfig}
          initialDimension={{ width: 900, height: 260 }}
        >
          <AreaChart
            accessibilityLayer
            data={resourceMetrics[range]}
            margin={{ left: 0, right: 12, top: 8, bottom: 0 }}
          >
            <defs>
              <linearGradient id="cpu-fill" x1="0" x2="0" y1="0" y2="1">
                <stop
                  offset="5%"
                  stopColor="var(--color-cpu)"
                  stopOpacity={0.34}
                />
                <stop
                  offset="95%"
                  stopColor="var(--color-cpu)"
                  stopOpacity={0.02}
                />
              </linearGradient>
              <linearGradient id="memory-fill" x1="0" x2="0" y1="0" y2="1">
                <stop
                  offset="5%"
                  stopColor="var(--color-memory)"
                  stopOpacity={0.26}
                />
                <stop
                  offset="95%"
                  stopColor="var(--color-memory)"
                  stopOpacity={0.02}
                />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" vertical={false} />
            <XAxis
              axisLine={false}
              dataKey="date"
              tickLine={false}
              tickMargin={10}
              tickFormatter={(value: string) => formatChartDate(value, locale, range)}
            />
            <YAxis
              axisLine={false}
              domain={[0, 100]}
              tickFormatter={(value: number) => formatPercent(value, locale)}
              tickLine={false}
              width={42}
            />
            <ChartTooltip
              content={
                <ChartTooltipContent
                  formatter={(value, name) => (
                    <div className="flex min-w-32 items-center justify-between gap-4">
                      <span className="text-muted-foreground">
                        {name === "cpu" ? t("chart.cpu") : t("chart.memory")}
                      </span>
                      <span className="font-mono font-medium">{formatPercent(Number(value), locale)}</span>
                    </div>
                  )}
                  labelFormatter={(_, payload) =>
                    formatChartDate(String(payload[0]?.payload?.date ?? ""), locale, range)
                  }
                />
              }
              cursor={false}
            />
            <Area
              dataKey="memory"
              fill="url(#memory-fill)"
              fillOpacity={1}
              stroke="var(--color-memory)"
              strokeWidth={2}
              type="monotone"
            />
            <Area
              dataKey="cpu"
              fill="url(#cpu-fill)"
              fillOpacity={1}
              stroke="var(--color-cpu)"
              strokeWidth={2}
              type="monotone"
            />
          </AreaChart>
        </ChartContainer>
      </CardContent>
    </Card>
  );
}
