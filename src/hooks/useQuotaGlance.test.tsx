import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useQuotaGlance } from "./useQuotaGlance";

afterEach(() => {
  vi.useRealTimers();
});

describe("useQuotaGlance", () => {
  it("操作完成后保留反馈 2600ms，再自动清除", async () => {
    const { result } = renderHook(() => useQuotaGlance());

    await waitFor(() => expect(result.current.snapshot.status).toBe("ok"));
    const nextTheme =
      result.current.preferencesEnvelope.preferences.theme === "aurora" ? "paper" : "aurora";

    vi.useFakeTimers();
    await act(async () => {
      await result.current.setTheme(nextTheme);
    });

    expect(result.current.feedback).toBe("主题已更新");

    await act(() => vi.advanceTimersByTime(2_599));
    expect(result.current.feedback).toBe("主题已更新");

    await act(() => vi.advanceTimersByTime(1));
    expect(result.current.feedback).toBeNull();
  });
});
