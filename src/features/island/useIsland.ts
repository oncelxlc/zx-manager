import { useCallback, useEffect, useReducer } from "react";
import { useDispatch, useSelector } from "react-redux";
import type { RootState } from "../../store";
import { islandApi } from "./islandApi";
import { island } from "./islandSlice";

const { configReceived, statusReceived, displaysReceived, errorChanged } = island.actions;

export function useIsland(settings: boolean) {
  const dispatch = useDispatch();
  const data = useSelector((state: RootState) => state.island);
  const desktop = islandApi.isDesktop();
  const [attempt, retry] = useReducer((value: number) => value + 1, 0);
  const report = useCallback((reason: unknown) => dispatch(errorChanged(String(reason))), [dispatch]);

  useEffect(() => {
    if (!desktop) return;
    dispatch(errorChanged(null));
    let active = true;
    let displayRequest = 0;
    const removers: (() => void)[] = [];
    const refreshDisplays = async () => {
      const request = ++displayRequest;
      const displays = await islandApi.displays();
      if (active && request === displayRequest) dispatch(displaysReceived(displays));
    };
    const refreshConfig = async () => {
      const config = await islandApi.config();
      if (active) dispatch(configReceived(config));
    };
    const fail = (error: unknown) => { if (active) report(error); };
    const attach = async (subscription: Promise<() => void>) => {
      const off = await subscription;
      if (active) removers.push(off);
      else off();
    };
    async function connect() {
      const subscriptions = [
        islandApi.listen("state-changed", status => { if (active) dispatch(statusReceived(status)); }),
        islandApi.listen("config-changed", config => { if (active) dispatch(configReceived(config)); }),
        islandApi.listen("runtime-error", fail),
      ];
      if (settings) {
        subscriptions.push(
          islandApi.listen("display-changed", () => { if (active) void refreshDisplays().catch(fail); }),
          islandApi.listen("position-changed", () => { if (active) void refreshConfig().catch(fail); }),
        );
      }
      await Promise.all(subscriptions.map(attach));
      if (!active) return;
      await Promise.all([
        refreshConfig(),
        islandApi.status().then(status => {
          if (active) {
            dispatch(statusReceived(status));
            if (status.lastError) report(status.lastError);
          }
        }),
        settings && refreshDisplays(),
      ]);
    }
    void connect().catch(fail);
    return () => {
      active = false;
      for (const off of removers) off();
    };
  }, [attempt, desktop, dispatch, report, settings]);

  return { ...data, desktop, report, dispatch, retry };
}
