import { useState } from "react";
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

const chartConfig = {
  cpu: {
    label: "CPU Usage",
    color: "var(--chart-1)",
  },
  memory: {
    label: "Memory Usage",
    color: "var(--chart-2)",
  },
} satisfies ChartConfig;

const ranges: Array<{ value: ResourceRange; label: string }> = [
  { value: "24h", label: "Last 24 hours" },
  { value: "7d", label: "Last 7 days" },
  { value: "30d", label: "Last 30 days" },
];

function isResourceRange(value: string): value is ResourceRange {
  return value === "24h" || value === "7d" || value === "30d";
}

export function ResourceChart() {
  const [range, setRange] = useState<ResourceRange>("7d");

  return (
    <Card>
      <CardHeader>
        <CardTitle>System Resource Usage</CardTitle>
        <CardDescription>
          CPU and memory usage during the selected period
        </CardDescription>
        <CardAction>
          <Tabs
            onValueChange={(value) => {
              if (isResourceRange(value)) {
                setRange(value);
              }
            }}
            value={range}
          >
            <TabsList aria-label="Resource chart time range">
              {ranges.map((item) => (
                <TabsTrigger key={item.value} value={item.value}>
                  {item.label}
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
            CPU Usage
          </span>
          <span className="flex items-center gap-2">
            <span className="size-2 rounded-full bg-chart-2" />
            Memory Usage
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
            />
            <YAxis
              axisLine={false}
              domain={[0, 100]}
              tickFormatter={(value: number) => `${value}%`}
              tickLine={false}
              width={42}
            />
            <ChartTooltip
              content={
                <ChartTooltipContent
                  formatter={(value, name) => (
                    <div className="flex min-w-32 items-center justify-between gap-4">
                      <span className="text-muted-foreground">
                        {name === "cpu" ? "CPU" : "Memory"}
                      </span>
                      <span className="font-mono font-medium">{`${value}%`}</span>
                    </div>
                  )}
                  labelFormatter={(_, payload) =>
                    String(payload[0]?.payload?.date ?? "")
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
