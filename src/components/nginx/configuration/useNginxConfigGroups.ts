import { useCallback, useEffect, useState } from "react";

import {
  getPreferences,
  setNginxPreferences,
} from "src/services/storage/preferences-storage";
import {
  defaultNginxPreferences,
  type NginxConfigCustomGroup,
} from "src/types/preferences";

export function useNginxConfigGroups() {
  const [groups, setGroups] = useState<NginxConfigCustomGroup[]>([]);
  const [nodeGroups, setNodeGroups] = useState<Record<string, string>>({});

  useEffect(() => {
    void getPreferences().then((preferences) => {
      const nginx = preferences.nginx ?? defaultNginxPreferences;
      setGroups(nginx.configCustomGroups);
      setNodeGroups(nginx.configNodeGroups);
    });
  }, []);

  const persist = useCallback((
    nextGroups: NginxConfigCustomGroup[],
    nextNodeGroups: Record<string, string>,
  ) => {
    setGroups(nextGroups);
    setNodeGroups(nextNodeGroups);
    void setNginxPreferences({
      configCustomGroups: nextGroups,
      configNodeGroups: nextNodeGroups,
    });
  }, []);

  const addGroup = useCallback((name: string) => {
    const trimmed = name.trim();
    if (!trimmed || groups.length >= 100) return;
    const id = `group-${Date.now().toString(36)}`;
    persist([...groups, { id, name: trimmed.slice(0, 80) }], nodeGroups);
  }, [groups, nodeGroups, persist]);

  const assignNode = useCallback((nodeId: string, groupId: string | null) => {
    const next = { ...nodeGroups };
    if (groupId) next[nodeId] = groupId;
    else delete next[nodeId];
    persist(groups, next);
  }, [groups, nodeGroups, persist]);

  return { groups, nodeGroups, addGroup, assignNode };
}
