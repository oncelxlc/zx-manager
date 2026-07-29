const byteUnits = ["B", "KB", "MB", "GB", "TB", "PB"] as const;

export function formatDataSize(
  value: number | null,
  locale: string,
  unavailable = "—",
): string {
  if (value === null || !Number.isFinite(value) || value < 0) {
    return unavailable;
  }
  if (value === 0) {
    return `0 ${byteUnits[0]}`;
  }
  const unitIndex = Math.min(
    Math.floor(Math.log(value) / Math.log(1024)),
    byteUnits.length - 1,
  );
  const scaled = value / (1024 ** unitIndex);
  return `${new Intl.NumberFormat(locale, {
    maximumFractionDigits: scaled >= 100 ? 0 : 1,
  }).format(scaled)} ${byteUnits[unitIndex]}`;
}

export function formatDataRate(
  value: number | null,
  locale: string,
  unavailable = "—",
): string {
  const formatted = formatDataSize(value, locale, unavailable);
  return formatted === unavailable ? unavailable : `${formatted}/s`;
}
