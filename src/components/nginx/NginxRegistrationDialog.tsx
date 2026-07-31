import { useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertTriangleIcon } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldDescription, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import type {
  NginxAuthorizationLevel,
  NginxInspection,
} from "src/types/nginx";

interface NginxRegistrationDialogProps {
  inspection: NginxInspection;
  loading: boolean;
  onCancel: () => void;
  onRegister: (
    name: string,
    authorizationLevel: NginxAuthorizationLevel,
  ) => void;
}

const authorizationOptions: Array<{
  value: NginxAuthorizationLevel;
  labelKey: string;
}> = [
  { value: "readOnly", labelKey: "registration.authorization.readOnly" },
  { value: "full", labelKey: "registration.authorization.full" },
];

export function NginxRegistrationDialog({
  inspection,
  loading,
  onCancel,
  onRegister,
}: NginxRegistrationDialogProps) {
  const { t } = useTranslation("nginx");
  const [name, setName] = useState("Nginx");
  const [authorizationLevel, setAuthorizationLevel] =
    useState<NginxAuthorizationLevel>("readOnly");
  const validName = name.trim().length > 0 && name.trim().length <= 80;

  return (
    <Dialog open onOpenChange={(open) => !open && onCancel()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{t("registration.title")}</DialogTitle>
          <DialogDescription>{t("registration.description")}</DialogDescription>
        </DialogHeader>

        <div className="rounded-lg border bg-muted/30 p-3 text-sm">
          <p className="font-medium">nginx/{inspection.version}</p>
          <p className="break-all text-muted-foreground">
            {inspection.displayBinary}
          </p>
        </div>

        {inspection.warnings.length > 0 ? (
          <Alert>
            <AlertTriangleIcon />
            <AlertTitle>{t("registration.warningsTitle")}</AlertTitle>
            <AlertDescription>
              {inspection.warnings.map((warning) => (
                <p key={warning}>{t(`warnings.${warning}`)}</p>
              ))}
            </AlertDescription>
          </Alert>
        ) : null}

        <form
          id="nginx-registration-form"
          onSubmit={(event) => {
            event.preventDefault();
            if (validName) {
              onRegister(name.trim(), authorizationLevel);
            }
          }}
        >
          <FieldGroup>
            <Field data-invalid={!validName}>
              <FieldLabel htmlFor="nginx-instance-name">
                {t("registration.name")}
              </FieldLabel>
              <Input
                aria-invalid={!validName}
                id="nginx-instance-name"
                maxLength={80}
                onChange={(event) => setName(event.target.value)}
                value={name}
              />
            </Field>
            <Field>
              <FieldLabel>{t("registration.authorization.label")}</FieldLabel>
              <Select
                items={authorizationOptions.map((option) => ({
                  label: t(option.labelKey),
                  value: option.value,
                }))}
                onValueChange={(value) =>
                  value && setAuthorizationLevel(value as NginxAuthorizationLevel)
                }
                value={authorizationLevel}
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent alignItemWithTrigger={false}>
                  <SelectGroup>
                    {authorizationOptions.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {t(option.labelKey)}
                      </SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
              <FieldDescription>
                {t(`registration.authorization.${authorizationLevel}Description`)}
              </FieldDescription>
            </Field>
          </FieldGroup>
        </form>

        <DialogFooter>
          <Button disabled={loading} onClick={onCancel} variant="outline">
            {t("common:actions.cancel")}
          </Button>
          <Button
            disabled={loading || !validName}
            form="nginx-registration-form"
            type="submit"
          >
            {loading ? <Spinner data-icon="inline-start" /> : null}
            {t("registration.register")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
