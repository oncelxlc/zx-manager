import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { ChevronRightIcon } from "lucide-react";
import { Outlet, useLocation } from "react-router";

import { AppSidebar } from "src/components/AppSidebar";
import { Separator } from "@/components/ui/separator";
import {
  SidebarInset,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar";
import { completeStartup } from "src/services/tauri/startup";

interface MainLayoutHeaderOptions {
  title: string;
  actions?: ReactNode;
}

interface MainLayoutHeaderConfig extends MainLayoutHeaderOptions {
  pathname: string;
}

interface MainLayoutHeaderContextValue {
  registerHeader: (config: MainLayoutHeaderConfig) => () => void;
}

const MainLayoutHeaderContext =
  createContext<MainLayoutHeaderContextValue | null>(null);

export function useMainLayoutHeader({
                                      title,
                                      actions,
                                    }: MainLayoutHeaderOptions) {
  const context = useContext(MainLayoutHeaderContext);
  const {pathname} = useLocation();

  useEffect(() => {
    if (!context) {
      return;
    }

    return context.registerHeader({actions, pathname, title});
  }, [actions, context, pathname, title]);

  if (!context) {
    throw new Error("useMainLayoutHeader must be used within MainLayout.");
  }
}

export default function MainLayout() {
  const {t} = useTranslation("common");
  const location = useLocation();
  const [headerConfig, setHeaderConfig] =
    useState<MainLayoutHeaderConfig | null>(null);
  const registerHeader = useCallback((config: MainLayoutHeaderConfig) => {
    setHeaderConfig(config);

    return () => {
      setHeaderConfig((current) => current === config ? null : current);
    };
  }, []);
  const headerContext = useMemo(
    () => ({registerHeader}),
    [registerHeader],
  );
  const activeHeader =
    headerConfig?.pathname === location.pathname ? headerConfig : null;

  useEffect(() => {
    void completeStartup().catch(() => {
      // Keep the static splashscreen visible if the native handoff fails.
    });
  }, []);

  return (
    <SidebarProvider className="bg-sidebar"
                     style={{"--sidebar-width": "15rem"} as React.CSSProperties}>
      <AppSidebar/>
      <div className="pt-14.25 flex min-w-0 flex-1 flex-col bg-sidebar h-full">
        <header className="fixed top-0 z-20 w-[stretch] shrink-0 bg-sidebar text-sidebar-foreground"
                data-slot="main-layout-header">
          <div
            className="mx-auto flex w-full max-w-[1800px] flex-wrap items-center justify-between gap-4 px-4 py-3 lg:px-6">
            <div className="flex min-w-0 items-center gap-3">
              <SidebarTrigger/>
              <Separator className="h-4" orientation="vertical"/>
              <span className="truncate text-sm text-muted-foreground">
                {t("app.name")}
              </span>
              {activeHeader ? (
                <>
                  <ChevronRightIcon
                    aria-hidden="true"
                    className="size-4 shrink-0 text-muted-foreground"
                  />
                  <span className="truncate text-sm font-medium">
                    {activeHeader.title}
                  </span>
                </>
              ) : null}
            </div>

            <div className="flex items-center gap-2">
              {activeHeader?.actions}
            </div>
          </div>
        </header>

        <SidebarInset className="min-w-0 flex-1 mb-2 mr-2 lg:w-auto lg:rounded-xl">
          <MainLayoutHeaderContext.Provider value={headerContext}>
            <Outlet/>
          </MainLayoutHeaderContext.Provider>
        </SidebarInset>
      </div>
    </SidebarProvider>
  );
}
