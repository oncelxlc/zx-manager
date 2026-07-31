import { useState } from "react";
import { useTranslation } from "react-i18next";
import { RefreshCwIcon, Trash2Icon } from "lucide-react";

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { NginxInstance } from "src/types/nginx";

interface NginxInstanceTableProps {
  instances: NginxInstance[];
  onRefresh: (instanceId: string) => void;
  onUnregister: (instanceId: string) => void;
}

function lifecycleVariant(state: NginxInstance["lifecycleState"]) {
  return state === "available" ? "success" : "warning";
}

export function NginxInstanceTable({
  instances,
  onRefresh,
  onUnregister,
}: NginxInstanceTableProps) {
  const { t } = useTranslation("nginx");
  const [pendingRemoval, setPendingRemoval] = useState<NginxInstance | null>(null);

  return (
    <>
      <Table containerClassName="rounded-lg border">
        <TableHeader>
          <TableRow>
            <TableHead>{t("instances.columns.name")}</TableHead>
            <TableHead>{t("instances.columns.version")}</TableHead>
            <TableHead>{t("instances.columns.lifecycle")}</TableHead>
            <TableHead>{t("instances.columns.authorization")}</TableHead>
            <TableHead>{t("instances.columns.path")}</TableHead>
            <TableHead className="text-right">
              {t("instances.columns.actions")}
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {instances.map((instance) => (
            <TableRow key={instance.id}>
              <TableCell className="font-medium">{instance.name}</TableCell>
              <TableCell>{instance.version}</TableCell>
              <TableCell>
                <Badge variant={lifecycleVariant(instance.lifecycleState)}>
                  {t(`lifecycle.${instance.lifecycleState}`)}
                </Badge>
              </TableCell>
              <TableCell>
                {t(`authorization.${instance.authorizationLevel}`)}
              </TableCell>
              <TableCell className="max-w-80 truncate" title={instance.rootPath}>
                {instance.rootPath}
              </TableCell>
              <TableCell>
                <div className="flex justify-end gap-1">
                  <Button
                    aria-label={t("instances.refreshOne", { name: instance.name })}
                    onClick={() => onRefresh(instance.id)}
                    size="icon-sm"
                    variant="ghost"
                  >
                    <RefreshCwIcon />
                  </Button>
                  <Button
                    aria-label={t("instances.unregisterOne", { name: instance.name })}
                    onClick={() => setPendingRemoval(instance)}
                    size="icon-sm"
                    variant="ghost"
                  >
                    <Trash2Icon />
                  </Button>
                </div>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>

      <AlertDialog
        onOpenChange={(open) => !open && setPendingRemoval(null)}
        open={pendingRemoval !== null}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("unregister.title")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("unregister.description", { name: pendingRemoval?.name })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common:actions.cancel")}</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                if (pendingRemoval) {
                  onUnregister(pendingRemoval.id);
                }
                setPendingRemoval(null);
              }}
              variant="destructive"
            >
              {t("unregister.action")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
