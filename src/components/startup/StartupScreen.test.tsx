import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { StartupScreen } from "./StartupScreen";

describe("StartupScreen", () => {
  it("renders the full-screen branded loading state", () => {
    render(<StartupScreen />);

    expect(screen.getByTestId("startup-screen")).toHaveAttribute("aria-busy", "true");
    expect(screen.getByRole("heading", { name: "ZxManager" })).toBeVisible();
    expect(document.querySelector('[data-slot="spinner"]')).toHaveClass("size-6");
  });
});
