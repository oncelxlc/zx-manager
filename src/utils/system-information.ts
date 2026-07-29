import type {
  CommandError,
  SystemInformation,
  SystemSummary,
} from "src/types/system-information";

const BYTE_UNITS = ["B", "KB", "MB", "GB", "TB", "PB"] as const;

export function formatBytes(
  value: number | null,
  locale: string,
  unavailable: string,
): string {
  if (value === null || !Number.isFinite(value) || value < 0) {
    return unavailable;
  }

  if (value === 0) {
    return `0 ${BYTE_UNITS[0]}`;
  }

  const unitIndex = Math.min(
    Math.floor(Math.log(value) / Math.log(1024)),
    BYTE_UNITS.length - 1,
  );
  const scaled = value / (1024 ** unitIndex);
  return `${new Intl.NumberFormat(locale, {
    maximumFractionDigits: scaled >= 100 ? 0 : 1,
  }).format(scaled)} ${BYTE_UNITS[unitIndex]}`;
}

export function formatPercent(
  value: number | null,
  locale: string,
  unavailable: string,
): string {
  if (value === null || !Number.isFinite(value)) {
    return unavailable;
  }

  return new Intl.NumberFormat(locale, {
    maximumFractionDigits: 1,
    style: "percent",
  }).format(Math.min(100, Math.max(0, value)) / 100);
}

export function formatFrequency(
  valueMhz: number | null,
  locale: string,
  unavailable: string,
): string {
  if (valueMhz === null || !Number.isFinite(valueMhz) || valueMhz < 0) {
    return unavailable;
  }

  return `${new Intl.NumberFormat(locale, {
    maximumFractionDigits: 2,
  }).format(valueMhz / 1000)} GHz`;
}

export function formatDuration(
  seconds: number | null,
  locale: string,
  unavailable: string,
  labels: { day: string; hour: string; minute: string },
): string {
  if (seconds === null || !Number.isFinite(seconds) || seconds < 0) {
    return unavailable;
  }

  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor((seconds % 86_400) / 3_600);
  const minutes = Math.floor((seconds % 3_600) / 60);
  const values = [
    days > 0 ? `${new Intl.NumberFormat(locale).format(days)} ${labels.day}` : null,
    hours > 0 ? `${new Intl.NumberFormat(locale).format(hours)} ${labels.hour}` : null,
    minutes > 0 || (days === 0 && hours === 0)
      ? `${new Intl.NumberFormat(locale).format(minutes)} ${labels.minute}`
      : null,
  ];

  return values.filter(Boolean).slice(0, 2).join(" ");
}

export function formatCollectedAt(
  value: string | null,
  locale: string,
  unavailable: string,
): string {
  if (!value) {
    return unavailable;
  }

  const date = new Date(value);
  if (Number.isNaN(date.valueOf())) {
    return unavailable;
  }

  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: "medium",
  }).format(date);
}

export function formatMachinePlatform(
  summary: SystemSummary | null,
  unavailable: string,
): string {
  if (!summary) {
    return unavailable;
  }

  const operatingSystem = summary.hostName ?? summary.osLongVersion ?? summary.osName ?? summary.osVersion;
  return [operatingSystem, summary.architecture].filter(Boolean).join(" · ")
    || unavailable;
}

export function getErrorTranslationKey(error: CommandError | null): string {
  if (!error) {
    return "errors.unknown";
  }

  return `errors.${error.code}`;
}

export function createDiagnosticReport(information: SystemInformation) {
  return {
    schemaVersion: 1,
    collectedAt: information.collectedAt,
    warnings: information.warnings,
    system: {
      platform: information.system.platform,
      osName: information.system.osName,
      osVersion: information.system.osVersion,
      osLongVersion: information.system.osLongVersion,
      kernelVersion: information.system.kernelVersion,
      architecture: information.system.architecture,
      uptimeSeconds: information.system.uptimeSeconds,
    },
    cpu: {
      model: information.cpu.model,
      vendor: information.cpu.vendor,
      physicalCoreCount: information.cpu.physicalCoreCount,
      logicalCoreCount: information.cpu.logicalCoreCount,
      frequencyMhz: information.cpu.frequencyMhz,
      usagePercent: information.cpu.usagePercent,
    },
    memory: {...information.memory},
    gpus: information.gpus.map((gpu) => ({
      name: gpu.name,
      vendorId: gpu.vendorId,
      deviceId: gpu.deviceId,
      deviceType: gpu.deviceType,
      backend: gpu.backend,
      dedicatedMemoryBytes: gpu.dedicatedMemoryBytes,
      sharedMemoryBytes: gpu.sharedMemoryBytes,
    })),
    disks: information.disks.map((disk) => ({
      name: disk.name,
      fileSystem: disk.fileSystem,
      kind: disk.kind,
      removable: disk.removable,
      totalBytes: disk.totalBytes,
      availableBytes: disk.availableBytes,
      usedBytes: disk.usedBytes,
      usagePercent: disk.usagePercent,
    })),
    runtime: {...information.runtime},
    availability: {...information.availability},
  };
}

export function serializeDiagnosticReport(
  information: SystemInformation,
): string {
  return JSON.stringify(createDiagnosticReport(information), null, 2);
}
