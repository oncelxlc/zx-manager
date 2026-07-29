import { describe, expect, it } from "vitest";

import enUS from "./locales/en-US";
import zhCN from "./locales/zh-CN";

function leafKeys(value: unknown, prefix = ""): string[] {
  if (typeof value !== "object" || value === null) {
    return [prefix];
  }
  return Object.entries(value).flatMap(([key, child]) =>
    leafKeys(child, prefix ? `${prefix}.${key}` : key),
  );
}

describe("network monitor locales", () => {
  it("keeps the Chinese and English key sets synchronized", () => {
    expect(leafKeys(enUS.networkMonitor).sort())
      .toEqual(leafKeys(zhCN.networkMonitor).sort());
  });
});
