import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { ServiceTable } from "./ServiceTable";
import type { LocalService } from "src/types/service";

function createServices(count: number): LocalService[] {
  return Array.from({ length: count }, (_, index) => ({
    id: `service-${index}`,
    name: `Service ${index}`,
    descriptionKey: "nginx",
    type: "nginx",
    status: "running",
    version: "1.0.0",
    port: 8000 + index,
    cpu: 1,
    memory: "10 MB",
  }));
}

describe("ServiceTable", () => {
  it("renders a bounded service-row window while preserving select-all", async () => {
    const user = userEvent.setup();
    const services = createServices(500);
    const onSelectionChange = vi.fn();

    render(
      <ServiceTable
        onLocalAction={vi.fn()}
        onOperation={vi.fn(async () => undefined)}
        onRemove={vi.fn()}
        onSelectionChange={onSelectionChange}
        pendingIds={new Set()}
        selectedIds={new Set()}
        services={services}
        visibleColumns={new Set(["type", "status", "version", "port", "cpu", "memory", "actions"])}
      />,
    );

    expect(screen.getByText("Service 0")).toBeInTheDocument();
    expect(screen.queryByText("Service 499")).not.toBeInTheDocument();
    expect(screen.getAllByRole("row").length).toBeLessThan(30);

    await user.click(screen.getByRole("checkbox", { name: "Select all services" }));
    expect(onSelectionChange).toHaveBeenCalledWith(new Set(services.map((service) => service.id)));
  });
});
