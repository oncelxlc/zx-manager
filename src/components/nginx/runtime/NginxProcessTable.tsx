import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import type { NginxRuntimeProcess } from "src/types/nginx";

interface NginxProcessTableProps {
  processes: NginxRuntimeProcess[];
}

export function NginxProcessTable({ processes }: NginxProcessTableProps) {
  const { t } = useTranslation("nginx");
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("runtimeDetails.processes")}</CardTitle>
        <CardDescription>{t("runtimeDetails.processesDescription")}</CardDescription>
      </CardHeader>
      <CardContent className="overflow-x-auto">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("runtimeDetails.role")}</TableHead>
              <TableHead>PID</TableHead>
              <TableHead>PPID</TableHead>
              <TableHead>CPU</TableHead>
              <TableHead>{t("runtimeDetails.memory")}</TableHead>
              <TableHead>{t("runtimeDetails.identity")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {processes.map((process) => (
              <TableRow key={process.pid}>
                <TableCell>{t(`runtimeDetails.roles.${process.role}`)}</TableCell>
                <TableCell className="font-mono">{process.pid}</TableCell>
                <TableCell className="font-mono">{process.parentPid ?? "—"}</TableCell>
                <TableCell>{process.cpuUsage.toFixed(1)}%</TableCell>
                <TableCell>{(process.memoryBytes / 1024 ** 2).toFixed(1)} MiB</TableCell>
                <TableCell>
                  <Badge variant={process.executableVerified ? "success" : "destructive"}>
                    {t(process.executableVerified
                      ? "runtimeDetails.verified"
                      : "runtimeDetails.unverified")}
                  </Badge>
                </TableCell>
              </TableRow>
            ))}
            {processes.length === 0 ? (
              <TableRow>
                <TableCell className="text-center text-muted-foreground" colSpan={6}>
                  {t("runtimeDetails.noProcesses")}
                </TableCell>
              </TableRow>
            ) : null}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}
