import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useLocation, MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { restartApplication, toastAdd } = vi.hoisted(() => ({
  restartApplication: vi.fn(),
  toastAdd: vi.fn(),
}));

vi.mock("@/components/ui/toast", () => ({
  toast: { add: toastAdd },
}));

vi.mock("src/services/tauri/application", () => ({
  restartApplication,
}));

import { SidebarProvider } from "@/components/ui/sidebar";
import { TooltipProvider } from "@/components/ui/tooltip";
import { AppSidebar } from "src/components/AppSidebar";
import {
  resetSystemInformationStore,
  useSystemInformationStore,
} from "src/stores/system-information-store";
import {
  systemInformationFixture,
  systemSummaryFixture,
} from "src/test/system-information-fixture";

function LocationProbe() {
  const location = useLocation();
  return <output aria-label="current path">{location.pathname}</output>;
}

function renderSidebar(defaultOpen = true) {
  return render(
    <MemoryRouter initialEntries={["/"]}>
      <TooltipProvider>
        <SidebarProvider defaultOpen={defaultOpen}>
          <AppSidebar />
          <LocationProbe />
        </SidebarProvider>
      </TooltipProvider>
    </MemoryRouter>,
  );
}

describe("AppSidebar system information navigation", () => {
  beforeEach(() => {
    restartApplication.mockReset();
    toastAdd.mockReset();
    resetSystemInformationStore();
    useSystemInformationStore.setState({
      summary: systemSummaryFixture,
      summaryStatus: "success",
    });
  });

  afterEach(() => {
    clearMocks();
  });

  it("opens the machine menu without navigating when the device card is clicked", async () => {
    const user = userEvent.setup();
    renderSidebar();

    await user.click(
      screen.getByRole("button", { name: "Open machine actions" }),
    );

    expect(
      await screen.findByRole("menuitem", { name: "System information" }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("current path")).toHaveTextContent("/");
  });

  it("keeps preference and machine actions usable when collapsed", async () => {
    const user = userEvent.setup();
    const { container } = renderSidebar(false);

    expect(
      container.querySelector('[data-slot="sidebar"][data-state="collapsed"]'),
    ).toHaveAttribute("data-collapsible", "icon");

    const header = container.querySelector('[data-sidebar="header"]');
    const footer = container.querySelector('[data-sidebar="footer"]');
    const sidebarContainer = container.querySelector(
      '[data-slot="sidebar-container"]',
    );
    const preferenceActions = container.querySelector(
      '[data-slot="sidebar-preference-actions"]',
    );

    expect(sidebarContainer).toHaveClass(
      "group-data-[side=left]:border-r-0",
    );
    expect(header).toHaveClass("group-data-[collapsible=icon]:p-2");
    expect(footer).toHaveClass("group-data-[collapsible=icon]:p-2");
    expect(preferenceActions).toHaveClass(
      "group-data-[collapsible=icon]:flex-col",
    );

    const languageButton = screen.getByRole("button", {
      name: "Change interface language",
    });
    const themeButton = screen.getByRole("button", {
      name: "Change theme",
    });
    const machineButton = screen.getByRole("button", {
      name: "Open machine actions",
    });

    expect(languageButton).toBeInTheDocument();
    expect(themeButton).toBeInTheDocument();
    expect(machineButton).toHaveAttribute("data-sidebar", "menu-button");
    expect(machineButton).not.toHaveClass(
      "group-data-[collapsible=icon]:hidden",
    );

    await user.click(languageButton);
    expect(await screen.findByRole("menu")).toBeInTheDocument();
    await user.keyboard("{Escape}");

    await user.click(themeButton);
    expect(await screen.findByRole("menu")).toBeInTheDocument();
    await user.keyboard("{Escape}");

    await user.click(machineButton);
    expect(
      await screen.findByRole("menuitem", { name: "System information" }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("current path")).toHaveTextContent("/");
  });

  it.each(["{Enter}", " "])(
    "opens the machine menu with %s without navigating",
    async (key) => {
      const user = userEvent.setup();
      renderSidebar();
      const button = screen.getByRole("button", {
        name: "Open machine actions",
      });

      button.focus();
      await user.keyboard(key);

      expect(
        screen.getByRole("menuitem", { name: "System information" }),
      ).toBeInTheDocument();
      expect(screen.getByLabelText("current path")).toHaveTextContent("/");
    },
  );

  it("navigates only after selecting System information from the menu", async () => {
    const user = userEvent.setup();
    renderSidebar();

    await user.click(
      screen.getByRole("button", { name: "Open machine actions" }),
    );
    await user.click(
      await screen.findByRole("menuitem", { name: "System information" }),
    );

    expect(screen.getByLabelText("current path")).toHaveTextContent(
      "/system-information",
    );
  });

  it("copies only the allowlisted diagnostic report", async () => {
    const user = userEvent.setup();
    let clipboardText = "";
    useSystemInformationStore.setState({
      information: systemInformationFixture,
      informationStatus: "success",
    });
    mockIPC((command, payload) => {
      if (command === "plugin:clipboard-manager|write_text") {
        clipboardText = String(
          payload && !Array.isArray(payload) && "text" in payload
            ? payload.text
            : "",
        );
        return null;
      }
      throw new Error(`Unexpected command: ${command}`);
    });
    renderSidebar();

    await user.click(
      screen.getByRole("button", { name: "Open machine actions" }),
    );
    await user.click(
      await screen.findByRole("menuitem", { name: "Copy diagnostics" }),
    );

    expect(clipboardText).toContain("\"schemaVersion\": 1");
    expect(clipboardText).not.toContain("private-host");
    expect(clipboardText).not.toContain("C:\\\\private");
    expect(clipboardText).not.toContain("private-driver");
  });

  it("requires confirmation before restarting the application", async () => {
    const user = userEvent.setup();
    const restartPending = new Promise<void>(() => undefined);
    restartApplication.mockReturnValue(restartPending);
    renderSidebar();

    await user.click(
      screen.getByRole("button", { name: "Open machine actions" }),
    );
    await user.click(
      await screen.findByRole("menuitem", { name: "Restart app" }),
    );

    expect(
      await screen.findByRole("heading", { name: "Restart ZxManager?" }),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(restartApplication).not.toHaveBeenCalled();
    expect(
      screen.queryByRole("heading", { name: "Restart ZxManager?" }),
    ).not.toBeInTheDocument();

    await user.click(
      screen.getByRole("button", { name: "Open machine actions" }),
    );
    await user.click(
      await screen.findByRole("menuitem", { name: "Restart app" }),
    );
    const restartButton = await screen.findByRole("button", {
      name: "Restart app",
    });
    await user.click(restartButton);

    expect(restartApplication).toHaveBeenCalledTimes(1);
    expect(restartButton).toBeDisabled();
  });

  it("shows an error and keeps the confirmation available when restart fails", async () => {
    const user = userEvent.setup();
    restartApplication.mockRejectedValueOnce(
      new Error("The Process plugin is unavailable."),
    );
    renderSidebar();

    await user.click(
      screen.getByRole("button", { name: "Open machine actions" }),
    );
    await user.click(
      await screen.findByRole("menuitem", { name: "Restart app" }),
    );
    await user.click(
      await screen.findByRole("button", { name: "Restart app" }),
    );

    await waitFor(() => {
      expect(toastAdd).toHaveBeenLastCalledWith({
        title: "Unable to restart app",
        description: "The Process plugin is unavailable.",
        type: "error",
      });
    });
    expect(
      screen.getByRole("heading", { name: "Restart ZxManager?" }),
    ).toBeInTheDocument();
  });
});
