import { useEffect, useState, type FormEvent } from "react";
import { PlusIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  Field,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type {
  AddServiceInput,
  ServiceType,
  StartupMode,
} from "src/types/service";

interface AddServiceForm {
  name: string;
  type: ServiceType | null;
  executablePath: string;
  workingDirectory: string;
  startupMode: StartupMode | null;
  port: string;
}

type FormErrors = Partial<Record<keyof AddServiceForm, string>>;

const emptyForm: AddServiceForm = {
  name: "",
  type: null,
  executablePath: "",
  workingDirectory: "",
  startupMode: null,
  port: "",
};

function validateForm(form: AddServiceForm, t: (key: string) => string): FormErrors {
  const errors: FormErrors = {};

  if (!form.name.trim()) {
    errors.name = t("validation:serviceNameRequired");
  }
  if (!form.type) {
    errors.type = t("validation:serviceTypeRequired");
  }
  if (!form.executablePath.trim()) {
    errors.executablePath = t("validation:executablePathRequired");
  }
  if (!form.workingDirectory.trim()) {
    errors.workingDirectory = t("validation:workingDirectoryRequired");
  }
  if (!form.startupMode) {
    errors.startupMode = t("validation:startupModeRequired");
  }
  if (form.port.trim()) {
    const port = Number(form.port);
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      errors.port = t("validation:invalidPort");
    }
  }

  return errors;
}

export function AddServiceDialog({
  onAddService,
}: {
  onAddService: (input: AddServiceInput) => void;
}) {
  const { i18n, t } = useTranslation(["services", "validation", "common"]);
  const [open, setOpen] = useState(false);
  const [form, setForm] = useState<AddServiceForm>(emptyForm);
  const [errors, setErrors] = useState<FormErrors>({});

  useEffect(() => {
    setErrors((current) =>
      Object.keys(current).length > 0 ? validateForm(form, t) : current,
    );
  }, [form, i18n.language, t]);

  function updateField<Key extends keyof AddServiceForm>(
    key: Key,
    value: AddServiceForm[Key],
  ) {
    setForm((current) => ({ ...current, [key]: value }));
    setErrors((current) => ({ ...current, [key]: undefined }));
  }

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const nextErrors = validateForm(form, t);
    setErrors(nextErrors);

    if (Object.keys(nextErrors).length > 0 || !form.type || !form.startupMode) {
      return;
    }

    onAddService({
      name: form.name.trim(),
      type: form.type,
      executablePath: form.executablePath.trim(),
      workingDirectory: form.workingDirectory.trim(),
      startupMode: form.startupMode,
      port: form.port.trim() ? Number(form.port) : undefined,
    });
    setForm(emptyForm);
    setErrors({});
    setOpen(false);
  }

  function handleOpenChange(nextOpen: boolean) {
    setOpen(nextOpen);
    if (!nextOpen) {
      setErrors({});
    }
  }

  return (
    <Dialog onOpenChange={handleOpenChange} open={open}>
      <DialogTrigger render={<Button />}>
        <PlusIcon data-icon="inline-start" />
        {t("dialog.add")}
      </DialogTrigger>
      <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-xl">
        <form className="flex flex-col gap-5" onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>{t("dialog.title")}</DialogTitle>
            <DialogDescription>{t("dialog.description")}</DialogDescription>
          </DialogHeader>

          <FieldGroup>
            <div className="grid gap-4 sm:grid-cols-2">
              <Field data-invalid={Boolean(errors.name) || undefined}>
                <FieldLabel htmlFor="service-name">{t("dialog.serviceName")}</FieldLabel>
                <Input
                  aria-invalid={Boolean(errors.name)}
                  id="service-name"
                  onChange={(event) => updateField("name", event.target.value)}
                  placeholder={t("dialog.serviceName")}
                  value={form.name}
                />
                <FieldError>{errors.name}</FieldError>
              </Field>

              <Field data-invalid={Boolean(errors.type) || undefined}>
                <FieldLabel htmlFor="service-type">{t("dialog.serviceType")}</FieldLabel>
                <Select
                  items={(["nginx", "database", "cache", "node", "application", "system"] as ServiceType[]).map((value) => ({
                    label: t(`types.${value}`),
                    value,
                  }))}
                  onValueChange={(value) => updateField("type", value)}
                  value={form.type}
                >
                  <SelectTrigger
                    aria-invalid={Boolean(errors.type)}
                    className="w-full"
                    id="service-type"
                  >
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent alignItemWithTrigger={false}>
                    <SelectGroup>
                      {(["nginx", "database", "cache", "node", "application", "system"] as ServiceType[])
                        .map((value) => (
                          <SelectItem key={value} value={value}>
                            {t(`types.${value}`)}
                          </SelectItem>
                        ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
                <FieldError>{errors.type}</FieldError>
              </Field>
            </div>

            <Field data-invalid={Boolean(errors.executablePath) || undefined}>
              <FieldLabel htmlFor="executable-path">
                {t("dialog.executablePath")}
              </FieldLabel>
              <Input
                aria-invalid={Boolean(errors.executablePath)}
                id="executable-path"
                onChange={(event) =>
                  updateField("executablePath", event.target.value)
                }
                placeholder="C:\tools\service\server.exe"
                value={form.executablePath}
              />
              <FieldError>{errors.executablePath}</FieldError>
            </Field>

            <Field data-invalid={Boolean(errors.workingDirectory) || undefined}>
              <FieldLabel htmlFor="working-directory">
                {t("dialog.workingDirectory")}
              </FieldLabel>
              <Input
                aria-invalid={Boolean(errors.workingDirectory)}
                id="working-directory"
                onChange={(event) =>
                  updateField("workingDirectory", event.target.value)
                }
                placeholder="C:\tools\service"
                value={form.workingDirectory}
              />
              <FieldError>{errors.workingDirectory}</FieldError>
            </Field>

            <div className="grid gap-4 sm:grid-cols-2">
              <Field data-invalid={Boolean(errors.startupMode) || undefined}>
                <FieldLabel htmlFor="startup-mode">{t("dialog.startupMode")}</FieldLabel>
                <Select
                  items={(["automatic", "manual", "disabled"] as StartupMode[]).map((value) => ({
                    label: t(`startupModes.${value}`),
                    value,
                  }))}
                  onValueChange={(value) =>
                    updateField("startupMode", value)
                  }
                  value={form.startupMode}
                >
                  <SelectTrigger
                    aria-invalid={Boolean(errors.startupMode)}
                    className="w-full"
                    id="startup-mode"
                  >
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent alignItemWithTrigger={false}>
                    <SelectGroup>
                      {(["automatic", "manual", "disabled"] as StartupMode[])
                        .map((value) => (
                          <SelectItem key={value} value={value}>
                            {t(`startupModes.${value}`)}
                          </SelectItem>
                        ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
                <FieldError>{errors.startupMode}</FieldError>
              </Field>

              <Field data-invalid={Boolean(errors.port) || undefined}>
                <FieldLabel htmlFor="service-port">{t("dialog.port")}</FieldLabel>
                <Input
                  aria-invalid={Boolean(errors.port)}
                  id="service-port"
                  inputMode="numeric"
                  max="65535"
                  min="1"
                  onChange={(event) => updateField("port", event.target.value)}
                  placeholder={t("dialog.optional")}
                  type="number"
                  value={form.port}
                />
                <FieldError>{errors.port}</FieldError>
              </Field>
            </div>
          </FieldGroup>

          <DialogFooter>
            <Button
              onClick={() => setOpen(false)}
              type="button"
              variant="outline"
            >
              {t("common:actions.cancel")}
            </Button>
            <Button type="submit">{t("dialog.add")}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
