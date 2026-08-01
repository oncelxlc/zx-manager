import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import {
  getPaginationItems,
  NetworkTablePagination,
} from "./NetworkTablePagination";

describe("NetworkTablePagination", () => {
  it("builds compact numbered pagination with ellipses", () => {
    expect(getPaginationItems(0, 10)).toEqual([
      0,
      1,
      2,
      3,
      "endEllipsis",
      9,
    ]);
    expect(getPaginationItems(5, 10)).toEqual([
      0,
      "startEllipsis",
      4,
      5,
      6,
      "endEllipsis",
      9,
    ]);
  });

  it("changes pages and page size through accessible controls", async () => {
    const user = userEvent.setup();
    const onPageChange = vi.fn();
    const onPageSizeChange = vi.fn();
    render(
      <NetworkTablePagination
        onPageChange={onPageChange}
        onPageSizeChange={onPageSizeChange}
        pageIndex={0}
        pageSize={10}
        totalCount={75}
      />,
    );

    expect(screen.getByText("1–10 of 75")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Go to page 2" }));
    expect(onPageChange).toHaveBeenCalledWith(1);

    await user.click(screen.getByRole("combobox", { name: "Rows per page" }));
    await user.click(await screen.findByRole("option", { name: "20" }));
    expect(onPageSizeChange).toHaveBeenCalledWith(20);
  });
});
