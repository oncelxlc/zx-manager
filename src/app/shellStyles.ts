import * as stylex from "@stylexjs/stylex";

export const shell = stylex.create({
  frame: { height: "100dvh", display: "flex", flexDirection: "column", overflow: "hidden",
    backgroundColor: "var(--workbench-background)", color: "var(--workbench-foreground)" },
  workspace: { flex: "1 1 0", minHeight: 0, overflow: "hidden" },
  main: {
    minHeight: 0, minWidth: 0, overflow: "auto", gap: 20, paddingBlock: 24, backgroundColor: "var(--workbench-background)",
    paddingInline: { default: 24, "@media (max-width: 799px)": 16 },
  },
  mobileBar: { display: "flex", alignItems: "center", gap: 9, flexShrink: 0, padding: "6px 12px",
    borderBottom: "1px solid var(--border)", backgroundColor: "var(--workbench-surface)" },
  brand: { minHeight: 72, display: "flex", alignItems: "center", justifyContent: "space-between", flexDirection: "row", gap: 8,
    paddingInline: 16, borderBottom: "1px solid var(--border)", backgroundColor: "var(--workbench-surface)", color: "var(--workbench-foreground)" },
  brandRail: { justifyContent: "center", paddingInline: 0 },
  brandLink: { display: "flex", alignItems: "center", gap: 10, minWidth: 0, color: "inherit", textDecoration: "none" },
  brandCopy: { display: "grid", gap: 2 },
  brandName: { fontSize: 17, fontWeight: 680, lineHeight: 1.2, letterSpacing: "-0.04em" },
  brandCaption: { fontSize: 11, color: "var(--workbench-muted)" },
  status: { display: "flex", alignItems: "center", height: 28, paddingInline: 16, gap: 12, flexShrink: 0,
    borderTop: "1px solid var(--border)", backgroundColor: "var(--workbench-surface)", color: "var(--workbench-muted)", fontSize: 11 },
  statusItem: { display: "inline-flex", alignItems: "center", gap: 6 },
  version: { marginLeft: "auto", fontFamily: "var(--font-mono)", fontVariantNumeric: "tabular-nums" },
});
