import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useRef } from "react";

import type { NginxLogLine } from "src/types/nginx";

export function NginxLogViewer({ lines }: { lines: NginxLogLine[] }) {
  const viewportRef = useRef<HTMLDivElement>(null);
  const followRef = useRef(true);
  const virtualizer = useVirtualizer({
    count: lines.length,
    getScrollElement: () => viewportRef.current,
    estimateSize: () => 24,
    overscan: 20,
  });
  useEffect(() => {
    if (followRef.current && lines.length > 0) virtualizer.scrollToIndex(lines.length - 1);
  }, [lines.length, virtualizer]);
  return (
    <div
      aria-label="Nginx log viewer"
      className="h-[min(65vh,720px)] overflow-auto rounded-xl border bg-zinc-950 text-zinc-100"
      onScroll={(event) => {
        const element = event.currentTarget;
        followRef.current = element.scrollHeight - element.scrollTop - element.clientHeight < 32;
      }}
      ref={viewportRef}
      role="log"
      tabIndex={0}
    >
      <div className="relative w-full" style={{ height: `${virtualizer.getTotalSize()}px` }}>
        {virtualizer.getVirtualItems().map((item) => {
          const line = lines[item.index];
          return (
            <div
              className="absolute left-0 top-0 w-full whitespace-pre-wrap break-all px-3 py-0.5 font-mono text-xs"
              data-index={item.index}
              key={`${line.offset}-${item.index}`}
              ref={virtualizer.measureElement}
              style={{ transform: `translateY(${item.start}px)` }}
            >
              {line.text}{line.truncated ? " …" : ""}
            </div>
          );
        })}
      </div>
    </div>
  );
}
