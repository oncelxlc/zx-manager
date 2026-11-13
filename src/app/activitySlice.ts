import { createSlice, nanoid, type PayloadAction } from "@reduxjs/toolkit";

interface Activity {
  id: string;
  text: string;
  to: string;
  at: number;
}

const initialState: { items: Activity[] } = {
  items: [
    { id: "sample-open", text: "打开项目 zx-once", to: "/projects?id=zx-once", at: Date.UTC(2026, 10, 13, 2, 12) },
    { id: "sample-connect", text: "连接预发布主机", to: "/remote?host=staging", at: Date.UTC(2026, 10, 13, 1, 45) },
  ],
};

const activity = createSlice({
  name: "activity",
  initialState,
  reducers: {
    addActivity: {
      prepare(event: Pick<Activity, "text" | "to">) {
        return { payload: { ...event, id: nanoid(), at: Date.now() } };
      },
      reducer(state, action: PayloadAction<Activity>) {
        state.items.unshift(action.payload);
      },
    },
  },
});

export const { addActivity } = activity.actions;
export const activityReducer = activity.reducer;
