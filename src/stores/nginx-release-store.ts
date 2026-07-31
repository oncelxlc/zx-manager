import { create } from "zustand";

import {
  checkNginxUpdates,
  getNginxReleaseStatus,
  toNginxCommandError,
} from "src/services/tauri/nginx-manager";
import { defaultNginxPreferences } from "src/types/preferences";
import type { NginxCommandError, NginxReleaseStatus } from "src/types/nginx";
import type { NginxPreferences } from "src/types/preferences";

interface NginxReleaseState {
  status: NginxReleaseStatus | null;
  loading: boolean;
  checking: boolean;
  error: NginxCommandError | null;
  preferences: NginxPreferences;
  configure: (preferences: NginxPreferences) => void;
  loadCached: () => Promise<NginxReleaseStatus | null>;
  check: (force?: boolean) => Promise<NginxReleaseStatus | null>;
}

let requestSequence = 0;
let appliedSequence = 0;

async function runReleaseRequest(
  fetcher: () => Promise<NginxReleaseStatus>,
  checking: boolean,
) {
  const requestId = ++requestSequence;
  useNginxReleaseStore.setState({
    loading: !checking,
    checking,
    error: null,
  });
  try {
    const status = await fetcher();
    if (requestId >= appliedSequence) {
      appliedSequence = requestId;
      useNginxReleaseStore.setState({
        status,
        loading: false,
        checking: false,
      });
    }
    return status;
  } catch (error) {
    if (requestId >= appliedSequence) {
      useNginxReleaseStore.setState({
        error: toNginxCommandError(error),
        loading: false,
        checking: false,
      });
    }
    return null;
  }
}

export const useNginxReleaseStore = create<NginxReleaseState>((set, get) => ({
  status: null,
  loading: false,
  checking: false,
  error: null,
  preferences: defaultNginxPreferences,
  configure: (preferences) => set({ preferences }),
  loadCached: () => {
    const { releaseChannel } = get().preferences;
    return runReleaseRequest(
      () => getNginxReleaseStatus(releaseChannel),
      false,
    );
  },
  check: (force = false) => {
    const { releaseChannel, updateCheckIntervalHours } = get().preferences;
    return runReleaseRequest(
      () => checkNginxUpdates(
        releaseChannel,
        force,
        updateCheckIntervalHours,
      ),
      true,
    );
  },
}));

export function resetNginxReleaseStore() {
  requestSequence = 0;
  appliedSequence = 0;
  useNginxReleaseStore.setState({
    status: null,
    loading: false,
    checking: false,
    error: null,
    preferences: defaultNginxPreferences,
  });
}
