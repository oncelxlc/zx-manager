import { useVirtualizer } from "@tanstack/react-virtual";
import { useRef } from "react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { NginxConfigGraphNode } from "src/types/nginx";

export function NginxConfigGraphList({
  nodes,
  onSelect,
}: {
  nodes: NginxConfigGraphNode[];
  onSelect: (nodeId: string) => void;
}) {
  const { t } = useTranslation("nginx");
  const viewportRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: nodes.length,
    getScrollElement: () => viewportRef.current,
    estimateSize: () => 112,
    overscan: 8,
  });
  return (
    <div ref={viewportRef} className="h-[min(65vh,720px)] overflow-auto rounded-xl border">
      <div
        className="relative w-full"
        role="list"
        style={{ height: `${virtualizer.getTotalSize()}px` }}
      >
        {virtualizer.getVirtualItems().map((item) => {
          const node = nodes[item.index];
          return (
            <div
              className="absolute left-0 top-0 w-full p-2"
              data-index={item.index}
              key={node.id}
              ref={virtualizer.measureElement}
              role="listitem"
              style={{ transform: `translateY(${item.start}px)` }}
            >
              <Card size="sm">
                <CardHeader>
                  <CardTitle className="truncate">{node.label}</CardTitle>
                  <div className="flex flex-wrap gap-1.5">
                    <Badge variant="secondary">{node.kind}</Badge>
                    {!node.known ? <Badge variant="outline">{t("configuration.unknown")}</Badge> : null}
                    <Badge variant="outline">{t("configuration.line", { line: node.location.line })}</Badge>
                  </div>
                </CardHeader>
                <CardContent className="flex items-center justify-between gap-3">
                  <code className="truncate text-xs text-muted-foreground">{node.sourceId}</code>
                  <Button onClick={() => onSelect(node.id)} size="sm" variant="outline">
                    {t("configuration.details")}
                  </Button>
                </CardContent>
              </Card>
            </div>
          );
        })}
      </div>
    </div>
  );
}
