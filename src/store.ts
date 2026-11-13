import { configureStore, createSlice } from "@reduxjs/toolkit";
import { activityReducer } from "./app/activitySlice";
import { preferencesReducer } from "./app/preferencesSlice";
import { environmentReducer } from "./features/environment/environmentSlice";
import { gitReducer } from "./features/git/gitSlice";
import { githubReducer } from "./features/github/githubSlice";
import { island } from "./features/island/islandSlice";
import { projectsReducer } from "./features/projects/projectsSlice";
import { remoteReducer } from "./features/remote/remoteSlice";

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

export function createAppStore(dependenciesExpanded = true) {
  return configureStore({
    reducer: {
      welcome: welcome.reducer, island: island.reducer, preferences: preferencesReducer, activity: activityReducer,
      projects: projectsReducer, environment: environmentReducer, git: gitReducer, github: githubReducer, remote: remoteReducer,
    },
    preloadedState: { welcome: { dependenciesExpanded } },
  });
}

type AppStore = ReturnType<typeof createAppStore>;
export type RootState = ReturnType<AppStore["getState"]>;
