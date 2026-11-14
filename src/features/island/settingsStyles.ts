import * as stylex from "@stylexjs/stylex";
import defaults from "./defaults.json";

export const lightSurface = "#f5f5f4";
// Keep these preview colors in sync with the native island renderer.
export const islandPalette = {
  darkSurface: defaults.appearance.background,
  darkBorder: defaults.appearance.border,
  tones: {
    foreground: { dark: "#ffffff", light: "#111111" },
    battery: { dark: "#dcdce0", light: "#38383c" },
    date: { dark: "#d2d2d6", light: "#37373a" },
    dim: { dark: "#b2b2ba", light: "#494951" },
    control: { dark: "#58585c", light: "#76767c" },
    elapsed: { dark: "#4e4e52", light: "#77777b" },
    track: { dark: "#242427", light: "#ceced2" },
    lyricsButton: { dark: "#161616", light: "#e4e4e4" },
    lyricsBorder: { dark: "#2f2f2f", light: "#bfbfbf" },
  },
  shadow: "#000000",
  charging: "#21eb54",
  lowBattery: "#f23046",
  emptyCover: "#19191b",
  coverNote: "#ffffffeb",
  emptyCoverNote: "#6e6e72",
  playbackTrack: "#5a5a5a",
  coverGradients: [
    ["#ff5e62", "#ff9966"], ["#56ccf2", "#2f80ed"], ["#a855f7", "#ec4899"],
    ["#34d399", "#1078c8"], ["#fbbf24", "#ef4444"],
  ],
} as const;

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
