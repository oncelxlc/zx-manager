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
  currentVersion: string;
  targetVersion: string;
  open: boolean;
  running: boolean;
  progress: NginxUpgradeProgress | null;
  result: NginxUpgradeResult | null;
  onConfirm: () => void;
  onOpenChange: (open: boolean) => void;
}

export function NginxUpgradeDialog({
  currentVersion,
  targetVersion,
  open,
  running,
  progress,
  result,
  onConfirm,
  onOpenChange,
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
        <div className="space-y-3 text-sm">
          <p className="flex items-center gap-2 rounded-lg border p-3">
            <ShieldCheckIcon className="size-4 text-success" />
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
