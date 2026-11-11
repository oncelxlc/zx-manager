import { useTranslation } from "react-i18next";
import { ArrowRightIcon } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { NginxTopologyEdge, NginxTopologyNode } from "src/types/nginx";

export function NginxTopology({
  nodes,
  edges,
}: {
  nodes: NginxTopologyNode[];
  edges: NginxTopologyEdge[];
}) {
  const { t } = useTranslation("nginx");
  const labels = new Map(nodes.map((node) => [node.id, node.label]));
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("topology.title")}</CardTitle>
        <CardDescription>{t("topology.description")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div className="flex flex-wrap gap-2">
          {nodes.map((node) => (
            <Badge key={node.id} variant="outline">
              {t(`topology.kinds.${node.kind}`, { defaultValue: node.kind })}: {node.label}
            </Badge>
          ))}
        </div>
        {edges.length > 0 ? edges.map((edge) => (
          <div className="flex items-center gap-2 text-sm" key={`${edge.from}-${edge.to}`}>
            <span>{labels.get(edge.from)}</span>
            <ArrowRightIcon className="text-muted-foreground" />
            <span>{labels.get(edge.to)}</span>
            <span className="text-muted-foreground">({edge.label})</span>
          </div>
        )) : <p className="text-sm text-muted-foreground">{t("topology.empty")}</p>}
      </CardContent>
    </Card>
  );
}
