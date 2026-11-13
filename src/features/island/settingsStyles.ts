import * as stylex from "@stylexjs/stylex";
import defaults from "./defaults.json";

export const lightSurface = "#f5f5f4";
export function isDefaultDarkSurface(color: string) {
  const value = color.toLowerCase();
  return value === defaults.appearance.background || value === "#101214" || value === "#202326";
}

export const ui = stylex.create({
  layout: { display: "grid", gridTemplateColumns: { default: "minmax(148px, 176px) minmax(0, 1fr)", "@media (max-width: 680px)": "minmax(0, 1fr)" },
    alignItems: "start", gap: 16 },
  nav: { display: "flex", flexDirection: { default: "column", "@media (max-width: 680px)": "row" },
    gap: 3, flexWrap: "wrap", padding: 6 },
  actions: { display: "flex", alignItems: "center", flexWrap: "wrap", gap: 8 },
});
