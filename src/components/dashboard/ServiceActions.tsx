import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  FileCodeIcon,
  FileTextIcon,
  MoreHorizontalIcon,
  PlayIcon,
  RefreshCwIcon,
  SquareIcon,
  Trash2Icon,
  TriangleAlertIcon,
} from "lucide-react";

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogMedia,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Spinner } from "@/components/ui/spinner";
import type {
  LocalService,
  ServiceOperation,
} from "src/types/service";

type ConfirmationAction = "stop" | "remove";

interface ServiceActionsProps {
  service: LocalService;
  pending: boolean;
  onOperation: (
    service: LocalService,
    operation: ServiceOperation,
  ) => Promise<void>;
  onRemove: (service: LocalService) => void;
  onLocalAction: (title: string, service: LocalService) => void;
}

export function ServiceActions({
  service,
  pending,
  onOperation,
  onRemove,
  onLocalAction,
}: ServiceActionsProps) {
  const { t } = useTranslation(["services", "common"]);
  const [confirmationAction, setConfirmationAction] =
    useState<ConfirmationAction | null>(null);
  const [confirming, setConfirming] = useState(false);

  async function handleConfirm() {
    if (!confirmationAction) {
      return;
    }

    setConfirming(true);
    if (confirmationAction === "stop") {
      await onOperation(service, "stop");
    } else {
      onRemove(service);
    }
    setConfirming(false);
    setConfirmationAction(null);
  }

  const removing = confirmationAction === "remove";

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <Button
              aria-label={t("table.actionsFor", { name: service.name })}
              disabled={pending}
              size="icon-sm"
              variant="ghost"
            />
          }
        >
          {pending ? <Spinner /> : <MoreHorizontalIcon />}
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-48">
          <DropdownMenuGroup>
            <DropdownMenuItem
              disabled={pending || (service.type === "nginx"
                ? service.status !== "stopped"
                : service.status === "running")}
              onClick={() => void onOperation(service, "start")}
            >
              <PlayIcon />
              {t("actions.start")}
            </DropdownMenuItem>
            <DropdownMenuItem
              disabled={pending || service.status === "stopped"}
              onClick={() => setConfirmationAction("stop")}
            >
              <SquareIcon />
              {t("actions.stop")}
            </DropdownMenuItem>
            <DropdownMenuItem
              disabled={pending || service.status === "stopped"}
              onClick={() => void onOperation(service, "restart")}
            >
              <RefreshCwIcon />
              {t("actions.restart")}
            </DropdownMenuItem>
          </DropdownMenuGroup>
          <DropdownMenuSeparator />
          <DropdownMenuGroup>
            <DropdownMenuItem
              onClick={() => onLocalAction(t("actions.openLogs"), service)}
            >
              <FileTextIcon />
              {t("actions.openLogs")}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => onLocalAction(t("actions.editConfiguration"), service)}
            >
              <FileCodeIcon />
              {t("actions.editConfiguration")}
            </DropdownMenuItem>
          </DropdownMenuGroup>
          <DropdownMenuSeparator />
          <DropdownMenuGroup>
            <DropdownMenuItem
              onClick={() => setConfirmationAction("remove")}
              variant="destructive"
            >
              <Trash2Icon />
              {t("actions.remove")}
            </DropdownMenuItem>
          </DropdownMenuGroup>
        </DropdownMenuContent>
      </DropdownMenu>

      <AlertDialog
        onOpenChange={(open) => {
          if (!open && !confirming) {
            setConfirmationAction(null);
          }
        }}
        open={confirmationAction !== null}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogMedia>
              <TriangleAlertIcon aria-hidden="true" />
            </AlertDialogMedia>
            <AlertDialogTitle>
              {removing ? t("confirmation.removeTitle") : t("confirmation.stopTitle")}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {removing
                ? t("confirmation.removeDescription", { name: service.name })
                : t("confirmation.stopDescription", { name: service.name })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={confirming}>{t("common:actions.cancel")}</AlertDialogCancel>
            <AlertDialogAction
              disabled={confirming}
              onClick={() => void handleConfirm()}
              variant="destructive"
            >
              {confirming ? <Spinner data-icon="inline-start" /> : null}
              {removing ? t("actions.remove") : t("actions.stop")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
