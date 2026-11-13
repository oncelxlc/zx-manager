import * as stylex from "@stylexjs/stylex";
import { Cable, Terminal } from "lucide-react";
import { Suspense, useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore } from "react";
import { useDispatch, useSelector } from "react-redux";
import { Link, Outlet, useLocation } from "react-router";
import { version } from "../../package.json";
import { Button } from "../components/ui/button";
import { Separator } from "../components/ui/separator";
import { SidebarInset, SidebarProvider, SidebarTrigger, useSidebar } from "../components/ui/sidebar";
import { TooltipProvider } from "../components/ui/tooltip";
import type { RootState } from "../store";
import { AppSidebar } from "./AppSidebar";
import { DetailPanelHost } from "./WorkbenchDialog";
import { persistPreferences, setSidebar } from "./preferencesSlice";
import { shell } from "./shellStyles";

const sidebarSize = { "--sidebar-width": "232px", "--sidebar-width-icon": "64px", minHeight: 0 };

function watchWidth(update: () => void) {
  window.addEventListener("resize", update);
  return () => window.removeEventListener("resize", update);
}
function windowSize() {
  if (window.innerWidth < 1100) return "small";
  return window.innerWidth < 1440 ? "desktop" : "wide";
}

export function AppShell() {
  const preferences = useSelector((state: RootState) => state.preferences);
  const dispatch = useDispatch();
  const size = useSyncExternalStore(watchWidth, windowSize, () => "desktop");
  const expanded = preferences.sidebar === "expanded" || (preferences.sidebar === "auto" && size !== "small");

  useEffect(() => { persistPreferences(preferences); }, [preferences]);
  useLayoutEffect(() => {
    const query = window.matchMedia?.("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark = preferences.theme === "dark" || (preferences.theme === "system" && Boolean(query?.matches));
      document.documentElement.classList.toggle("dark", dark);
      document.documentElement.dataset.theme = dark ? "dark" : "light";
    };
    apply();
    if (preferences.theme !== "system" || !query) return;
    query.addEventListener("change", apply);
    return () => query.removeEventListener("change", apply);
  }, [preferences.theme]);

  return <TooltipProvider>
    <div {...stylex.props(shell.frame)}>
      <SidebarProvider {...stylex.props(shell.workspace)} style={sidebarSize} open={expanded}
        onOpenChange={open => dispatch(setSidebar(open ? "expanded" : "collapsed"))}>
        <AppSidebar />
        <Workspace wide={size === "wide"} />
      </SidebarProvider>
      <StatusBar />
    </div>
  </TooltipProvider>;
}

function Workspace({ wide }: { wide: boolean }) {
  const { isMobile, openMobile, setOpenMobile } = useSidebar();
  const [detailHost, setDetailHost] = useState<HTMLDivElement | null>(null);
  const content = useRef<HTMLElement>(null);
  const location = useLocation();
  useEffect(() => { setOpenMobile(false); }, [isMobile, location.key, setOpenMobile]);
  useLayoutEffect(() => { if (content.current) content.current.scrollTop = 0; }, [location.pathname]);
  return <div className="flex min-h-0 min-w-0 flex-1 flex-col">
    <Button variant="outline" size="sm" className="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50"
      onClick={() => content.current?.focus()}>跳到主内容</Button>
    {isMobile && <header {...stylex.props(shell.mobileBar)}>
      <SidebarTrigger aria-label="打开导航" aria-expanded={openMobile} title="打开导航" />
      <img src="/icon.png" width={24} height={24} alt="" /><strong>zx-once</strong>
    </header>}
    <div className="flex min-h-0 flex-1">
      <DetailPanelHost.Provider value={wide ? detailHost : null}>
        <SidebarInset ref={content} id="workbench-content" tabIndex={-1} {...stylex.props(shell.main)}>
          <Suspense fallback={<p role="status">正在打开页面…</p>}><Outlet /></Suspense>
        </SidebarInset>
      </DetailPanelHost.Provider>
      <div ref={setDetailHost} className="shrink-0 overflow-auto border-l bg-card empty:hidden" />
    </div>
  </div>;
}

function StatusBar() {
  const connected = useSelector((state: RootState) => state.remote.connections.filter(item => item.connected).length);
  return <footer aria-label="工作台状态" {...stylex.props(shell.status)}>
    <span {...stylex.props(shell.statusItem)}><Terminal size={13} aria-hidden="true" />本地工作区</span>
    <Separator orientation="vertical" className="h-3" />
    <Button render={<Link to="/remote?tab=connections" />} nativeButton={false} variant="link" size="xs">
      <Cable aria-hidden="true" />演示连接 · {connected}
    </Button>
    <span {...stylex.props(shell.version)}>v{version}</span>
  </footer>;
}
