import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import enUS from "./locales/en-US";
import zhCN from "./locales/zh-CN";
import type { SupportedLocale } from "src/types/preferences";

const resources = {
  "zh-CN": zhCN,
  "en-US": enUS,
};

function applyDocumentLocale(locale: SupportedLocale) {
  if (typeof document === "undefined") {
    return;
  }

  document.documentElement.lang = locale;
  document.documentElement.dir = "ltr";
}

export async function initializeI18n(locale: SupportedLocale) {
  if (!i18n.isInitialized) {
    await i18n.use(initReactI18next).init({
      fallbackLng: "zh-CN",
      interpolation: { escapeValue: false },
      lng: locale,
      resources,
      returnNull: false,
    });
  } else if (i18n.language !== locale) {
    await i18n.changeLanguage(locale);
  }

  applyDocumentLocale(locale);
}

export async function changeLocale(locale: SupportedLocale) {
  await i18n.changeLanguage(locale);
  applyDocumentLocale(locale);
}

export default i18n;
