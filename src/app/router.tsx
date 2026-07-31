import { createBrowserRouter } from "react-router";
import MainLayout from "../layouts/MainLayout.tsx";

export const router = createBrowserRouter([
  {
    path: "/",
    Component: MainLayout,
    children: [
      {
        index: true,
        lazy: () =>
          import("../pages/dashboard/DashboardPage.tsx").then((module) => ({
            Component: module.DashboardPage,
          })),
      },
      {
        path: "system-information",
        lazy: () =>
          import("../pages/system-information/SystemInformationPage.tsx").then(
            (module) => ({
              Component: module.SystemInformationPage,
            }),
          ),
      },
      {
        path: "network-monitor",
        lazy: () =>
          import("../pages/network-monitor/NetworkMonitorPage.tsx").then(
            (module) => ({
              Component: module.NetworkMonitorPage,
            }),
          ),
      },
      {
        path: "nginx/manage",
        lazy: () =>
          import("../pages/nginx/NginxManagementPage.tsx").then((module) => ({
            Component: module.NginxManagementPage,
          })),
      },
      {
        path: "nginx/manage/instances",
        lazy: () =>
          import("../pages/nginx/NginxInstancesPage.tsx").then((module) => ({
            Component: module.NginxInstancesPage,
          })),
      },
      {
        path: "nginx/logs",
        lazy: () =>
          import("../pages/nginx/NginxLogsPage.tsx").then((module) => ({
            Component: module.NginxLogsPage,
          })),
      },
    ],
  },
]);
