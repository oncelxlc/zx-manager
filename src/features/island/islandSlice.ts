import { createSlice, type PayloadAction } from "@reduxjs/toolkit";
import type { IslandConfig } from "./types";
import type { IslandDisplay, RuntimeStatus } from "./islandApi";

interface IslandData {
  config: IslandConfig | null;
  status: RuntimeStatus | null;
  displays: IslandDisplay[];
  error: string | null;
}

const initialState: IslandData = { config: null, status: null, displays: [], error: null };

export const island = createSlice({
  name: "island",
  initialState,
  reducers: {
    configReceived(state, { payload }: PayloadAction<IslandConfig>) {
      if (!state.config || payload.revision >= state.config.revision) state.config = payload;
    },
    statusReceived(state, { payload }: PayloadAction<RuntimeStatus>) {
      if (!state.status || payload.sequence >= state.status.sequence) state.status = payload;
    },
    displaysReceived(state, { payload }: PayloadAction<IslandDisplay[]>) {
      state.displays = payload;
    },
    errorChanged(state, { payload }: PayloadAction<string | null>) {
      state.error = payload;
    },
  },
});
