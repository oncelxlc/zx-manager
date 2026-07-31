import { useMemo } from "react";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  MemoryRouter,
  Route,
  Routes,
  useNavigate,
} from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import MainLayout, { useMainLayoutHeader } from "src/layouts/MainLayout";

const { completeStartup, restoreStartupBackgroundTasks } = vi.hoisted(() => ({
  completeStartup: vi.fn(() => Promise.resolve()),
  restoreStartupBackgroundTasks: vi.fn(() => Promise.resolve()),
}));

vi.mock("src/components/AppSidebar", () => ({
  AppSidebar: () => <aside data-slot="app-sidebar" />,
}));

vi.mock("src/services/tauri/startup", () => ({
  completeStartup,
}));

vi.mock("src/services/tauri/startup-background", () => ({
  restoreStartupBackgroundTasks,
}));

function DashboardFixture() {
  const navigate = useNavigate();
  const actions = useMemo(
    () => <button type="button">Dashboard action</button>,
    [],
  );

  useMainLayoutHeader({
    actions,
    title: "Dashboard",
  });

  return (
    <button
      onClick={() => void navigate("/system-information")}
      type="button"
    >
      Open system information
    </button>
  );
}

function SystemInformationFixture() {
  const actions = useMemo(
    () => <button type="button">System information action</button>,
    [],
  );

  useMainLayoutHeader({
    actions,
    title: "System Information",
  });

  return <p>System information content</p>;
}

function renderLayout() {
  return render(
    <MemoryRouter initialEntries={["/"]}>
      <Routes>
        <Route element={<MainLayout />}>
          <Route index element={<DashboardFixture />} />
          <Route
            element={<SystemInformationFixture />}
            path="system-information"
          />
        </Route>
      </Routes>
    </MemoryRouter>,
  );
}

describe("MainLayout", () => {
  beforeEach(() => {
    completeStartup.mockReset().mockResolvedValue(undefined);
    restoreStartupBackgroundTasks.mockReset().mockResolvedValue(undefined);
  });

  it("hands off from the splashscreen after the layout mounts", async () => {
    renderLayout();

    await waitFor(() => {
      expect(completeStartup).toHaveBeenCalledTimes(1);
      expect(restoreStartupBackgroundTasks).toHaveBeenCalledTimes(1);
    });
  });

  it("starts background restoration only after the native handoff succeeds", async () => {
    let resolveHandoff!: () => void;
    completeStartup.mockReturnValueOnce(
      new Promise<void>((resolve) => { resolveHandoff = resolve; }),
    );
    renderLayout();

    await waitFor(() => expect(completeStartup).toHaveBeenCalledOnce());
    expect(restoreStartupBackgroundTasks).not.toHaveBeenCalled();
    resolveHandoff();
    await waitFor(() => {
      expect(restoreStartupBackgroundTasks).toHaveBeenCalledOnce();
    });
  });

  it("merges the header and sidebar layer around an inset content surface", async () => {
    const { container } = renderLayout();

    await screen.findByText("Dashboard action");

    expect(
      container.querySelector('[data-slot="sidebar-wrapper"]'),
    ).toHaveClass("bg-sidebar");

    const header = container.querySelector('[data-slot="main-layout-header"]');
    expect(header).toHaveClass("bg-sidebar");
    expect(header).not.toHaveClass("border-b");

    const content = container.querySelector('[data-slot="sidebar-inset"]');
    expect(content).toHaveClass(
      "bg-background",
      "mb-2",
      "mr-2",
      "lg:w-auto",
      "lg:rounded-xl",
    );
  });

  it("replaces route header content without retaining the previous actions", async () => {
    const user = userEvent.setup();
    renderLayout();

    expect(await screen.findByText("Dashboard")).toBeInTheDocument();
    expect(screen.getByText("Dashboard action")).toBeInTheDocument();

    await user.click(
      screen.getByRole("button", { name: "Open system information" }),
    );

    expect(
      await screen.findByText("System Information"),
    ).toBeInTheDocument();
    expect(screen.getByText("System information action")).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.queryByText("Dashboard action")).not.toBeInTheDocument();
    });
  });
});
