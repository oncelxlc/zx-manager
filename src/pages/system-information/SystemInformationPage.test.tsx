import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import MainLayout from "src/layouts/MainLayout";
import { SystemInformationPage } from "src/pages/system-information/SystemInformationPage";
import {
  resetSystemInformationStore,
  useSystemInformationStore,
} from "src/stores/system-information-store";
import { systemInformationFixture } from "src/test/system-information-fixture";

vi.mock("src/components/AppSidebar", () => ({
  AppSidebar: () => null,
}));

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/"]}>
      <Routes>
        <Route element={<MainLayout />}>
          <Route index element={<SystemInformationPage />} />
        </Route>
      </Routes>
    </MemoryRouter>,
  );
}

describe("SystemInformationPage", () => {
  beforeEach(() => {
    resetSystemInformationStore();
  });

  afterEach(() => {
    clearMocks();
  });

  it("shows a layout-stable skeleton during the first load", async () => {
    mockIPC(() => new Promise(() => undefined));
    renderPage();

    expect(
      await screen.findByRole("heading", { name: "System Information" }),
    ).toBeInTheDocument();
    expect(document.querySelector("[aria-busy='true']")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Refreshing" })).toBeDisabled();
  });

  it("shows a retryable empty state when the first load fails", async () => {
    mockIPC(() => {
      throw {
        code: "systemInformationCollectionFailed",
        message: "private backend error",
      };
    });
    renderPage();

    expect(
      await screen.findByText("Unable to load system information"),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Retry" })).toBeEnabled();
    expect(screen.getByText("System information collection failed."))
      .toBeInTheDocument();
  });

  it("renders multiple GPUs and unavailable fields", () => {
    useSystemInformationStore.setState({
      information: systemInformationFixture,
      informationStatus: "success",
      summary: systemInformationFixture.summary,
      summaryStatus: "success",
    });
    renderPage();

    expect(screen.getByText("Integrated Example")).toBeInTheDocument();
    expect(screen.getByText("Discrete Example")).toBeInTheDocument();
    expect(screen.getAllByText("Unavailable").length).toBeGreaterThan(0);
    expect(screen.getByText("C:\\private")).toBeInTheDocument();
  });

  it("keeps old data visible and disables refresh while refreshing", async () => {
    useSystemInformationStore.setState({
      information: systemInformationFixture,
      informationStatus: "loading",
      summary: systemInformationFixture.summary,
      summaryStatus: "success",
    });
    renderPage();

    expect(screen.getAllByText("Example CPU").length).toBeGreaterThan(0);
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Refreshing" })).toBeDisabled();
    });
  });
});
