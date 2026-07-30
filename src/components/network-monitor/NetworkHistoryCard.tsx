import { ArrowDownIcon, ArrowUpIcon, RefreshCwIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Empty, EmptyHeader, EmptyTitle } from "@/components/ui/empty";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { NetworkTablePagination, type NetworkTablePageSize } from "./NetworkTablePagination";
import type { RangePreset } from "./network-monitor-utils";
import type { NetworkPathFilter, NetworkUsageResult, NetworkUsageSortBy, SortDirection } from "src/types/network-monitor";
import { formatDataSize } from "src/utils/format-data-size";

interface NetworkHistoryCardProps {
  applicationDisplayName: (id: string, name: string) => string;
  customFrom: string;
  customTo: string;
  history: NetworkUsageResult | null;
  loading: boolean;
  pageIndex: number;
  pageSize: NetworkTablePageSize;
  path: NetworkPathFilter;
  range: RangePreset;
  sortBy: NetworkUsageSortBy;
  sortDirection: SortDirection;
  locale: string;
  onCustomFromChange: (value: string) => void;
  onCustomToChange: (value: string) => void;
  onPageChange: (page: number) => void;
  onPageSizeChange: (size: NetworkTablePageSize) => void;
  onPathChange: (path: NetworkPathFilter) => void;
  onQuery: () => void;
  onRangeChange: (range: RangePreset) => void;
  onSort: (sortBy: NetworkUsageSortBy) => void;
}

function SortIcon({active, direction}: {active: boolean; direction: SortDirection}) {
  return active ? direction === "asc"
    ? <ArrowUpIcon data-icon="inline-end" />
    : <ArrowDownIcon data-icon="inline-end" />
    : null;
}

export function NetworkHistoryCard(props: NetworkHistoryCardProps) {
  const {t} = useTranslation("networkMonitor");
  const columns: {key: NetworkUsageSortBy; label: string; className?: string}[] = [
    {key: "application", label: t("history.columns.application")},
    {key: "download", label: t("history.columns.download"), className: "w-40 text-right"},
    {key: "upload", label: t("history.columns.upload"), className: "w-40 text-right"},
    {key: "total", label: t("history.columns.total"), className: "w-40 text-right"},
  ];

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("history.title")}</CardTitle>
        <CardDescription>{t("history.description")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-5">
        <FieldGroup>
          <Field>
            <FieldLabel>{t("history.range")}</FieldLabel>
            <ToggleGroup onValueChange={(values) => {
              const value = values[0] as RangePreset | undefined;
              if (value) props.onRangeChange(value);
            }} value={[props.range]} variant="outline">
              {(["10m", "1h", "24h", "7d", "custom"] as const).map((value) => (
                <ToggleGroupItem key={value} value={value}>{t(`history.ranges.${value}`)}</ToggleGroupItem>
              ))}
            </ToggleGroup>
          </Field>
          {props.range === "custom" ? <FieldGroup className="grid md:grid-cols-2">
            <Field><FieldLabel htmlFor="network-history-from">{t("history.from")}</FieldLabel>
              <Input id="network-history-from" onChange={(event) => props.onCustomFromChange(event.target.value)} type="datetime-local" value={props.customFrom} />
            </Field>
            <Field><FieldLabel htmlFor="network-history-to">{t("history.to")}</FieldLabel>
              <Input id="network-history-to" onChange={(event) => props.onCustomToChange(event.target.value)} type="datetime-local" value={props.customTo} />
            </Field>
          </FieldGroup> : null}
          <Field>
            <FieldLabel>{t("pathFilter.history")}</FieldLabel>
            <ToggleGroup onValueChange={(values) => {
              const value = values[0] as NetworkPathFilter | undefined;
              if (value) props.onPathChange(value);
            }} value={[props.path]} variant="outline">
              {(["all", "proxy", "direct"] as const).map((value) => (
                <ToggleGroupItem key={value} value={value}>{t(`pathFilter.options.${value}`)}</ToggleGroupItem>
              ))}
            </ToggleGroup>
          </Field>
        </FieldGroup>
        <div><Button disabled={props.loading} onClick={props.onQuery}>
          {props.loading ? <Spinner data-icon="inline-start" /> : <RefreshCwIcon data-icon="inline-start" />}
          {t("history.query")}
        </Button></div>
        <Table className="min-w-176 table-fixed" containerClassName="max-h-[52.5rem] overflow-auto rounded-md border">
          <TableHeader className="[&_th]:sticky [&_th]:top-0 [&_th]:bg-card"><TableRow>
            {columns.map((column) => {
              const active = props.sortBy === column.key;
              return <TableHead aria-sort={active ? props.sortDirection === "asc" ? "ascending" : "descending" : "none"} className={column.className} key={column.key}>
                <Button onClick={() => props.onSort(column.key)} variant="ghost">{column.label}<SortIcon active={active} direction={props.sortDirection} /></Button>
              </TableHead>;
            })}
          </TableRow></TableHeader>
          <TableBody>
            {(props.history?.points ?? []).map((point) => <TableRow className="h-10" key={point.applicationId}>
              <TableCell className="min-w-0"><div className="flex items-center gap-2">
                <span className="block truncate" title={props.applicationDisplayName(point.applicationId, point.displayName)}>
                  {props.applicationDisplayName(point.applicationId, point.displayName)}
                </span>
                {point.includesUnknown ? <Badge variant="secondary">{t("quality.includesUnknown")}</Badge> : null}
              </div></TableCell>
              <TableCell className="text-right">{formatDataSize(point.downloadBytes, props.locale)}</TableCell>
              <TableCell className="text-right">{formatDataSize(point.uploadBytes, props.locale)}</TableCell>
              <TableCell className="text-right">{formatDataSize(point.totalBytes, props.locale)}</TableCell>
            </TableRow>)}
            {!props.history?.points.length ? <TableRow className="h-100"><TableCell colSpan={4}>
              <Empty className="h-full border-0"><EmptyHeader><EmptyTitle>
                {props.loading ? t("history.loading") : t("history.empty")}
              </EmptyTitle></EmptyHeader></Empty>
            </TableCell></TableRow> : null}
          </TableBody>
        </Table>
        <NetworkTablePagination
          onPageChange={props.onPageChange}
          onPageSizeChange={props.onPageSizeChange}
          pageIndex={props.pageIndex}
          pageSize={props.pageSize}
          totalCount={props.history?.totalCount ?? 0}
        />
      </CardContent>
    </Card>
  );
}
