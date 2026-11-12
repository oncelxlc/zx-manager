import * as stylex from "@stylexjs/stylex";
import { ChevronDown, ChevronUp, MessageCircle } from "lucide-react";
import { useEffect, useState } from "react";
import { ui } from "./settingsStyles";
import type { IslandConfig } from "./types";

export function NotificationExample({ config }: { config: IslandConfig }) {
  const options = config.notifications;
  const source = options.wechatEnabled ? "微信" : "QQ";
  const [sample, setSample] = useState(0);
  const [expanded, setExpanded] = useState(false);
  const [showText, setShowText] = useState(false);
  const canSimulate = options.windowsNotificationsEnabled && (options.wechatEnabled || options.qqEnabled);
  const animate = options.animateNewMessage && config.animation.enabled && !config.animation.reducedMotion
    && config.animation.performance !== "performance";
  useEffect(() => {
    if (!sample || !options.showPreview) return;
    const timer = setTimeout(() => setShowText(false), options.previewTimeoutSeconds * 1000);
    return () => clearTimeout(timer);
  }, [sample, options.showPreview, options.previewTimeoutSeconds]);
  const close = () => { setSample(0); setExpanded(false); setShowText(false); };
  const Icon = expanded ? ChevronUp : ChevronDown;
  return <section aria-label="通知样例" {...stylex.props(s.example)}>
    <div {...stylex.props(s.actions)}>
      <button type="button" disabled={!canSimulate} {...stylex.props(ui.button)} onClick={() => {
        setSample(value => value + 1); setExpanded(false); setShowText(options.showPreview);
      }}>模拟一条提醒</button>
      {canSimulate && sample > 0 && <button type="button" {...stylex.props(ui.button)} onClick={close}>结束演示</button>}
    </div>
    {canSimulate && sample > 0 && <>
      <p role="status" {...stylex.props(ui.message)}>本地示例 · {source}有系统提醒</p>
      <div {...stylex.props(s.island)}>
        <button type="button" aria-expanded={expanded} aria-label={expanded ? "收起通知示例" : "展开通知示例"}
          {...stylex.props(s.capsule)} onClick={() => setExpanded(value => !value)}>
          <span {...stylex.props(s.icon)}>
            <MessageCircle aria-hidden="true" size={19} />
            <span key={sample} aria-hidden="true" {...stylex.props(s.dot, animate && s.appear)} />
          </span>
          <strong>{source}</strong><Icon size={16} aria-hidden="true" />
        </button>
        {expanded && <div {...stylex.props(s.body)}>
          <div><strong>有新提醒</strong><p {...stylex.props(s.copy)}>{showText && options.showPreview
            ? "敏感内容已隐藏" : "来自 Windows 通知中心"}</p></div>
          {options.accountListEnabled && <ul aria-label="示例窗口" {...stylex.props(s.list)}>
            <li>{source} 1</li>{options.wechatEnabled && <li>微信 2</li>}
          </ul>}
        </div>}
      </div>
    </>}
  </section>;
}

const pop = stylex.keyframes({ from: { opacity: 0, transform: "scale(0.75)" }, to: { opacity: 1, transform: "scale(1)" } });
const s = stylex.create({
  example: { marginBlock: 18 },
  actions: { display: "flex", alignItems: "center", flexWrap: "wrap", gap: 8 },
  island: { color: "#fafaf9", backgroundColor: "#202326", width: "fit-content", maxWidth: "100%", borderRadius: 18, padding: 3 },
  capsule: { display: "flex", alignItems: "center", gap: 12, paddingInline: 13, paddingBlock: 8, minHeight: 38, cursor: "pointer",
    fontFamily: "inherit", fontSize: "1rem", color: "inherit", backgroundColor: "transparent", borderWidth: 0, borderRadius: 16,
    outline: { default: null, ":focus-visible": "2px solid #7ddaca" } },
  icon: { position: "relative", display: "inline-flex" },
  dot: { position: "absolute", top: -2, right: -4, width: 9, height: 9, borderRadius: 8, backgroundColor: "#ff7777",
    border: "1px solid #202326" },
  appear: { animationName: pop, animationDuration: { default: "160ms", "@media (prefers-reduced-motion: reduce)": "0ms" } },
  body: { display: "flex", flexWrap: "wrap", gap: 18, padding: 12, alignItems: "start" },
  copy: { fontSize: 13, marginBlock: 6, color: "#dededb" },
  list: { margin: 0, paddingBlock: 0, paddingInline: 10, borderInlineStart: "1px solid #777c80", listStyle: "none", lineHeight: 1.6 },
});
