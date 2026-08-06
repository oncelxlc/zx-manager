import { create } from "zustand";

import {
  applyNginxGlobalConfigurationPatch,
  getNginxGlobalConfiguration,
  toNginxCommandError,
  validateNginxGlobalConfigurationPatch,
} from "src/services/tauri/nginx-manager";
import type {
  NginxCommandError,
  NginxGlobalConfigApplyMode,
  NginxGlobalConfigApplyResult,
  NginxGlobalConfigPatchValidation,
  NginxGlobalConfiguration,
  NginxGlobalConfigurationPatch,
} from "src/types/nginx";

type Status = "idle" | "loading" | "validating" | "applying" | "error";

interface State {
  source: NginxGlobalConfiguration | null;
  draft: NginxGlobalConfigurationPatch | null;
  validation: NginxGlobalConfigPatchValidation | null;
  result: NginxGlobalConfigApplyResult | null;
  status: Status;
  error: NginxCommandError | null;
  load: (instanceId: string) => Promise<void>;
  update: <K extends keyof NginxGlobalConfigurationPatch>(
    field: K,
    value: NginxGlobalConfigurationPatch[K],
  ) => void;
  discard: () => void;
  validate: () => Promise<boolean>;
  apply: (mode: NginxGlobalConfigApplyMode) => Promise<boolean>;
  clear: () => void;
}

function values(configuration: NginxGlobalConfiguration): NginxGlobalConfigurationPatch {
  const { instanceId: _instanceId, revision: _revision, ...patch } = configuration;
  return patch;
}

export const useNginxGlobalConfigStore = create<State>((set, get) => ({
  source: null,
  draft: null,
  validation: null,
  result: null,
  status: "idle",
  error: null,
  load: async (instanceId) => {
    set({ status: "loading", error: null, validation: null, result: null });
    try {
      const source = await getNginxGlobalConfiguration(instanceId);
      set({ source, draft: values(source), status: "idle" });
    } catch (error) {
      set({ status: "error", error: toNginxCommandError(error) });
    }
  },
  update: (field, value) => set((state) => ({
    draft: state.draft ? { ...state.draft, [field]: value } : null,
    validation: null,
    result: null,
  })),
  discard: () => set((state) => ({
    draft: state.source ? values(state.source) : null,
    validation: null,
    result: null,
    error: null,
  })),
  validate: async () => {
    const { source, draft } = get();
    if (!source || !draft) return false;
    set({ status: "validating", error: null });
    try {
      const validation = await validateNginxGlobalConfigurationPatch(
        source.instanceId,
        source.revision.value,
        draft,
      );
      set({ validation, status: "idle" });
      return validation.parserValid && validation.nativeValid
        && validation.fieldErrors.length === 0;
    } catch (error) {
      set({ status: "error", error: toNginxCommandError(error) });
      return false;
    }
  },
  apply: async (mode) => {
    const { source, draft } = get();
    if (!source || !draft) return false;
    set({ status: "applying", error: null, result: null });
    try {
      const result = await applyNginxGlobalConfigurationPatch(
        source.instanceId,
        source.revision.value,
        draft,
        mode,
      );
      set({ result, status: "idle" });
      if (result.success) await get().load(source.instanceId);
      return result.success;
    } catch (error) {
      set({ status: "error", error: toNginxCommandError(error) });
      return false;
    }
  },
  clear: () => set({
    source: null,
    draft: null,
    validation: null,
    result: null,
    status: "idle",
    error: null,
  }),
}));
