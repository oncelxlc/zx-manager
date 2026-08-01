import { HardDriveIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import type { SystemInformation } from "src/types/system-information";
import { formatBytes } from "src/utils/system-information";

export function SystemStorageSection({ information, locale, unavailable }: {
  information: SystemInformation;
  locale: string;
  unavailable: string;
}) {
  const { t } = useTranslation("systemInformation");
  return (
    <Card>
      <CardHeader><div className="flex items-start gap-3"><div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground"><HardDriveIcon aria-hidden="true" /></div><div><CardTitle>{t("sections.storage.title")}</CardTitle><CardDescription>{t("sections.storage.description")}</CardDescription></div></div></CardHeader>
      <CardContent>{information.disks.length === 0 ? (
        <Empty className="border"><EmptyHeader><EmptyMedia variant="icon"><HardDriveIcon /></EmptyMedia><EmptyTitle>{t("empty.storageTitle")}</EmptyTitle><EmptyDescription>{t("empty.storageDescription")}</EmptyDescription></EmptyHeader></Empty>
      ) : <Table><TableHeader><TableRow>{["disk", "mountPoint", "fileSystem", "kind", "used", "available", "total"].map((field) => <TableHead key={field}>{t(`fields.${field}`)}</TableHead>)}</TableRow></TableHeader><TableBody>{information.disks.map((disk, index) => <TableRow key={`${disk.mountPoint ?? disk.name}-${index}`}><TableCell className="font-medium">{disk.name ?? unavailable}</TableCell><TableCell>{disk.mountPoint ?? unavailable}</TableCell><TableCell>{disk.fileSystem ?? unavailable}</TableCell><TableCell>{disk.kind}</TableCell><TableCell>{formatBytes(disk.usedBytes, locale, unavailable)}</TableCell><TableCell>{formatBytes(disk.availableBytes, locale, unavailable)}</TableCell><TableCell>{formatBytes(disk.totalBytes, locale, unavailable)}</TableCell></TableRow>)}</TableBody></Table>}</CardContent>
    </Card>
  );
}
