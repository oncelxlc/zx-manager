import { ArrowDownIcon, ArrowUpIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Empty, EmptyHeader, EmptyTitle } from "@/components/ui/empty";
import { Field, FieldLabel, FieldTitle } from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { NetworkTablePagination, type NetworkTablePageSize } from "./NetworkTablePagination";
import type { AggregatedApplication, ApplicationSortBy } from "./network-monitor-utils";
import type { NetworkPathFilter, NetworkMonitorStatus, SortDirection } from "src/types/network-monitor";
import { formatDataRate, formatDataSize } from "src/utils/format-data-size";

interface NetworkApplicationsCardProps {
  applicationDisplayName: (id: string, name: string) => string;
  applications: AggregatedApplication[];
  pageIndex: number;
  pageSize: NetworkTablePageSize;
  path: NetworkPathFilter;
  sampleIntervalItems: {label: string; value: string}[];
  sampleIntervalLoading: boolean;
  sortBy: ApplicationSortBy;
  sortDirection: SortDirection;
  status: NetworkMonitorStatus | null;
  visibleApplications: AggregatedApplication[];
  locale: string;
  onPageChange: (page: number) => void;
  onPageSizeChange: (size: NetworkTablePageSize) => void;
  onPathChange: (path: NetworkPathFilter) => void;
  onSampleIntervalChange: (value: unknown) => void;
  onSort: (sortBy: ApplicationSortBy) => void;
}

function SortIcon({active, direction}: {active: boolean; direction: SortDirection}) {
  return active ? direction === "asc"
    ? <ArrowUpIcon data-icon="inline-end" />
    : <ArrowDownIcon data-icon="inline-end" />
    : null;
}

export function NetworkApplicationsCard(props: NetworkApplicationsCardProps) {
  const {t} = useTranslation("networkMonitor");
  const columns: {key: ApplicationSortBy; label: string; className?: string}[] = [
    {key: "application", label: t("applications.columns.name")},
    {key: "downloadRate", label: t("applications.columns.downloadRate"), className: "w-40 text-right"},
    {key: "uploadRate", label: t("applications.columns.uploadRate"), className: "w-40 text-right"},
    {key: "download", label: t("applications.columns.sessionDownload"), className: "w-40 text-right"},
    {key: "upload", label: t("applications.columns.sessionUpload"), className: "w-40 text-right"},
  ];

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("applications.title")}</CardTitle>
        <CardDescription>{t("applications.description")}</CardDescription>
        <CardAction>
          <Field data-disabled={props.sampleIntervalLoading ? "" : undefined}>
            <FieldLabel htmlFor="network-sample-interval">
              {t("sampling.label")}
            </FieldLabel>
            <Select
              disabled={props.sampleIntervalLoading}
              items={props.sampleIntervalItems}
              onValueChange={props.onSampleIntervalChange}
              value={String(props.status?.sampleIntervalSeconds ?? 5)}
            >
              <SelectTrigger className="w-32" id="network-sample-interval">
                <SelectValue />
              </SelectTrigger>
              <SelectContent><SelectGroup>
                {props.sampleIntervalItems.map((item) => (
                  <SelectItem key={item.value} value={item.value}>{item.label}</SelectItem>
                ))}
              </SelectGroup></SelectContent>
            </Select>
          </Field>
        </CardAction>
      </CardHeader>
      <CardContent className="flex flex-col gap-5">
        <Field orientation="horizontal">
          <FieldTitle id="network-application-path-label">{t("pathFilter.applications")}</FieldTitle>
          <ToggleGroup
            aria-labelledby="network-application-path-label"
            onValueChange={(values) => {
              const value = values[0] as NetworkPathFilter | undefined;
              if (value) props.onPathChange(value);
            }}
            value={[props.path]}
            variant="outline"
          >
            {(["all", "proxy", "direct"] as const).map((value) => (
              <ToggleGroupItem key={value} value={value}>{t(`pathFilter.options.${value}`)}</ToggleGroupItem>
            ))}
          </ToggleGroup>
        </Field>
        <Table className="min-w-4xl table-fixed" containerClassName="max-h-[52.5rem] overflow-auto rounded-md border">
          <TableHeader className="[&_th]:sticky [&_th]:top-0 [&_th]:bg-card">
            <TableRow>{columns.map((column) => {
              const active = props.sortBy === column.key;
              return <TableHead aria-sort={active ? props.sortDirection === "asc" ? "ascending" : "descending" : "none"} className={column.className} key={column.key}>
                <Button onClick={() => props.onSort(column.key)} variant="ghost">
                  {column.label}<SortIcon active={active} direction={props.sortDirection} />
                </Button>
              </TableHead>;
            })}</TableRow>
          </TableHeader>
          <TableBody>
            {props.visibleApplications.map((application) => (
              <TableRow className="h-10" key={application.applicationId}>
                <TableCell className="min-w-0 font-medium">
                  <div className="flex items-center gap-2">
                    <span className="block truncate" title={props.applicationDisplayName(application.applicationId, application.displayName)}>
                      {props.applicationDisplayName(application.applicationId, application.displayName)}
                    </span>
                    {application.quality === "partial" ? <Badge variant="secondary">{t("quality.partial")}</Badge> : null}
                  </div>
                </TableCell>
                <TableCell className="text-right">{formatDataRate(application.traffic.downloadBytesPerSecond, props.locale)}</TableCell>
                <TableCell className="text-right">{formatDataRate(application.traffic.uploadBytesPerSecond, props.locale)}</TableCell>
                <TableCell className="text-right">{formatDataSize(application.traffic.sessionDownloadBytes, props.locale)}</TableCell>
                <TableCell className="text-right">{formatDataSize(application.traffic.sessionUploadBytes, props.locale)}</TableCell>
              </TableRow>
            ))}
            {props.applications.length === 0 ? <TableRow className="h-100"><TableCell colSpan={5}>
              <Empty className="h-full border-0"><EmptyHeader><EmptyTitle>
                {t(props.status?.enabled ? "applications.waiting" : "applications.disabled")}
              </EmptyTitle></EmptyHeader></Empty>
            </TableCell></TableRow> : null}
          </TableBody>
        </Table>
        <NetworkTablePagination
          onPageChange={props.onPageChange}
          onPageSizeChange={props.onPageSizeChange}
          pageIndex={props.pageIndex}
          pageSize={props.pageSize}
          totalCount={props.applications.length}
        />
      </CardContent>
    </Card>
  );
}
