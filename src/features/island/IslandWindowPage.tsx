import * as stylex from "@stylexjs/stylex";
import { useEffect, useRef, type PointerEvent } from "react";
import { IslandShell } from "./IslandShell";
import { islandApi } from "./islandApi";
import { useIsland } from "./useIsland";

export function IslandWindowPage() {
  const { config, status, desktop, error, report } = useIsland(false);
  const pointer = useRef<{ id: number; x: number; y: number } | null>(null);
  const dragged = useRef(false);
  const initialized = config !== null && status !== null;
  useEffect(() => {
    if (desktop && initialized) void islandApi.ready().catch(report);
  }, [desktop, initialized, report]);

  function press(event: PointerEvent<HTMLElement>) {
    if (!event.isPrimary || event.button !== 0) return;
    dragged.current = false;
    if (!config?.interaction.dragEnabled || status?.state === "edgePeek"
      || (event.target instanceof Element && event.target.closest("[data-island-actions]"))) return;
    pointer.current = { id: event.pointerId, x: event.screenX, y: event.screenY };
    // Capture on the pressed button so a short press still delivers its normal click.
    if (event.target instanceof Element) event.target.setPointerCapture(event.pointerId);
  }
  function move(event: PointerEvent<HTMLElement>) {
    const start = pointer.current;
    if (!start || start.id !== event.pointerId || Math.hypot(event.screenX - start.x, event.screenY - start.y) < 5) return;
    pointer.current = null;
    dragged.current = true;
    void islandApi.drag().catch(report);
  }
  function click() {
    if (!dragged.current) void islandApi.interact("click").catch(report);
    dragged.current = false;
  }
  if (!desktop) return <p>请在 zx-once 桌面应用中打开灵动岛。</p>;
  return (
    <main {...stylex.props(s.window)}>
      {config && status && <IslandShell config={config} state={status.state} onClick={click}
        onPin={() => { void islandApi.pin(status.state !== "pinned").catch(report); }}
        onHide={() => { void islandApi.hide().catch(report); }}
        events={{
          onPointerDown: press, onPointerMove: move,
          onPointerUp: () => { pointer.current = null; }, onPointerCancel: () => { pointer.current = null; },
          onPointerEnter: () => { void islandApi.interact("pointerEnter").catch(report); },
          onPointerLeave: () => { void islandApi.interact("pointerLeave").catch(report); },
        }} />}
      {error && <span role="alert" title={error} {...stylex.props(s.error)}>{error}</span>}
    </main>
  );
}

const s = stylex.create({
  window: { position: "fixed", inset: 0, overflow: "hidden", backgroundColor: "transparent" },
  error: { position: "absolute", insetInline: 0, bottom: 0, borderRadius: 6, paddingInline: 6, color: "#fff",
    backgroundColor: "#842d25", overflow: "hidden", whiteSpace: "nowrap", textOverflow: "ellipsis", fontSize: 12, pointerEvents: "none" },
});
