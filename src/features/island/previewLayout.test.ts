import { describe, expect, it } from "vitest";
import defaults from "./defaults.json";
import { notchOutline, previewLayout } from "./previewLayout";
import type { IslandConfig } from "./types";

const config = defaults as IslandConfig;

describe("Daozai preview dimensions in unscaled DIPs", () => {
  it.each([
    [false, false, 120], [true, false, 152], [false, true, 146], [true, true, 178],
  ] as const)("fits battery=%s, music=%s without counting the shoulders as content", (battery, music, bodyWidth) => {
    const bar = previewLayout(config, "compact", battery, music);
    expect(bar).toMatchObject({ bodyWidth, width: bodyWidth + 10, height: 30, bodyX: 5, radius: 12.5 });
  });

  it("removes hidden slots and keeps the clock-only bar at the source minimum", () => {
    const appearance = { ...config.appearance, pet: false, showBattery: false };
    expect(previewLayout({ ...config, appearance }, "compact", true, false).bodyWidth).toBe(96);
  });

  it.each(["centerLeft", "centerRight"] as const)("keeps the music page horizontal while docked at %s", anchor => {
    const side = { ...config, placement: { ...config.placement, anchor, offsetY: -50 } };
    expect(previewLayout(side, "compact", true, true)).toMatchObject({ width: 36, bodyHeight: 84, height: 94, bodyY: 5, offset: -0.5 });
    expect(previewLayout(side, "expanded", true, true)).toMatchObject({ width: 419, height: 175.2, bodyY: 17.6, bodyHeight: 140 });
    expect(previewLayout(side, "edgePeek", true, true)).toMatchObject({ width: 4.5, height: 44, wing: 0 });
  });

  it("uses the full 419-DIP music canvas and puts the flare outside it", () => {
    const panel = previewLayout(config, "expanded", true, true);
    expect(panel).toMatchObject({ width: 454.2, height: 140, bodyX: 17.6, bodyWidth: 419, radius: 33 });
    expect(notchOutline(panel)).toContain("M454.200 0.000");
    expect(notchOutline(panel)).toContain("436.600 16.000");
    expect(notchOutline(panel)).toContain("403.600 140.000");
  });

  it("keeps pill spacing and the size multiplier independent of the design coordinates", () => {
    const appearance: IslandConfig["appearance"] = { ...config.appearance, shape: "pill", minimal: true, islandSize: 140 };
    const compact = previewLayout({ ...config, appearance }, "compact", true, false);
    expect(compact).toMatchObject({ width: 44, height: 4.5, radius: 2.25, wing: 0, gap: 6, scale: 1.4 });
    expect(previewLayout({ ...config, appearance }, "expanded", false, false)).toMatchObject({ width: 419, height: 140, radius: 26 });
  });
});
