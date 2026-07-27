import type { SupportedLocale } from "src/types/preferences";

function toIntlLocale(locale: SupportedLocale) {
  return locale === "zh-CN" ? "zh-CN" : "en-US";
}

export function formatNumber(value: number, locale: SupportedLocale) {
  return new Intl.NumberFormat(toIntlLocale(locale)).format(value);
}

export function formatPercent(value: number, locale: SupportedLocale) {
  return new Intl.NumberFormat(toIntlLocale(locale), {
    maximumFractionDigits: 1,
    style: "percent",
  }).format(value / 100);
}

export function formatBytes(megabytes: number, locale: SupportedLocale) {
  if (megabytes >= 1024) {
    return new Intl.NumberFormat(toIntlLocale(locale), {
      maximumFractionDigits: 1,
    }).format(megabytes / 1024) + " GB";
  }

  return `${formatNumber(megabytes, locale)} MB`;
}

export function formatChartDate(value: string, locale: SupportedLocale, range: string) {
  const date = new Date(value);
  const options: Intl.DateTimeFormatOptions =
    range === "24h"
      ? { hour: "2-digit", minute: "2-digit" }
      : range === "7d"
        ? { weekday: "short" }
        : { month: "short", day: "numeric" };

  return new Intl.DateTimeFormat(toIntlLocale(locale), options).format(date);
}
