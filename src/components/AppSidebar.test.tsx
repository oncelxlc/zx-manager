import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useLocation, MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

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

function renderSidebar() {
  return render(
    <MemoryRouter initialEntries={["/"]}>
      <TooltipProvider>
        <SidebarProvider>
          <AppSidebar />
          <LocationProbe />
        </SidebarProvider>
      </TooltipProvider>
    </MemoryRouter>,
  );
}

describe("AppSidebar system information navigation", () => {
  beforeEach(() => {
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

    expect(screen.getByText("Windows 11 Pro · x86_64")).toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "Open machine actions" }),
    );

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
});
