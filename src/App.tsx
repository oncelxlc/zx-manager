import { lazy } from "react";
import { Navigate, Route, Routes } from "react-router";
import { WelcomePage } from "./WelcomePage";
import { AppShell } from "./app/AppShell";
import { ToastProvider } from "./app/toasts";
import { IslandSettingsPage } from "./features/island/IslandSettingsPage";

const ProjectsPage = lazy(() => import("./features/projects/ProjectsPage").then(module => ({ default: module.ProjectsPage })));
const EnvironmentPage = lazy(() => import("./features/environment/EnvironmentPage").then(module => ({ default: module.EnvironmentPage })));
const GitPage = lazy(() => import("./features/git/GitPage").then(module => ({ default: module.GitPage })));
const GitHubPage = lazy(() => import("./features/github/GitHubPage").then(module => ({ default: module.GitHubPage })));
const RemotePage = lazy(() => import("./features/remote/RemotePage").then(module => ({ default: module.RemotePage })));
const ToolboxPage = lazy(() => import("./features/toolbox/ToolboxPage").then(module => ({ default: module.ToolboxPage })));
const SettingsPage = lazy(() => import("./app/SettingsPage").then(module => ({ default: module.SettingsPage })));

export default function App() {
  return (
    <ToastProvider>
      <Routes>
        <Route path="/" element={<AppShell />}>
          <Route index element={<WelcomePage />} />
          <Route path="projects" element={<ProjectsPage />} />
          <Route path="environment" element={<EnvironmentPage />} />
          <Route path="git" element={<GitPage />} />
          <Route path="github" element={<GitHubPage />} />
          <Route path="remote" element={<RemotePage />} />
          <Route path="toolbox" element={<ToolboxPage />} />
          <Route path="settings" element={<SettingsPage />} />
          <Route path="settings/appearance" element={<SettingsPage />} />
          <Route path="settings/navigation" element={<SettingsPage />} />
          <Route path="settings/island" element={<IslandSettingsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </ToastProvider>
  );
}
