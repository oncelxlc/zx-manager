import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@/components/ui/empty";
import { Spinner } from "@/components/ui/spinner";
import { NginxDiagnostics } from "src/components/nginx/NginxDiagnostics";
import { NginxConfigGraphList } from "src/components/nginx/configuration/NginxConfigGraphList";
import { NginxConfigGraphToolbar } from "src/components/nginx/configuration/NginxConfigGraphToolbar";
import { NginxConfigNodeSheet } from "src/components/nginx/configuration/NginxConfigNodeSheet";
import { useNginxConfigGroups } from "src/components/nginx/configuration/useNginxConfigGroups";
import { NginxInstanceGate } from "src/components/nginx/instance/NginxInstanceGate";
import { NginxPageHeading } from "src/components/nginx/NginxPageHeading";
import { useMainLayoutHeader } from "src/layouts/MainLayout";
import { useNginxConfigurationStore } from "src/stores/nginx-configuration-store";
import { nginxErrorTranslationKey } from "src/utils/nginx-error";
import { useNginxConfigurationPage } from "./useNginxConfigurationPage";

function NginxConfigurationContent() {
  const { t } = useTranslation("nginx");
  const [search, setSearch] = useState("");
  const [kindFilter, setKindFilter] = useState("all");
  const [groupFilter, setGroupFilter] = useState("all");
  const { groups, nodeGroups, addGroup, assignNode } = useNginxConfigGroups();
  useNginxConfigurationPage(null, "graph");
  const graph = useNginxConfigurationStore((state) => state.graph);
  const selectedNode = useNginxConfigurationStore((state) => state.selectedNode);
  const validation = useNginxConfigurationStore((state) => state.validation);
  const loadStatus = useNginxConfigurationStore((state) => state.loadStatus);
  const error = useNginxConfigurationStore((state) => state.error);
  const selectNode = useNginxConfigurationStore((state) => state.selectNode);
  const clearSelectedNode = useNginxConfigurationStore((state) => state.clearSelectedNode);
  const validate = useNginxConfigurationStore((state) => state.validate);
  const nodes = useMemo(() => graph?.nodes.filter((node) => {
    const queryMatches = !search.trim()
      || node.label.toLocaleLowerCase().includes(search.trim().toLocaleLowerCase());
    const kindMatches = kindFilter === "all"
      || (kindFilter === "block" && node.childIds.length > 0)
      || (kindFilter === "unknown" && !node.known)
      || (kindFilter === "directive" && node.childIds.length === 0);
    const assignedGroup = nodeGroups[node.id];
    const groupMatches = groupFilter === "all"
      || (groupFilter === "unassigned" && !assignedGroup)
      || assignedGroup === groupFilter;
    return queryMatches && kindMatches && groupMatches;
  }) ?? [], [graph, groupFilter, kindFilter, nodeGroups, search]);
  if (loadStatus === "loading") {
    return <div className="flex min-h-48 items-center justify-center"><Spinner /></div>;
  }
  if (loadStatus === "error") {
    return (
      <Empty className="border"><EmptyHeader>
        <EmptyTitle>{t("configuration.errorTitle")}</EmptyTitle>
        <EmptyDescription>{t(nginxErrorTranslationKey(error?.code))}</EmptyDescription>
      </EmptyHeader></Empty>
    );
  }
  if (!graph) return null;
  return (
    <div className="flex min-w-0 flex-col gap-4">
      <NginxConfigGraphToolbar
        groupFilter={groupFilter}
        groups={groups}
        kindFilter={kindFilter}
        onAddGroup={addGroup}
        onGroupFilterChange={setGroupFilter}
        onKindFilterChange={setKindFilter}
        onSearchChange={setSearch}
        onValidate={() => void validate()}
        search={search}
      />
      {validation ? (
        <Alert variant={validation.parserValid && validation.nativeValid ? "default" : "destructive"}>
          <AlertTitle>{t("configuration.validationTitle")}</AlertTitle>
          <AlertDescription>{t(
            validation.parserValid && validation.nativeValid
              ? "configuration.validationPassed"
              : "configuration.validationFailed",
          )}</AlertDescription>
        </Alert>
      ) : null}
      <NginxDiagnostics diagnostics={validation?.diagnostics ?? graph.diagnostics} />
      <p className="text-sm text-muted-foreground">
        {t("configuration.graphSummary", {
          files: graph.sources.length,
          nodes: graph.nodes.length,
          visible: nodes.length,
        })}
      </p>
      <NginxConfigGraphList nodes={nodes} onSelect={(id) => void selectNode(id)} />
      <NginxConfigNodeSheet
        detail={selectedNode}
        groupId={selectedNode ? nodeGroups[selectedNode.node.id] ?? null : null}
        groups={groups}
        onGroupChange={(groupId) => selectedNode && assignNode(selectedNode.node.id, groupId)}
        onOpenChange={(open) => { if (!open) clearSelectedNode(); }}
      />
    </div>
  );
}

export function NginxConfigurationPage() {
  const { t } = useTranslation("nginx");
  useMainLayoutHeader({ title: t("configuration.title") });
  return (
    <div className="mx-auto flex w-full min-w-0 max-w-[1800px] flex-col gap-5 p-4">
      <NginxPageHeading
        description={t("configuration.description")}
        title={t("configuration.title")}
      />
      <NginxInstanceGate><NginxConfigurationContent /></NginxInstanceGate>
    </div>
  );
}
