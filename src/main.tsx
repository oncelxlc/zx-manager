import App from "./App.tsx";
import React from "react";
import { flushSync } from "react-dom";
import ReactDOM from "react-dom/client";
import "./styles/globals.css";
import { applyTheme } from "@/components/theme-provider";
import { StartupScreen } from "src/components/startup/StartupScreen";
import { initializeI18n } from "src/i18n";
import { getPreferences } from "src/services/storage/preferences-storage";
import { restoreNetworkMonitorOnStartup } from "src/services/tauri/network-monitor-startup";
import type { UserPreferences } from "src/types/preferences";

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);

flushSync(() => {
  root.render(<StartupScreen/>);
});

async function bootstrap() {
  let preferences: Partial<UserPreferences> = {};

  try {
    preferences = await getPreferences();
  } catch {
    // Preference persistence must never prevent the application from opening.
  }

  const locale = preferences.locale ?? "zh-CN";
  const theme = preferences.theme ?? "dark";

  applyTheme(theme);
  try {
    await initializeI18n(locale);
  } catch {
    // The bundled fallback locale still allows the application shell to mount.
  }

  root.render(
    <React.StrictMode>
      <App initialTheme={theme}/>
    </React.StrictMode>,
  );

  void restoreNetworkMonitorOnStartup(preferences);
}

void bootstrap();
