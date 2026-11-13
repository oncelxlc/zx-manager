import * as stylex from "@stylexjs/stylex";
import { useEffect, useRef, useState } from "react";
import { islandApi, type MediaStatus } from "./islandApi";
import { PreviewContent, motionEasing } from "./PreviewContent";
import { previewLayout, type PreviewState } from "./previewLayout";
import { Button } from "../../components/ui/button";
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
  const { animation } = config;
  const expanded = state === "expanded";
  const media = desktop ? sample?.media : undefined;
  const layout = previewLayout(config, state, Boolean(media?.battery), Boolean(media?.title));
  const { dock, side, scale, gap, offset } = layout;
  const along = `calc(max(0px, 50${side ? "cqh" : "cqw"} - ${(side ? 150 : 240) * scale}px) * ${offset})`;
  const inset = `${gap * scale * (dock === "right" ? -1 : 1)}px`;
  const position = side ? `translate(${inset}, ${along})` : `translate(${along}, ${inset})`;
  const duration = !animation.enabled || animation.reducedMotion || animation.performance === "performance" ? 0
    : state === "edgePeek" || previous === "edgePeek" || state === "hidden" || previous === "hidden"
      ? animation.edgeMs : expanded ? animation.expandMs : animation.collapseMs;
  return (
    <section aria-label="动画预览" {...stylex.props(s.preview)}>
      <div aria-hidden="true" {...stylex.props(s.canvas, side && s.sideCanvas,
        dock === "left" ? s.left : dock === "right" ? s.right : s.top)}>
        <div {...stylex.props(s.frame(layout.width * scale, layout.height * scale, duration,
          motionEasing(animation.preset, expanded, animation.customCurve), position, side ? gap * scale : 0), state === "hidden" && s.hidden)}>
          <PreviewContent config={config} layout={layout} media={media} sampledAt={sample?.at ?? 0} />
        </div>
      </div>
      <Button variant="outline" aria-pressed={playing}
        onClick={event => {
          if (playing) cancel();
          else if (event.currentTarget.form?.reportValidity() !== false) void play();
        }}>{playing ? "取消预览" : "播放预览"}</Button>
    </section>
  );
}

const s = stylex.create({
  preview: { marginBlock: 18, display: "grid", justifyItems: "start", gap: 10 },
  canvas: { display: "grid", gridTemplateColumns: "minmax(0, 1fr)", gridTemplateRows: "minmax(0, 1fr)", placeItems: "center",
    width: "100%", height: 224, overflow: "hidden", containerType: "size",
    borderRadius: "var(--radius)", borderWidth: 1, borderStyle: "solid", borderColor: "var(--border)", backgroundColor: "var(--muted)" },
  sideCanvas: { height: "clamp(320px, 65svh, 460px)" },
  top: { alignItems: "start" },
  left: { justifyItems: "start" },
  right: { justifyItems: "end" },
  frame: (w: number, h: number, ms: number, easing: string, position: string, sideGap: number) => ({
    width: `min(${w}px, calc(100cqw - ${sideGap}px), ${100 * w / h}cqh)`,
    height: `min(${h}px, 100cqh, calc(${100 * h / w}cqw - ${sideGap * h / w}px))`,
    transform: position, transitionProperty: "width, height, opacity, transform", transitionTimingFunction: easing,
    transitionDuration: { default: `${ms}ms`, "@media (prefers-reduced-motion: reduce)": "0ms" },
  }),
  hidden: { opacity: 0 },
});
