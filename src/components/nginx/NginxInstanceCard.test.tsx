import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { NginxInstanceCard } from "./NginxInstanceCard";
import type { NginxInstance } from "src/types/nginx";

function instance(runtimeStatus: NginxInstance["runtimeStatus"]): NginxInstance {
  return {
    id: "nginx-1",
    name: "Nginx",
    kind: "external",
    rootPath: "C:\\nginx",
    binaryPath: "C:\\nginx\\nginx.exe",
    configPath: "C:\\nginx\\conf\\nginx.conf",
    authorizedRoots: ["C:\\nginx"],
    authorizationLevel: "full",
    version: "1.28.0",
    configureArguments: [],
    binaryFingerprint: "hash",
    providerIdentity: { provider: "portable", externalId: null },
    controlBackend: "portable",
    lifecycleState: "available",
    runtimeStatus,
    capabilities: { canRead: true, canEdit: true, canControl: true, canUnregister: true },
    createdAt: "2026-08-01T00:00:00Z",
    updatedAt: "2026-08-01T00:00:00Z",
  };
}

describe("NginxInstanceCard", () => {
  it("offers start only for a stopped singleton", async () => {
    const user = userEvent.setup();
    const onControl = vi.fn();
    render(
      <NginxInstanceCard
        busy={false}
        instance={instance("stopped")}
        observedAt={null}
        onControl={onControl}
        onRefresh={vi.fn()}
        onUnregister={vi.fn()}
        operationPhase={null}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Start" }));
    expect(onControl).toHaveBeenCalledWith("start");
  });

  it("blocks the primary action when runtime identity conflicts", () => {
    render(
      <NginxInstanceCard
        busy={false}
        instance={instance("conflict")}
        observedAt={null}
        onControl={vi.fn()}
        onRefresh={vi.fn()}
        onUnregister={vi.fn()}
        operationPhase={null}
      />,
    );
    expect(screen.getByRole("button", { name: "Start" })).toBeDisabled();
  });
});
