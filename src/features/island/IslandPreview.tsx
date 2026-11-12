import * as stylex from "@stylexjs/stylex";
import { useEffect, useRef, useState } from "react";
import { islandApi, type MediaStatus } from "./islandApi";
import { PreviewContent, motionEasing, type PreviewState } from "./PreviewContent";
import { ui } from "./settingsStyles";
import type { IslandConfig } from "./types";

export function IslandPreview({ config, desktop, report }: {
  config: IslandConfig; desktop: boolean; report: (error: unknown) => void;
}) {
  const [{ state, previous }, setStep] = useState<{ state: PreviewState; previous: PreviewState }>({ state: "compact", previous: "compact" });
  const [playing, setPlaying] = useState(false);
  const [sample, setSample] = useState<{ media: MediaStatus; at: number }>();
  useEffect(() => {
    if (!desktop) return;
    let active = true;
    let revision = 0;
    let off: (() => void) | undefined;
    const fail = (error: unknown) => { if (active) report(error); };
    const refresh = async () => {
      const current = ++revision;
      try {
        const media = await islandApi.media();
        if (active && current === revision) setSample({ media, at: Date.now() });
      } catch (error) { if (current === revision) fail(error); }
    };
    void islandApi.listen("media-changed", refresh).then(unlisten => {
      if (!active) { unlisten(); return; }
      off = unlisten;
      void refresh();
    }).catch(fail);
    return () => { active = false; off?.(); };
  }, [desktop, report]);
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
        ["expanded", expandMs + 800], ["compact", collapseMs + 300],
      ];
      if (config.edge.autoHide && (config.edge.mode === "peek" || config.edge.mode === "fullHide")) {
        steps.push([config.edge.mode === "peek" ? "edgePeek" : "hidden", edgeMs + 400]);
      }
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
  const expanded = state === "expanded";
  const vertical = placement.orientation === "vertical";
  const media = desktop ? sample?.media : undefined;
  const compactWidth = Math.min(appearance.expandedWidth, appearance.compactWidth + (media?.battery ? 32 : 0) + (media?.title ? 26 : 0));
  const width = expanded ? appearance.expandedWidth : compactWidth;
  const height = state === "edgePeek" ? config.edge.peekSize : expanded ? appearance.expandedHeight : appearance.compactHeight;
  const docked = !vertical && width >= height && placement.anchor.startsWith("top") && placement.offsetY <= 0;
  const w = vertical ? height : width;
  const h = vertical ? width : height;
  const duration = !animation.enabled || animation.reducedMotion || animation.performance === "performance" ? 0
    : state === "edgePeek" || previous === "edgePeek" || state === "hidden" || previous === "hidden"
      ? animation.edgeMs : expanded ? animation.expandMs : animation.collapseMs;
  const offset = state !== "edgePeek" || docked ? "none" : vertical
    ? "translateX(calc(-50cqw + 50%))" : "translateY(calc(-50cqh + 50%))";
  return (
    <section aria-label="动画预览" {...stylex.props(s.preview)}>
      <div aria-hidden="true" {...stylex.props(s.canvas, vertical && s.verticalCanvas, docked && s.dockedCanvas)}>
        <div {...stylex.props(s.frame(w, h, duration,
          motionEasing(animation.preset, expanded, animation.customCurve), offset), state === "hidden" && s.hidden)}>
          <PreviewContent config={config} state={state} width={w} height={h} docked={docked}
            media={media} sampledAt={sample?.at ?? 0} />
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
  canvas: { display: "grid", gridTemplateColumns: "minmax(0, 1fr)", gridTemplateRows: "minmax(0, 1fr)", placeItems: "center",
    width: "100%", height: 200, padding: 12, overflow: "hidden", containerType: "size",
    borderRadius: 12, borderWidth: 1, borderStyle: "solid", borderColor: "#dedee2", backgroundColor: "#f0f0f2" },
  verticalCanvas: { height: "clamp(200px, 65svh, 460px)" },
  dockedCanvas: { alignItems: "start", paddingBlockStart: 0 },
  frame: (w: number, h: number, ms: number, easing: string, transform: string) => ({
    width: `min(${w}px, 100cqw, ${100 * w / h}cqh)`, height: `min(${h}px, 100cqh, ${100 * h / w}cqw)`,
    transitionProperty: "width, height, transform, opacity", transitionTimingFunction: easing, transform,
    transitionDuration: { default: `${ms}ms`, "@media (prefers-reduced-motion: reduce)": "0ms" },
  }),
  hidden: { opacity: 0 },
});
