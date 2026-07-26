import { useState } from "react";
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
} from "@/types/service";

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
              aria-label={`Open actions for ${service.name}`}
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
              disabled={pending || service.status === "running"}
              onClick={() => void onOperation(service, "start")}
            >
              <PlayIcon />
              Start
            </DropdownMenuItem>
            <DropdownMenuItem
              disabled={pending || service.status === "stopped"}
              onClick={() => setConfirmationAction("stop")}
            >
              <SquareIcon />
              Stop
            </DropdownMenuItem>
            <DropdownMenuItem
              disabled={pending || service.status === "stopped"}
              onClick={() => void onOperation(service, "restart")}
            >
              <RefreshCwIcon />
              Restart
            </DropdownMenuItem>
          </DropdownMenuGroup>
          <DropdownMenuSeparator />
          <DropdownMenuGroup>
            <DropdownMenuItem
              onClick={() => onLocalAction("Open Logs", service)}
            >
              <FileTextIcon />
              Open Logs
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => onLocalAction("Edit Configuration", service)}
            >
              <FileCodeIcon />
              Edit Configuration
            </DropdownMenuItem>
          </DropdownMenuGroup>
          <DropdownMenuSeparator />
          <DropdownMenuGroup>
            <DropdownMenuItem
              onClick={() => setConfirmationAction("remove")}
              variant="destructive"
            >
              <Trash2Icon />
              Remove Service
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
              {removing ? "Remove service?" : "Stop service?"}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {removing
                ? `${service.name} will be removed from this local dashboard. This does not delete its files.`
                : `${service.name} will stop accepting local traffic until it is started again.`}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={confirming}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              disabled={confirming}
              onClick={() => void handleConfirm()}
              variant="destructive"
            >
              {confirming ? <Spinner data-icon="inline-start" /> : null}
              {removing ? "Remove" : "Stop"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
