import type { IslandConfig } from "./types";

export type PreviewDock = "top" | "left" | "right";
export type PreviewState = "compact" | "expanded" | "edgePeek" | "hidden";

export function previewLayout(config: IslandConfig, state: PreviewState, battery: boolean, music: boolean) {
  const { appearance, placement } = config;
  const dock: PreviewDock = placement.anchor === "centerLeft" ? "left" : placement.anchor === "centerRight" ? "right" : "top";
  const side = dock !== "top";
  const expanded = state === "expanded";
  const minimal = !expanded && (appearance.minimal || state === "edgePeek");
  const pill = appearance.shape === "pill";
  const scale = Number.isFinite(appearance.islandSize) ? Math.max(80, Math.min(140, appearance.islandSize)) / 100 : 1;
  const showBattery = appearance.showBattery && battery;
  const compactWidth = Math.max(96, 76 + (appearance.pet ? 44 : 0) + (showBattery ? 32 : 0) + (music ? 26 : 0));
  const sideHeight = 46 + (showBattery ? 20 : 0) + (music ? 18 : 0);
  const bodyWidth = expanded ? 419 : side ? (minimal ? 4.5 : 36) : (minimal ? 44 : compactWidth);
  const bodyHeight = expanded ? 140 : side ? (minimal ? 44 : sideHeight) : (minimal ? 4.5 : 30);
  const wing = minimal || pill ? 0 : expanded ? 17.6 : 5;
  const radius = minimal || pill ? Math.min(bodyWidth / 2, bodyHeight / 2, expanded ? 26 : Infinity) : expanded ? 33 : 12.5;
  return { dock, side, expanded, minimal, pill, scale, bodyWidth, bodyHeight, wing, radius,
    width: bodyWidth + (side ? 0 : 2 * wing), height: bodyHeight + (side ? 2 * wing : 0),
    bodyX: side ? 0 : wing, bodyY: side ? wing : 0, gap: pill ? 6 : 0,
    offset: Math.max(-1, Math.min(1, ((side ? placement.offsetY : placement.offsetX) || 0) / 100)) };
}

export type PreviewLayout = ReturnType<typeof previewLayout>;

// The upstream outline samples the shoulders at 14 intervals and the lower circular corners at 20.
export function notchOutline(layout: PreviewLayout, grow = 0) {
  const { side, dock, bodyWidth, bodyHeight, wing, expanded } = layout;
  const length = (side ? bodyHeight : bodyWidth) + 1.2 * grow;
  const depth = (side ? bodyWidth : bodyHeight) + grow;
  const radius = Math.min(layout.radius + grow * 0.8, depth / 2, length / 2);
  const shoulder = Math.min(expanded ? 16 : 8, depth / 2);
  const rows: [number, number][] = [];
  for (let i = 0; i <= 14; i++) {
    const t = i / 14;
    rows.push([wing * (1 - (1 - t) ** 2.4), shoulder * t]);
  }
  if (depth - radius > shoulder) rows.push([wing, depth - radius]);
  for (let i = 1; i <= 20; i++) {
    const angle = i / 20 * Math.PI / 2;
    rows.push([wing + radius * (1 - Math.cos(angle)), depth - radius * (1 - Math.sin(angle))]);
  }
  const toScreen = (along: number, away: number) => {
    const x = along - grow * 0.6;
    return dock === "left" ? [away, x] : dock === "right" ? [bodyWidth - away, x] : [x, away];
  };
  const points = [...rows.map(([x, y]) => toScreen(length + 2 * wing - x, y)), ...rows.reverse().map(([x, y]) => toScreen(x, y))];
  return "M" + points.map(([x, y]) => `${x.toFixed(3)} ${y.toFixed(3)}`).join(" L") + " Z";
}
