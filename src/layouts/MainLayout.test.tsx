import { useMemo } from "react";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  MemoryRouter,
  Route,
  Routes,
  useNavigate,
} from "react-router";
import { describe, expect, it, vi } from "vitest";

import MainLayout, { useMainLayoutHeader } from "src/layouts/MainLayout";

vi.mock("src/components/AppSidebar", () => ({
  AppSidebar: () => <aside data-slot="app-sidebar" />,
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
      "lg:m-2",
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
