import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  resetSystemInformationStore,
  useSystemInformationStore,
} from "src/stores/system-information-store";
import {
  systemInformationFixture,
  systemSummaryFixture,
} from "src/test/system-information-fixture";

describe("system information store", () => {
  beforeEach(() => {
    resetSystemInformationStore();
  });

  afterEach(() => {
    clearMocks();
  });

  it("loads a summary once for concurrent callers", async () => {
    const handler = vi.fn((command: string) => {
      expect(command).toBe("get_system_summary");
      return systemSummaryFixture;
    });
    mockIPC(handler);

    const { loadSummary } = useSystemInformationStore.getState();
    const [first, second] = await Promise.all([loadSummary(), loadSummary()]);

    expect(first).toEqual(systemSummaryFixture);
    expect(second).toEqual(systemSummaryFixture);
    expect(handler).toHaveBeenCalledTimes(1);
    expect(useSystemInformationStore.getState().summaryStatus).toBe("success");
  });

  it("synchronizes the summary after loading full information", async () => {
    mockIPC((command) => {
      expect(command).toBe("get_system_information");
      return systemInformationFixture;
    });

    await useSystemInformationStore.getState().loadInformation();

    expect(useSystemInformationStore.getState().information).toEqual(
      systemInformationFixture,
    );
    expect(useSystemInformationStore.getState().summary).toEqual(
      systemInformationFixture.summary,
    );
  });

  it("preserves the previous snapshot when refresh fails", async () => {
    useSystemInformationStore.setState({
      information: systemInformationFixture,
      informationStatus: "success",
    });
    mockIPC(() => {
      throw {
        code: "systemInformationCollectionFailed",
        message: "collector failed",
      };
    });

    const result =
      await useSystemInformationStore.getState().refreshInformation();

    expect(result).toBeNull();
    expect(useSystemInformationStore.getState().information).toEqual(
      systemInformationFixture,
    );
    expect(useSystemInformationStore.getState().informationStatus).toBe("error");
    expect(useSystemInformationStore.getState().informationError?.code).toBe(
      "systemInformationCollectionFailed",
    );
  });

  it("keeps a newer full snapshot when an older summary resolves later", async () => {
    let resolveSummary: ((value: typeof systemSummaryFixture) => void) | undefined;
    const delayedSummary = new Promise<typeof systemSummaryFixture>((resolve) => {
      resolveSummary = resolve;
    });
    mockIPC((command) =>
      command === "get_system_summary"
        ? delayedSummary
        : systemInformationFixture,
    );

    const summaryPromise =
      useSystemInformationStore.getState().loadSummary({ force: true });
    await useSystemInformationStore.getState().loadInformation();
    resolveSummary?.({
      ...systemSummaryFixture,
      osLongVersion: "Stale OS",
    });
    await summaryPromise;

    expect(useSystemInformationStore.getState().summary?.osLongVersion).toBe(
      "Windows 11 Pro",
    );
  });
});
