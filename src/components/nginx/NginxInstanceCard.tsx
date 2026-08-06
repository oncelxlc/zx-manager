import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  EllipsisIcon,
  PlayIcon,
  RefreshCwIcon,
  RotateCwIcon,
  SquareIcon,
  Trash2Icon,
} from "lucide-react";

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
import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Spinner } from "@/components/ui/spinner";
import { NginxRuntimeBadge } from "src/components/nginx/NginxRuntimeBadge";
import type {
  NginxControlAction,
  NginxInstance,
  NginxOperationPhase,
} from "src/types/nginx";

interface NginxInstanceCardProps {
  instance: NginxInstance;
  observedAt: string | null;
  operationPhase: NginxOperationPhase | null;
  busy: boolean;
  onControl: (action: NginxControlAction) => void;
  onRefresh: () => void;
  onUnregister: () => void;
}

export function NginxInstanceCard({
  instance,
  observedAt,
  operationPhase,
  busy,
  onControl,
  onRefresh,
  onUnregister,
}: NginxInstanceCardProps) {
  const { i18n, t } = useTranslation("nginx");
  const [confirmRemoval, setConfirmRemoval] = useState(false);
  const canStart = instance.capabilities.canControl
    && instance.runtimeStatus === "stopped";
  const canStop = instance.capabilities.canControl
    && instance.runtimeStatus === "running";
  const observed = observedAt
    ? new Intl.DateTimeFormat(i18n.resolvedLanguage ?? i18n.language, {
      dateStyle: "medium",
      timeStyle: "medium",
    }).format(new Date(observedAt))
    : t("instances.notObserved");

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle className="flex flex-wrap items-center gap-2">
            {instance.name}
            <NginxRuntimeBadge status={instance.runtimeStatus} />
          </CardTitle>
          <CardDescription className="break-all">{instance.rootPath}</CardDescription>
          <CardAction className="flex items-center gap-2">
            <Button disabled={busy} onClick={onRefresh} size="icon-sm" variant="ghost">
              <RefreshCwIcon />
              <span className="sr-only">{t("instances.refreshOne", { name: instance.name })}</span>
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger
                disabled={busy || instance.runtimeStatus !== "running"}
                render={<Button size="icon-sm" variant="outline" />}
              >
                <EllipsisIcon />
                <span className="sr-only">{t("instances.moreActions")}</span>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={() => onControl("reload")}>
                  <RotateCwIcon />{t("control.reloadLabel")}
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => onControl("restart")}>
                  <RefreshCwIcon />{t("control.restartLabel")}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </CardAction>
        </CardHeader>
        <CardContent className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          {[
            [t("instances.columns.version"), instance.version],
            [t("instances.columns.lifecycle"), t(`lifecycle.${instance.lifecycleState}`)],
            [t("instances.columns.authorization"), t(`authorization.${instance.authorizationLevel}`)],
            [t("instances.observedAt"), observed],
          ].map(([label, value]) => (
            <div className="rounded-lg border p-3" key={label}>
              <p className="text-xs text-muted-foreground">{label}</p>
              <p className="mt-1 font-medium">{value}</p>
            </div>
          ))}
          {operationPhase ? (
            <p className="flex items-center gap-2 text-sm text-muted-foreground sm:col-span-2 lg:col-span-4">
              <Spinner />{t(`operationPhase.${operationPhase}`)}
            </p>
          ) : null}
        </CardContent>
        <CardFooter className="flex flex-wrap justify-between gap-3">
          <Button
            disabled={busy || (!canStart && !canStop)}
            onClick={() => onControl(canStop ? "stop" : "start")}
          >
            {busy ? <Spinner data-icon="inline-start" /> : canStop
              ? <SquareIcon data-icon="inline-start" />
              : <PlayIcon data-icon="inline-start" />}
            {t(canStop ? "control.stopLabel" : "control.startLabel")}
          </Button>
          <Button onClick={() => setConfirmRemoval(true)} variant="destructive">
            <Trash2Icon data-icon="inline-start" />{t("unregister.action")}
          </Button>
        </CardFooter>
      </Card>

      <AlertDialog open={confirmRemoval} onOpenChange={setConfirmRemoval}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("unregister.title")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("unregister.description", { name: instance.name })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common:actions.cancel")}</AlertDialogCancel>
            <AlertDialogAction onClick={onUnregister} variant="destructive">
              {t("unregister.action")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
