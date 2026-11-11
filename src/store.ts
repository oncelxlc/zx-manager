import { configureStore, createSlice } from "@reduxjs/toolkit";

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
  return configureStore({ reducer: { welcome: welcome.reducer } });
}

type AppStore = ReturnType<typeof createAppStore>;
export type RootState = ReturnType<AppStore["getState"]>;
