import * as stylex from "@stylexjs/stylex";

export const ui = stylex.create({
  stack: { display: "flex", flexDirection: "column", gap: 12, minWidth: 0 },
  row: { display: "flex", alignItems: "center", flexWrap: "wrap", gap: "var(--space)" },
  grid: { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 240px), 1fr))", gap: 12 },
  toolbar: { display: "flex", flexWrap: "wrap", alignItems: "center", gap: "var(--space)", minWidth: 0 },
  muted: { color: "var(--muted-foreground)", fontSize: 13 },
  mono: { fontFamily: "var(--mono)", fontVariantNumeric: "tabular-nums", overflowWrap: "anywhere" },
  link: { color: "var(--primary)", cursor: "pointer", textDecoration: { default: "none", ":hover": "underline" } },
});
