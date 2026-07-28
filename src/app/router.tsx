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
    ],
  },
]);
