import { ExternalLinkIcon, ShieldCheckIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetClose,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Spinner } from "@/components/ui/spinner";
import { toast } from "@/components/ui/toast";
import { useNginxGlobalConfigStore } from "src/stores/nginx-global-config-store";
import type { NginxGlobalConfigApplyMode } from "src/types/nginx";
import { NginxGlobalConfigFields } from "./NginxGlobalConfigFields";

export function NginxGlobalConfigSheet({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation("nginx");
  const source = useNginxGlobalConfigStore((state) => state.source);
  const draft = useNginxGlobalConfigStore((state) => state.draft);
  const validation = useNginxGlobalConfigStore((state) => state.validation);
  const result = useNginxGlobalConfigStore((state) => state.result);
  const error = useNginxGlobalConfigStore((state) => state.error);
  const status = useNginxGlobalConfigStore((state) => state.status);
  const update = useNginxGlobalConfigStore((state) => state.update);
  const validate = useNginxGlobalConfigStore((state) => state.validate);
  const apply = useNginxGlobalConfigStore((state) => state.apply);
  const load = useNginxGlobalConfigStore((state) => state.load);
  const discard = useNginxGlobalConfigStore((state) => state.discard);
  const busy = ["loading", "validating", "applying"].includes(status);

  async function run(mode: NginxGlobalConfigApplyMode) {
    const succeeded = await apply(mode);
    const state = useNginxGlobalConfigStore.getState();
    toast.add({
      title: t(succeeded ? "globalConfig.applySucceeded" : "globalConfig.applyFailed"),
      description: state.result?.rolledBack
        ? t(state.result.rollbackSucceeded
          ? "globalConfig.rollbackSucceeded"
          : "globalConfig.rollbackFailed")
        : undefined,
      type: succeeded ? "success" : "error",
    });
    if (succeeded) onOpenChange(false);
  }

  return (
    <Sheet open={open} onOpenChange={(next) => { if (!busy) onOpenChange(next); }}>
      <SheetContent className="sm:max-w-2xl" showCloseButton={!busy}>
        <SheetHeader>
          <SheetTitle>{t("globalConfig.editTitle")}</SheetTitle>
          <SheetDescription>{t("globalConfig.editDescription")}</SheetDescription>
        </SheetHeader>
        <div className="min-h-0 flex-1 overflow-auto px-4">
          {source && draft ? (
            <div className="flex flex-col gap-5 pb-4">
              <p className="break-all text-xs text-muted-foreground">
                {t("globalConfig.revision", { revision: source.revision.value })}
              </p>
              {error?.code === "NGINX_CONFIG_REVISION_CONFLICT" ? (
                <Alert variant="destructive">
                  <AlertTitle>{t("globalConfig.conflictTitle")}</AlertTitle>
                  <AlertDescription className="flex flex-wrap gap-2">
                    <span>{t("globalConfig.conflictDescription")}</span>
                    <Button render={<Link to="/nginx/configuration" />} size="sm" variant="outline">
                      <ExternalLinkIcon data-icon="inline-start" />{t("globalConfig.viewDisk")}
                    </Button>
                    <Button
                      onClick={() => void load(source.instanceId)}
                      size="sm"
                      variant="outline"
                    >
                      {t("globalConfigReloadDisk")}
                    </Button>
                    <Button onClick={discard} size="sm" variant="outline">
                      {t("globalConfig.discard")}
                    </Button>
                  </AlertDescription>
                </Alert>
              ) : null}
              {validation ? (
                <Alert variant={validation.parserValid && validation.nativeValid ? "default" : "destructive"}>
                  <AlertTitle>{t("globalConfig.validationTitle")}</AlertTitle>
                  <AlertDescription>{t(
                    validation.parserValid && validation.nativeValid
                      ? "globalConfig.validationPassed"
                      : "globalConfig.validationFailed",
                  )}</AlertDescription>
                </Alert>
              ) : null}
              {result?.rolledBack ? (
                <Alert variant="destructive"><AlertTitle>{t("globalConfig.applyFailed")}</AlertTitle>
                  <AlertDescription>{t(result.rollbackSucceeded
                    ? "globalConfig.rollbackSucceeded"
                    : "globalConfig.rollbackFailed")}</AlertDescription>
                </Alert>
              ) : null}
              <NginxGlobalConfigFields
                draft={draft}
                errors={validation?.fieldErrors ?? []}
                onChange={update}
              />
            </div>
          ) : <div className="flex min-h-48 items-center justify-center"><Spinner /></div>}
        </div>
        <SheetFooter className="sm:flex-row sm:justify-end">
          <Button disabled={busy || !draft} onClick={() => void validate()} variant="outline">
            <ShieldCheckIcon data-icon="inline-start" />{t("globalConfig.validate")}
          </Button>
          <SheetClose disabled={busy} render={<Button variant="ghost" />}>{t("globalConfig.cancel")}</SheetClose>
          <Button disabled={busy || !draft} onClick={() => void run("save")} variant="outline">
            {t("globalConfig.saveOnly")}
          </Button>
          <Button disabled={busy || !draft} onClick={() => void run("restart")} variant="outline">
            {t("globalConfig.saveRestart")}
          </Button>
          <Button disabled={busy || !draft} onClick={() => void run("reload")}>
            {status === "applying" ? <Spinner data-icon="inline-start" /> : null}
            {t("globalConfig.saveReload")}
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
