import { useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { toast } from "@/components/ui/toast";
import { initialServices } from "src/data/dashboard-mock-data";
import {
  restartService,
  startService,
  stopService,
} from "src/services/tauri/service-manager";
import type {
  AddServiceInput,
  LocalService,
  ServiceOperation,
} from "src/types/service";
import { useNginxStore } from "src/stores/nginx-store";

const operationHandlers = {
  start: startService,
  stop: stopService,
  restart: restartService,
} satisfies Record<ServiceOperation, (serviceId: string) => Promise<void>>;

function createServiceId(name: string, sequence: number) {
  const slug = name
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");

  return `${slug || "service"}-${sequence}`;
}

/** Contains mock-service mutations so DashboardPage only composes feature areas. */
export function useDashboardServices() {
  const { t } = useTranslation(["services", "common", "nginx"]);
  const [services, setServices] = useState<LocalService[]>(
    initialServices.filter((service) => service.type !== "nginx"),
  );
  const [pendingIds, setPendingIds] = useState<Set<string>>(new Set());
  const nextServiceSequence = useRef(initialServices.length + 1);
  const nginx = useNginxStore((state) => state.instance);
  const nginxOperationStatus = useNginxStore((state) => state.operationStatus);
  const controlNginx = useNginxStore((state) => state.controlInstance);
  const unregisterNginx = useNginxStore((state) => state.unregisterInstance);
  const nginxService = useMemo<LocalService | null>(() => nginx ? ({
    id: nginx.id,
    name: nginx.name,
    descriptionKey: "nginx",
    type: "nginx",
    status: nginx.runtimeStatus === "running"
      ? "running"
      : nginx.runtimeStatus === "stopped" ? "stopped"
        : nginx.runtimeStatus === "conflict" ? "error" : "warning",
    version: nginx.version,
    port: 80,
    cpu: 0,
    memory: "—",
  }) : null, [nginx]);

  async function handleOperation(
    service: LocalService,
    operation: ServiceOperation,
  ) {
    setPendingIds((current) => new Set(current).add(service.id));

    try {
      if (service.type === "nginx") {
        const succeeded = await controlNginx(operation);
        if (!succeeded) {
          const error = useNginxStore.getState().error;
          throw new Error(t(`nginx:errors.${error?.code ?? "NGINX_UNKNOWN"}`));
        }
      } else {
        await operationHandlers[operation](service.id);
      }
      setServices((current) => current.map((item) => {
        if (item.id !== service.id) {
          return item;
        }

        return operation === "stop"
          ? {...item, cpu: 0, memory: "0 MB", status: "stopped"}
          : {...item, status: "running"};
      }));
      toast.add({
        title: t("services:toast.operationSuccess", {
          name: service.name,
          operation: t(`services:toast.operations.${operation}`),
        }),
        description: t(service.type === "nginx"
          ? "services:toast.operationSuccessRealDescription"
          : "services:toast.operationSuccessDescription"),
        type: "success",
      });
    } catch (error) {
      toast.add({
        title: t("services:toast.operationError", {
          name: service.name,
          operation: t(`services:actions.${operation}`),
        }),
        description: error instanceof Error
          ? error.message
          : t("common:status.error"),
        type: "error",
      });
    } finally {
      setPendingIds((current) => {
        const nextPending = new Set(current);
        nextPending.delete(service.id);
        return nextPending;
      });
    }
  }

  function handleAddService(input: AddServiceInput) {
    const sequence = nextServiceSequence.current;
    nextServiceSequence.current += 1;

    const newService: LocalService = {
      id: createServiceId(input.name, sequence),
      name: input.name,
      descriptionKey: "custom",
      descriptionValues: {
        path: input.executablePath,
        startupMode: t(`services:startupModes.${input.startupMode}`),
      },
      type: input.type,
      status: "stopped",
      version: "—",
      port: input.port,
      cpu: 0,
      memory: "0 MB",
    };

    setServices((current) => [...current, newService]);
    toast.add({
      title: t("services:toast.added", {name: input.name}),
      description: t("services:toast.addedDescription"),
      type: "success",
    });
  }

  function handleRemove(service: LocalService) {
    if (service.type === "nginx") {
      void unregisterNginx();
      return;
    }
    setServices((current) => current.filter((item) => item.id !== service.id));
    toast.add({
      title: t("services:toast.removed", {name: service.name}),
      description: t("services:toast.removedDescription"),
      type: "success",
    });
  }

  function handleLocalAction(title: string, service: LocalService) {
    toast.add({
      title,
      description: t("services:toast.localAction", {name: service.name}),
      type: "info",
    });
  }

  return {
    handleAddService,
    handleLocalAction,
    handleOperation,
    handleRemove,
    pendingIds: nginx && nginxOperationStatus === "loading"
      ? new Set([...pendingIds, nginx.id])
      : pendingIds,
    services: nginxService ? [nginxService, ...services] : services,
  };
}
