import { useEffect } from "react";
import { useTranslation } from "react-i18next";

import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { NginxConfigSource } from "src/types/nginx";

export function NginxSourceViewer({
  sources,
  sourceId,
  focusLine,
  onSourceChange,
}: {
  sources: NginxConfigSource[];
  sourceId: string;
  focusLine: number | null;
  onSourceChange: (sourceId: string) => void;
}) {
  const { t } = useTranslation("nginx");
  const source = sources.find((item) => item.id === sourceId) ?? sources[0];
  const items = sources.map((item) => ({ label: item.displayPath, value: item.id }));
  useEffect(() => {
    if (focusLine && source) {
      document.getElementById(`line-${focusLine}`)?.scrollIntoView({ block: "center" });
    }
  }, [focusLine, source]);
  if (!source) {
    return null;
  }
  return (
    <div className="space-y-3">
      <Select
        items={items}
        onValueChange={(next) => next && onSourceChange(next)}
        value={source.id}
      >
        <SelectTrigger className="w-full max-w-3xl"><SelectValue /></SelectTrigger>
        <SelectContent alignItemWithTrigger={false}>
          <SelectGroup>
            {items.map((item) => (
              <SelectItem key={item.value} value={item.value}>{item.label}</SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
      <p className="truncate text-xs text-muted-foreground" title={source.displayPath}>
        {source.displayPath}
      </p>
      <pre
        aria-label={t("configuration.source")}
        className="max-h-[65vh] overflow-auto rounded-lg border bg-muted/30 p-4 font-mono text-xs leading-5"
      >
        {source.text.split("\n").map((line, index) => {
          const lineNumber = index + 1;
          return (
            <span
              className={focusLine === lineNumber ? "block bg-primary/15" : "block"}
              id={`line-${lineNumber}`}
              key={lineNumber}
            >
              <span className="mr-4 inline-block w-10 select-none text-right text-muted-foreground">
                {lineNumber}
              </span>
              {line || " "}
            </span>
          );
        })}
      </pre>
    </div>
  );
}
