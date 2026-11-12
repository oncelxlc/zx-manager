export type IslandState = "disabled" | "hidden" | "edgePeek" | "compact" | "expanded" | "pinned" | "suspended";
export type Anchor = "topLeft" | "topCenter" | "topRight" | "centerLeft" | "center" | "centerRight" | "bottomLeft" | "bottomCenter" | "bottomRight" | "custom";
export type Orientation = "horizontal" | "vertical" | "auto";
export type ExpandDirection = "auto" | "up" | "down" | "left" | "right";
export type EdgeMode = "disabled" | "compact" | "peek" | "fullHide";
export type Theme = "system" | "light" | "dark";
export type AnimationPreset = "subtle" | "smooth" | "spring" | "custom";
export type CustomCurve = "linear" | "easeIn" | "easeOut";
export type Performance = "performance" | "balanced" | "quality";

export interface IslandConfig {
  schemaVersion: number;
  revision: number;
  enabled: boolean;
  startup: {
    launchAtLogin: boolean;
    showOnStart: boolean;
    startCompact: boolean;
  };
  placement: {
    anchor: Anchor;
    displayId: string | null;
    offsetX: number;
    offsetY: number;
    orientation: Orientation;
    expandDirection: ExpandDirection;
    rememberPosition: boolean;
  };
  edge: {
    mode: EdgeMode;
    autoHide: boolean;
    hideDelayMs: number;
    wakeDelayMs: number;
    triggerWidth: number;
    peekSize: number;
    hideOnFullscreen: boolean;
  };
  appearance: {
    compactWidth: number;
    compactHeight: number;
    expandedWidth: number;
    expandedHeight: number;
    radius: number;
    opacity: number;
    background: string;
    border: string;
    shadow: boolean;
    blur: boolean;
    theme: Theme;
  };
  animation: {
    enabled: boolean;
    preset: AnimationPreset;
    customCurve?: CustomCurve;
    expandMs: number;
    collapseMs: number;
    edgeMs: number;
    contentMs: number;
    amplitude: number;
    reducedMotion: boolean;
    performance: Performance;
  };
  interaction: {
    hoverExpand: boolean;
    clickExpand: boolean;
    pinEnabled: boolean;
    dragEnabled: boolean;
  };
  notifications: {
    enabled: boolean;
  };
}
