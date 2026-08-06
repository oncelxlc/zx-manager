import { useTranslation } from "react-i18next";
import { ShieldCheckIcon } from "lucide-react";

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
import { Progress, ProgressLabel, ProgressValue } from "@/components/ui/progress";
import { Spinner } from "@/components/ui/spinner";
import type { NginxUpgradeProgress, NginxUpgradeResult } from "src/types/nginx";

interface NginxUpgradeDialogProps {
  cacheSource: string;
  checkedAt: string;
  currentVersion: string;
  targetVersion: string;
  open: boolean;
  running: boolean;
  progress: NginxUpgradeProgress | null;
  result: NginxUpgradeResult | null;
  onConfirm: () => void;
  onOpenChange: (open: boolean) => void;
  supportReason: string;
}

export function NginxUpgradeDialog({
  cacheSource,
  checkedAt,
  currentVersion,
  targetVersion,
  open,
  running,
  progress,
  result,
  onConfirm,
  onOpenChange,
  supportReason,
}: NginxUpgradeDialogProps) {
  const { t } = useTranslation("nginx");
  return (
    <AlertDialog open={open} onOpenChange={(next) => !running && onOpenChange(next)}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t("upgrade.confirmTitle")}</AlertDialogTitle>
          <AlertDialogDescription>
            {t("upgrade.confirmDescription", { currentVersion, targetVersion })}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <div aria-live="polite" className="flex flex-col gap-3 text-sm">
          <dl className="grid gap-2 rounded-lg border p-3 sm:grid-cols-2">
            <div><dt className="text-muted-foreground">{t("updates.checkedAt")}</dt><dd>{checkedAt}</dd></div>
            <div><dt className="text-muted-foreground">{t("upgrade.support")}</dt><dd>{supportReason}</dd></div>
            <div><dt className="text-muted-foreground">{t("updates.source")}</dt><dd>{cacheSource}</dd></div>
            <div><dt className="text-muted-foreground">{t("updates.latest")}</dt><dd>{targetVersion}</dd></div>
          </dl>
          <p className="flex items-center gap-2 rounded-lg border p-3">
            <ShieldCheckIcon className="text-success" />
            {t("upgrade.safetySummary")}
          </p>
          {progress ? (
            <Progress value={progress.progress}>
              <ProgressLabel>{t(progress.messageCode)}</ProgressLabel>
              <ProgressValue>
                {(_formatted, value) => `${value ?? 0}%`}
              </ProgressValue>
            </Progress>
          ) : null}
          {result && !result.success ? (
            <p className={result.rollbackSucceeded === false ? "text-destructive" : "text-warning"}>
              {result.rollbackSucceeded === false
                ? t("upgrade.rollbackFailed", { backupId: result.backupId })
                : result.rolledBack
                  ? t("upgrade.rolledBack")
                  : t(`errors.${result.errorCode ?? "NGINX_UNKNOWN"}`)}
            </p>
          ) : null}
        </div>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={running}>{t("common:actions.cancel")}</AlertDialogCancel>
          <AlertDialogAction disabled={running} onClick={onConfirm}>
            {running ? <Spinner data-icon="inline-start" /> : null}
            {t("upgrade.confirmAction")}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
