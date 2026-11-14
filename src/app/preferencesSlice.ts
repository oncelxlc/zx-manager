import { createSlice, type PayloadAction } from "@reduxjs/toolkit";
import { readPreference, writePreference } from "./localPreferences";
import { defaults, destinations, isNavId, type NavGroup, type NavId, type Preferences, type Theme } from "./navigation";

const storageKey = "zx-once.workbench.preferences.v1";

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object";
}
function isTheme(value: unknown): value is Theme {
  return value === "light" || value === "dark" || value === "system";
}
function isIdList(value: unknown): value is NavId[] {
  return Array.isArray(value) && value.every(isNavId) && new Set(value).size === value.length;
}
function isGroup(value: unknown): value is NavGroup {
  return isRecord(value) && typeof value.id === "string" && Boolean(value.id.trim())
    && typeof value.title === "string" && Boolean(value.title.trim()) && isIdList(value.items)
    && typeof value.hidden === "boolean";
}
function isPreferences(value: unknown): value is Preferences {
  if (!isRecord(value) || !isTheme(value.theme)
    || (value.sidebar !== "auto" && value.sidebar !== "expanded" && value.sidebar !== "collapsed")
    || !isIdList(value.hidden) || !isIdList(value.pins)
    || !Array.isArray(value.groups) || !value.groups.every(isGroup)) return false;
  const groups = value.groups;
  const items = groups.flatMap(group => group.items);
  return isIdList(items) && items.length === Object.keys(destinations).length
    && new Set(groups.map(group => group.id)).size === groups.length;
}

function load(): Preferences {
  const saved = readPreference(storageKey);
  if (!isPreferences(saved)) {
    return isRecord(saved) && isTheme(saved.theme) ? { ...defaults, theme: saved.theme } : defaults;
  }
  // Older preferences also stored a collapsed flag on each group.
  const groups = saved.groups.map(({ id, title, items, hidden }) => ({ id, title, items, hidden }));
  return { ...saved, groups };
}

export function persistPreferences(value: Preferences) {
  writePreference(storageKey, value);
}

const preferences = createSlice({
  name: "preferences",
  initialState: load,
  reducers: {
    setTheme(state, { payload }: PayloadAction<Theme>) { state.theme = payload; },
    setSidebar(state, { payload }: PayloadAction<Preferences["sidebar"]>) { state.sidebar = payload; },
    toggleHidden(state, { payload }: PayloadAction<NavId>) {
      state.hidden = state.hidden.includes(payload) ? state.hidden.filter(id => id !== payload) : [...state.hidden, payload];
    },
    togglePin(state, { payload }: PayloadAction<NavId>) {
      state.pins = state.pins.includes(payload) ? state.pins.filter(id => id !== payload) : [...state.pins, payload];
    },
    addGroup(state, { payload }: PayloadAction<{ id: string; title: string }>) {
      if (payload.title.trim() && !state.groups.some(group => group.id === payload.id)) {
        state.groups.push({ id: payload.id, title: payload.title.trim(), items: [], hidden: false });
      }
    },
    renameGroup(state, { payload }: PayloadAction<{ id: string; title: string }>) {
      const group = state.groups.find(group => group.id === payload.id);
      if (group && payload.title.trim()) group.title = payload.title.trim();
    },
    toggleGroupVisibility(state, { payload }: PayloadAction<string>) {
      const group = state.groups.find(group => group.id === payload);
      if (group) group.hidden = !group.hidden;
    },
    moveGroup(state, { payload }: PayloadAction<{ id: string; index: number }>) {
      const from = state.groups.findIndex(group => group.id === payload.id);
      if (from < 0 || from === payload.index) return;
      const [group] = state.groups.splice(from, 1);
      state.groups.splice(payload.index, 0, group);
    },
    moveItem(state, { payload }: PayloadAction<{ id: NavId; groupId: string; index?: number }>) {
      const origin = state.groups.find(group => group.items.includes(payload.id));
      const target = state.groups.find(group => group.id === payload.groupId);
      if (!origin || !target) return;
      origin.items.splice(origin.items.indexOf(payload.id), 1);
      target.items.splice(payload.index ?? target.items.length, 0, payload.id);
    },
    resetNavigation(state) {
      state.groups = defaults.groups;
      state.hidden = defaults.hidden;
      state.pins = defaults.pins;
      state.sidebar = defaults.sidebar;
    },
  },
});

export const { setTheme, setSidebar, toggleHidden, togglePin, addGroup, renameGroup,
  toggleGroupVisibility, moveGroup, moveItem, resetNavigation } = preferences.actions;
export const preferencesReducer = preferences.reducer;
