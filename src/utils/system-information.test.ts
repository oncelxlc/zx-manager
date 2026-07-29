import { describe, expect, it } from "vitest";

import { systemInformationFixture } from "src/test/system-information-fixture";
import {
  createDiagnosticReport,
  formatBytes,
  formatCollectedAt,
  formatDuration,
  formatPercent,
  serializeDiagnosticReport,
} from "src/utils/system-information";

describe("system information formatters", () => {
  it("formats bytes, percentages, durations, and unavailable values", () => {
    expect(formatBytes(1_073_741_824, "en-US", "N/A")).toBe("1 GB");
    expect(formatBytes(null, "en-US", "N/A")).toBe("N/A");
    expect(formatPercent(24.55, "en-US", "N/A")).toBe("24.6%");
    expect(formatPercent(Number.NaN, "en-US", "N/A")).toBe("N/A");
    expect(
      formatDuration(90_000, "en-US", "N/A", {
        day: "d",
        hour: "h",
        minute: "min",
      }),
    ).toBe("1 d 1 h");
    expect(formatCollectedAt("invalid", "en-US", "N/A")).toBe("N/A");
  });
});

describe("diagnostic allowlist", () => {
  it("excludes host names, mount points, and driver details", () => {
    const report = createDiagnosticReport(systemInformationFixture);
    const serialized = serializeDiagnosticReport(systemInformationFixture);

    expect(report.system).not.toHaveProperty("hostName");
    expect(report.disks[0]).not.toHaveProperty("mountPoint");
    expect(report.gpus[0]).not.toHaveProperty("driver");
    expect(report.gpus[0]).not.toHaveProperty("driverInfo");
    expect(serialized).not.toContain("private-host");
    expect(serialized).not.toContain("C:\\\\private");
    expect(serialized).not.toContain("private-driver");
  });
});
