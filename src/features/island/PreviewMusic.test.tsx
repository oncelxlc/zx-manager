// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import defaults from "./defaults.json";
import { PreviewMusic } from "./PreviewMusic";
import type { MediaStatus } from "./islandApi";

afterEach(cleanup);

it("uses the source's two-column music page whenever lyrics are enabled, even with no lyrics provider", () => {
  const media: MediaStatus = {
    title: "测试曲目", artist: "测试音乐人", playing: false, durationMs: 210000, positionMs: 60000,
    canNext: true, canPrevious: true, canToggle: true, canSeek: true, cover: null, battery: null,
  };
  const page = (lyricsEnabled: boolean, top = true) => <svg><PreviewMusic media={media} sampledAt={0}
    now={new Date(2026, 0, 1)} clock="00:00" motion={false} lyricsEnabled={lyricsEnabled} top={top} /></svg>;
  const { container, rerender } = render(page(defaults.appearance.lyricsEnabled));
  expect(screen.getByText("暂无歌词")).toBeTruthy();
  expect(screen.getByText("岛上歌词")).toBeTruthy();
  expect(container.querySelector('rect[x="44"][y="23"][width="44"]')).toBeTruthy();
  expect(container.querySelector('rect[x="44"][width="176"][height="3.5"]')).toBeTruthy();
  expect(screen.getByText("01:00").getAttribute("y")).toBe("100");
  rerender(page(false));
  expect(screen.queryByText("暂无歌词")).toBeNull();
  expect(screen.getByText("岛上歌词")).toBeTruthy();
  expect(container.querySelector('rect[x="44"][y="13"][width="62"]')).toBeTruthy();
  expect(container.querySelector('rect[x="96"][width="221"][height="3.5"]')).toBeTruthy();
  rerender(page(true, false));
  expect(screen.getByText("暂无歌词")).toBeTruthy();
  expect(screen.queryByText("岛上歌词")).toBeNull();
});
