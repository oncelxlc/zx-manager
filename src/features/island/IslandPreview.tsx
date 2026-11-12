import * as stylex from "@stylexjs/stylex";
import { useEffect, useRef, useState } from "react";
import { islandApi } from "./islandApi";
import { PreviewContent, motionEasing, type PreviewState } from "./PreviewContent";
import { ui } from "./settingsStyles";
import type { IslandConfig } from "./types";

export function IslandPreview({ config, desktop, report }: {
  config: IslandConfig; desktop: boolean; report: (error: unknown) => void;
}) {
  const [{ state, previous }, setStep] = useState<{ state: PreviewState; previous: PreviewState }>({ state: "compact", previous: "compact" });
  const [playing, setPlaying] = useState(false);
  const generation = useRef(0);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  function show(state: PreviewState) {
    setStep(current => current.state === state ? current : { state, previous: current.state });
  }
  function cancel() {
    ++generation.current;
    clearTimeout(timer.current);
    show("compact");
    setPlaying(false);
  }
  useEffect(() => {
    cancel();
    const visibility = () => { if (document.hidden) cancel(); };
    document.addEventListener("visibilitychange", visibility);
    return () => {
      ++generation.current;
      clearTimeout(timer.current);
      document.removeEventListener("visibilitychange", visibility);
    };
  }, [config]);
  async function play() {
    cancel();
    const request = generation.current;
    setPlaying(true);
    try {
      if (desktop) await islandApi.validatePreview(config);
      if (request !== generation.current) return;
      const { expandMs, collapseMs, edgeMs } = config.animation;
      const steps: [PreviewState, number][] = [
        ["expanded", expandMs + 450], ["pinned", 450], ["compact", collapseMs + 300], ["edgePeek", edgeMs + 400],
      ];
      function next(index: number) {
        if (request !== generation.current) return;
        if (index === steps.length) { cancel(); return; }
        const [target, wait] = steps[index];
        show(target);
        timer.current = setTimeout(() => next(index + 1), wait);
      }
      next(0);
    } catch (error) {
      if (request === generation.current) { cancel(); report(error); }
    }
  }
  const { appearance, animation, placement } = config;
  const expanded = state === "expanded" || state === "pinned";
  const vertical = placement.orientation === "vertical";
  const width = expanded ? appearance.expandedWidth : appearance.compactWidth;
  const height = state === "edgePeek" ? config.edge.peekSize : expanded ? appearance.expandedHeight : appearance.compactHeight;
  const duration = !animation.enabled || animation.reducedMotion || animation.performance === "performance" ? 0
    : state === "edgePeek" || previous === "edgePeek" ? animation.edgeMs : expanded ? animation.expandMs : animation.collapseMs;
  const offset = state !== "edgePeek" ? "none" : vertical
    ? "translateX(calc(-50cqw + 50%))" : "translateY(calc(-50cqh + 50%))";
  return (
    <section aria-label="动画预览" {...stylex.props(s.preview)}>
      <div aria-hidden="true" {...stylex.props(s.canvas)}>
        <div {...stylex.props(s.frame(vertical ? height : width, vertical ? width : height, duration, motionEasing(animation.preset, expanded, animation.customCurve), offset))}>
          <PreviewContent config={config} state={state} />
        </div>
      </div>
      <button type="button" {...stylex.props(ui.button)} aria-pressed={playing}
        onClick={event => {
          if (playing) cancel();
          else if (event.currentTarget.form?.reportValidity() !== false) void play();
        }}>{playing ? "取消预览" : "播放预览"}</button>
    </section>
  );
}

const s = stylex.create({
  preview: { marginBlock: 18, display: "grid", justifyItems: "start", gap: 10 },
  canvas: { display: "grid", placeItems: "center", width: "100%", height: 200, padding: 12, overflow: "hidden", containerType: "size",
    borderRadius: 12, border: "1px solid #d6ded9", backgroundColor: "#e8eeeb" },
  frame: (w: number, h: number, ms: number, easing: string, transform: string) => ({
    width: `${w}px`, height: `${h}px`, maxWidth: "100%", maxHeight: "100%", minWidth: 0, minHeight: 0,
    transitionProperty: "width, height, transform", transitionTimingFunction: easing, transform,
    transitionDuration: { default: `${ms}ms`, "@media (prefers-reduced-motion: reduce)": "0ms" },
  }),
});
