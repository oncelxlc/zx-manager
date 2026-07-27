import App from "./App.tsx";
import React from "react";
import ReactDOM from "react-dom/client";
import "./styles/globals.css";
import { applyTheme } from "@/components/theme-provider";
import { initializeI18n } from "src/i18n";
import { getPreferences } from "src/services/storage/preferences-storage";

async function bootstrap() {
  const preferences = await getPreferences();
  const locale = preferences.locale ?? "zh-CN";
  const theme = preferences.theme ?? "dark";

  applyTheme(theme);
  await initializeI18n(locale);

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App initialTheme={theme} />
    </React.StrictMode>,
  );
}

void bootstrap();
