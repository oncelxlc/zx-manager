import { createBrowserRouter } from "react-router";
import MainLayout from "../layouts/MainLayout.tsx";

export const router = createBrowserRouter([
  {
    path: "/",
    Component: MainLayout,
    children: [
      {
        index: true,
        lazy: () => import("../pages/index/Index.tsx").then((module) => ({Component: module.IndexPage})),
      },
    ],
  },
]);