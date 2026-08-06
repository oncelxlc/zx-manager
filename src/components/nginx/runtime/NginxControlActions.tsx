import { useTranslation } from "react-i18next";
import { PlayIcon, RefreshCwIcon, RotateCwIcon, SquareIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { toast } from "@/components/ui/toast";
import { useNginxStore } from "src/stores/nginx-store";
import type { NginxControlAction } from "src/types/nginx";

export function NginxControlActions() {
  const { t } = useTranslation("nginx");
  const instance = useNginxStore((state) => state.instance);
  const busy = useNginxStore((state) => state.operationStatus === "loading");
  const phase = useNginxStore((state) => state.operationPhase);
  const control = useNginxStore((state) => state.controlInstance);

  async function run(action: NginxControlAction) {
    const succeeded = await control(action);
    const state = useNginxStore.getState();
    toast.add({
      title: succeeded ? t("toast.operationSuccess") : t("toast.errorTitle"),
      description: succeeded
        ? t(state.lastOperation?.outcome === "noop"
          ? "toast.operationNoop"
          : "toast.operationExecuted")
        : t(`errors.${state.error?.code ?? "NGINX_UNKNOWN"}`),
      type: succeeded ? "success" : "error",
    });
  }

  if (!instance) return null;
  const running = instance.runtimeStatus === "running";
  const verified = !["conflict", "unknown"].includes(instance.runtimeStatus);
  const label = phase ? t(`operationPhase.${phase}`) : null;

  return (
    <div className="flex flex-wrap items-center gap-2">
      <Button
        disabled={busy || !instance.capabilities.canControl || running}
        onClick={() => void run("start")}
        size="sm"
      >
        {busy && phase === "starting" ? <Spinner data-icon="inline-start" /> : <PlayIcon data-icon="inline-start" />}
        {label && phase === "starting" ? label : t("control.startLabel")}
      </Button>
      <Button
        disabled={busy || !instance.capabilities.canControl || !running}
        onClick={() => void run("stop")}
        size="sm"
        variant="outline"
      >
        <SquareIcon data-icon="inline-start" />{t("control.stopLabel")}
      </Button>
      <Button
        disabled={busy || !instance.capabilities.canControl || !running}
        onClick={() => void run("reload")}
        size="sm"
        variant="outline"
      >
        <RotateCwIcon data-icon="inline-start" />{t("control.reloadLabel")}
      </Button>
      <Button
        disabled={busy || !instance.capabilities.canControl || !verified}
        onClick={() => void run("restart")}
        size="sm"
        variant="outline"
      >
        <RefreshCwIcon data-icon="inline-start" />{t("control.restartLabel")}
      </Button>
    </div>
  );
}
