import { createBrowserRouter, redirect } from "react-router";
import MainLayout from "../layouts/MainLayout.tsx";

function redirectWithSearch(pathname: string) {
  return ({ request }: { request: Request }) => {
    const url = new URL(request.url);
    return redirect(`${pathname}${url.search}`);
  };
}

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
        path: "nginx",
        loader: redirectWithSearch("/nginx/overview"),
      },
      {
        path: "nginx/overview",
        lazy: () =>
          import("../pages/nginx/NginxOverviewPage.tsx").then((module) => ({
            Component: module.NginxOverviewPage,
          })),
      },
      {
        path: "nginx/runtime",
        lazy: () =>
          import("../pages/nginx/NginxRuntimePage.tsx").then((module) => ({
            Component: module.NginxRuntimePage,
          })),
      },
      {
        path: "nginx/configuration",
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
      { path: "nginx/manage", loader: redirectWithSearch("/nginx/overview") },
      { path: "nginx/manage/instances", loader: redirectWithSearch("/nginx/overview") },
      { path: "nginx/manage/sites", loader: redirectWithSearch("/nginx/configuration") },
      { path: "nginx/manage/configuration", loader: redirectWithSearch("/nginx/configuration") },
    ],
  },
]);
