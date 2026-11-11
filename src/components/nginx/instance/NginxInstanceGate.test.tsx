import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, describe, expect, it } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { NginxInstanceGate } from "./NginxInstanceGate";
import { resetNginxStore, useNginxStore } from "src/stores/nginx-store";
import type { NginxInstance } from "src/types/nginx";

function instance(lifecycleState: NginxInstance["lifecycleState"]): NginxInstance {
  return {
    id: "one",
    name: "Nginx",
    kind: "external",
    rootPath: "C:\\nginx",
    binaryPath: "C:\\nginx\\nginx.exe",
    configPath: "C:\\nginx\\conf\\nginx.conf",
    authorizedRoots: ["C:\\nginx"],
    authorizationLevel: "readOnly",
    version: "1.28.0",
    configureArguments: [],
    binaryFingerprint: "fingerprint",
    providerIdentity: { provider: "portable", externalId: null },
    controlBackend: "portable",
    lifecycleState,
    runtimeStatus: "stopped",
    capabilities: {
      canRead: true,
      canEdit: false,
      canControl: false,
      canUnregister: true,
    },
    createdAt: "2026-08-06T00:00:00Z",
    updatedAt: "2026-08-06T00:00:00Z",
  };
}

function renderGate() {
  return render(
    <MemoryRouter>
      <TooltipProvider>
        <NginxInstanceGate><p>ready content</p></NginxInstanceGate>
      </TooltipProvider>
    </MemoryRouter>,
  );
}

describe("NginxInstanceGate", () => {
  afterEach(() => resetNginxStore());

  it("shows real selection and disabled installation for an empty registry", () => {
    useNginxStore.setState({
      registryState: { status: "empty", instance: null, migrationCandidates: [] },
      loadStatus: "success",
    });
    renderGate();
    expect(screen.getByRole("button", { name: "Select existing Nginx" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Install Nginx" })).toBeDisabled();
    expect(screen.queryByText("ready content")).not.toBeInTheDocument();
  });

  it("blocks content when the registered binary is invalid", () => {
    const invalid = instance("missing");
    useNginxStore.setState({
      registryState: { status: "ready", instance: invalid, migrationCandidates: [] },
      instance: invalid,
      loadStatus: "success",
    });
    renderGate();
    expect(screen.getByText("Nginx instance is invalid")).toBeInTheDocument();
    expect(screen.queryByText("ready content")).not.toBeInTheDocument();
  });

  it("mounts page content only for an available singleton", () => {
    const ready = instance("available");
    useNginxStore.setState({
      registryState: { status: "ready", instance: ready, migrationCandidates: [] },
      instance: ready,
      loadStatus: "success",
    });
    renderGate();
    expect(screen.getByText("ready content")).toBeInTheDocument();
  });
});
