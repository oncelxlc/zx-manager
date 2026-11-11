import { configureStore, createSlice } from "@reduxjs/toolkit";
import { island } from "./features/island/islandSlice";

const welcome = createSlice({
  name: "welcome",
  initialState: { dependenciesExpanded: true },
  reducers: {
    toggleDependencies(state) {
      state.dependenciesExpanded = !state.dependenciesExpanded;
    },
  },
});

export const { toggleDependencies } = welcome.actions;

export function createAppStore() {
  return configureStore({ reducer: { welcome: welcome.reducer, island: island.reducer } });
}

type AppStore = ReturnType<typeof createAppStore>;
export type RootState = ReturnType<AppStore["getState"]>;
