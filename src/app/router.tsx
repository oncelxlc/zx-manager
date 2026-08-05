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
        path: "nginx/manage/sites",
        lazy: () =>
          import("../pages/nginx/NginxSitesPage.tsx").then((module) => ({
            Component: module.NginxSitesPage,
          })),
      },
      {
        path: "nginx/manage/configuration",
        lazy: () =>
          import("../pages/nginx/NginxConfigurationPage.tsx").then((module) => ({
            Component: module.NginxConfigurationPage,
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
