import { useTranslation } from "react-i18next";
import { Link } from "react-router";

import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { NginxSite } from "src/types/nginx";

export function NginxSitesTable({
  instanceId,
  sites,
}: {
  instanceId: string;
  sites: NginxSite[];
}) {
  const { t } = useTranslation("nginx");
  return (
    <Table containerClassName="rounded-lg border">
      <TableHeader>
        <TableRow>
          <TableHead>{t("sites.columns.serverName")}</TableHead>
          <TableHead>{t("sites.columns.listen")}</TableHead>
          <TableHead>{t("sites.columns.context")}</TableHead>
          <TableHead>{t("sites.columns.target")}</TableHead>
          <TableHead>{t("sites.columns.source")}</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {sites.map((site) => (
          <TableRow key={site.id}>
            <TableCell>{site.serverNames.join(", ") || "_"}</TableCell>
            <TableCell>{site.listens.join(", ") || "—"}</TableCell>
            <TableCell>{site.context}</TableCell>
            <TableCell className="max-w-96 whitespace-normal">
              <details>
                <summary className="cursor-pointer">
                  {site.proxyPass.join(", ") || site.root || t("sites.details")}
                </summary>
                <dl className="mt-2 grid gap-1 text-xs text-muted-foreground">
                  <div>{t("sites.fields.root")}: {site.root || "—"}</div>
                  <div>{t("sites.fields.proxyPass")}: {site.proxyPass.join(", ") || "—"}</div>
                  <div>{t("sites.fields.locations")}: {site.locations.join(", ") || "—"}</div>
                </dl>
              </details>
            </TableCell>
            <TableCell>
              <Button
                nativeButton={false}
                render={
                  <Link
                    to={`/nginx/manage/configuration?instance=${encodeURIComponent(instanceId)}&source=${encodeURIComponent(site.source.sourceId)}&line=${site.source.line}`}
                  />
                }
                size="sm"
                variant="ghost"
              >
                {t("sites.openSource", { line: site.source.line })}
              </Button>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
