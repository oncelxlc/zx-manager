import { useState, type FormEvent } from "react";
import { PlusIcon } from "lucide-react";

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

const serviceTypeItems: Array<{
  label: string;
  value: ServiceType | null;
}> = [
  { label: "Select a service type", value: null },
  { label: "Nginx", value: "nginx" },
  { label: "Database", value: "database" },
  { label: "Cache", value: "cache" },
  { label: "Node", value: "node" },
  { label: "Application", value: "application" },
  { label: "System", value: "system" },
];

const startupModeItems: Array<{
  label: string;
  value: StartupMode | null;
}> = [
  { label: "Select a startup mode", value: null },
  { label: "Automatic", value: "automatic" },
  { label: "Manual", value: "manual" },
  { label: "Disabled", value: "disabled" },
];

function validateForm(form: AddServiceForm): FormErrors {
  const errors: FormErrors = {};

  if (!form.name.trim()) {
    errors.name = "Service name is required.";
  }
  if (!form.type) {
    errors.type = "Select a service type.";
  }
  if (!form.executablePath.trim()) {
    errors.executablePath = "Executable path is required.";
  }
  if (!form.workingDirectory.trim()) {
    errors.workingDirectory = "Working directory is required.";
  }
  if (!form.startupMode) {
    errors.startupMode = "Select a startup mode.";
  }
  if (form.port.trim()) {
    const port = Number(form.port);
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      errors.port = "Port must be an integer between 1 and 65535.";
    }
  }

  return errors;
}

export function AddServiceDialog({
  onAddService,
}: {
  onAddService: (input: AddServiceInput) => void;
}) {
  const [open, setOpen] = useState(false);
  const [form, setForm] = useState<AddServiceForm>(emptyForm);
  const [errors, setErrors] = useState<FormErrors>({});

  function updateField<Key extends keyof AddServiceForm>(
    key: Key,
    value: AddServiceForm[Key],
  ) {
    setForm((current) => ({ ...current, [key]: value }));
    setErrors((current) => ({ ...current, [key]: undefined }));
  }

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const nextErrors = validateForm(form);
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
        Add Service
      </DialogTrigger>
      <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-xl">
        <form className="flex flex-col gap-5" onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>Add local service</DialogTitle>
            <DialogDescription>
              Register a local executable for management. No system command will
              be executed.
            </DialogDescription>
          </DialogHeader>

          <FieldGroup>
            <div className="grid gap-4 sm:grid-cols-2">
              <Field data-invalid={Boolean(errors.name) || undefined}>
                <FieldLabel htmlFor="service-name">Service Name</FieldLabel>
                <Input
                  aria-invalid={Boolean(errors.name)}
                  id="service-name"
                  onChange={(event) => updateField("name", event.target.value)}
                  placeholder="Local API"
                  value={form.name}
                />
                <FieldError>{errors.name}</FieldError>
              </Field>

              <Field data-invalid={Boolean(errors.type) || undefined}>
                <FieldLabel htmlFor="service-type">Service Type</FieldLabel>
                <Select
                  items={serviceTypeItems}
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
                      {serviceTypeItems
                        .filter(
                          (
                            item,
                          ): item is { label: string; value: ServiceType } =>
                            item.value !== null,
                        )
                        .map((item) => (
                          <SelectItem key={item.value} value={item.value}>
                            {item.label}
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
                Executable Path
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
                Working Directory
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
                <FieldLabel htmlFor="startup-mode">Startup Mode</FieldLabel>
                <Select
                  items={startupModeItems}
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
                      {startupModeItems
                        .filter(
                          (
                            item,
                          ): item is { label: string; value: StartupMode } =>
                            item.value !== null,
                        )
                        .map((item) => (
                          <SelectItem key={item.value} value={item.value}>
                            {item.label}
                          </SelectItem>
                        ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
                <FieldError>{errors.startupMode}</FieldError>
              </Field>

              <Field data-invalid={Boolean(errors.port) || undefined}>
                <FieldLabel htmlFor="service-port">Port</FieldLabel>
                <Input
                  aria-invalid={Boolean(errors.port)}
                  id="service-port"
                  inputMode="numeric"
                  max="65535"
                  min="1"
                  onChange={(event) => updateField("port", event.target.value)}
                  placeholder="Optional"
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
              Cancel
            </Button>
            <Button type="submit">Add Service</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
