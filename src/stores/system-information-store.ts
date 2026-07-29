import { create } from "zustand";

import {
  getSystemInformation,
  getSystemSummary,
  toCommandError,
} from "src/services/tauri/system-information";
import type {
  CommandError,
  SystemInformation,
  SystemSummary,
} from "src/types/system-information";

type LoadStatus = "idle" | "loading" | "success" | "error";

interface LoadSummaryOptions {
  force?: boolean;
}

interface SystemInformationState {
  summary: SystemSummary | null;
  information: SystemInformation | null;
  summaryStatus: LoadStatus;
  informationStatus: LoadStatus;
  summaryError: CommandError | null;
  informationError: CommandError | null;
  loadSummary: (options?: LoadSummaryOptions) => Promise<SystemSummary | null>;
  loadInformation: () => Promise<SystemInformation | null>;
  refreshInformation: () => Promise<SystemInformation | null>;
}

let summaryFlight: Promise<SystemSummary | null> | null = null;
let informationFlight: Promise<SystemInformation | null> | null = null;
let requestSequence = 0;
let appliedSummarySequence = 0;
let appliedInformationSequence = 0;

async function runSummaryLoad(
  force: boolean,
): Promise<SystemSummary | null> {
  const state = useSystemInformationStore.getState();
  if (!force && state.summary) {
    return state.summary;
  }
  if (summaryFlight) {
    return summaryFlight;
  }

  const requestId = ++requestSequence;
  useSystemInformationStore.setState({
    summaryStatus: "loading",
    summaryError: null,
  });
  summaryFlight = getSystemSummary()
    .then((summary) => {
      if (requestId >= appliedSummarySequence) {
        appliedSummarySequence = requestId;
        useSystemInformationStore.setState({
          summary,
          summaryStatus: "success",
          summaryError: null,
        });
      }
      return summary;
    })
    .catch((error: unknown) => {
      const commandError = toCommandError(error);
      if (requestId >= appliedSummarySequence) {
        useSystemInformationStore.setState({
          summaryStatus: "error",
          summaryError: commandError,
        });
      }
      return null;
    })
    .finally(() => {
      summaryFlight = null;
    });

  return summaryFlight;
}

async function runInformationLoad(): Promise<SystemInformation | null> {
  if (informationFlight) {
    return informationFlight;
  }

  const requestId = ++requestSequence;
  useSystemInformationStore.setState({
    informationStatus: "loading",
    informationError: null,
  });
  informationFlight = getSystemInformation()
    .then((information) => {
      if (requestId >= appliedInformationSequence) {
        appliedInformationSequence = requestId;
        appliedSummarySequence = Math.max(appliedSummarySequence, requestId);
        useSystemInformationStore.setState({
          information,
          informationStatus: "success",
          informationError: null,
          summary: information.summary,
          summaryStatus: "success",
          summaryError: null,
        });
      }
      return information;
    })
    .catch((error: unknown) => {
      const commandError = toCommandError(error);
      if (requestId >= appliedInformationSequence) {
        useSystemInformationStore.setState({
          informationStatus: "error",
          informationError: commandError,
        });
      }
      return null;
    })
    .finally(() => {
      informationFlight = null;
    });

  return informationFlight;
}

export const useSystemInformationStore = create<SystemInformationState>(
  (_set, get) => ({
    summary: null,
    information: null,
    summaryStatus: "idle",
    informationStatus: "idle",
    summaryError: null,
    informationError: null,
    loadSummary: (options) => runSummaryLoad(options?.force ?? false),
    loadInformation: () => {
      const information = get().information;
      return information ? Promise.resolve(information) : runInformationLoad();
    },
    refreshInformation: () => runInformationLoad(),
  }),
);

export function resetSystemInformationStore() {
  summaryFlight = null;
  informationFlight = null;
  requestSequence = 0;
  appliedSummarySequence = 0;
  appliedInformationSequence = 0;
  useSystemInformationStore.setState({
    summary: null,
    information: null,
    summaryStatus: "idle",
    informationStatus: "idle",
    summaryError: null,
    informationError: null,
  });
}
