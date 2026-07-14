import { describe, expect, it } from "vitest";
import { createMockSnapshot } from "../api/fixtures";
import type { QuotaSnapshot } from "../types/quota";
import {
  formatExpirationAt,
  resolveQuotaPresentation,
  selectWeeklyQuotaWindow,
} from "./quota-view";

describe("额度展示投影", () => {
  it("从动态额度桶中投影短周期、周额度和重置机会", () => {
    const snapshot = createMockSnapshot("ok", Date.parse("2026-07-14T00:00:00Z"));
    const presentation = resolveQuotaPresentation(snapshot, {
      limitId: "codex",
      slot: "primary",
    });

    expect(presentation.primaryWindow?.kind).toBe("shortTerm");
    expect(presentation.primaryWindow?.remainingPercent).toBe(74);
    expect(presentation.weeklyWindow?.kind).toBe("weekly");
    expect(presentation.weeklyWindow?.remainingPercent).toBe(96);
    expect(presentation.resetCreditCount).toBe(1);
    expect(presentation.resetCreditExpirations).toHaveLength(1);
  });

  it("服务端只返回周额度时不伪造短周期窗口", () => {
    const base = createMockSnapshot("ok");
    const bucket = base.buckets[0];
    if (bucket === undefined) {
      throw new Error("测试快照缺少额度桶");
    }
    const snapshot: QuotaSnapshot = {
      ...base,
      buckets: [
        {
          ...bucket,
          windows: bucket.windows.filter((quotaWindow) => quotaWindow.kind === "weekly"),
        },
      ],
    };
    const presentation = resolveQuotaPresentation(snapshot, {
      limitId: "codex",
      slot: "primary",
    });

    expect(presentation.primaryWindow).toBeNull();
    expect(presentation.weeklyWindow).not.toBeNull();
    expect(selectWeeklyQuotaWindow(snapshot, { limitId: null, slot: null })).toEqual(
      presentation.weeklyWindow,
    );
  });

  it("不会根据槽位猜测周期，也不会跨额度桶拼接窗口", () => {
    const base = createMockSnapshot("ok");
    const bucket = base.buckets[0];
    if (bucket === undefined) {
      throw new Error("测试快照缺少额度桶");
    }
    const shortTerm = bucket.windows.find((quotaWindow) => quotaWindow.kind === "shortTerm");
    const weekly = bucket.windows.find((quotaWindow) => quotaWindow.kind === "weekly");
    if (shortTerm === undefined || weekly === undefined) {
      throw new Error("测试快照缺少短周期或周额度窗口");
    }
    const snapshot: QuotaSnapshot = {
      ...base,
      buckets: [
        {
          ...bucket,
          windows: [
            shortTerm,
            {
              ...weekly,
              slot: "primary",
              kind: "monthly",
              label: "月度额度",
            },
            {
              ...weekly,
              slot: "secondary",
              kind: "unknown",
              label: "未知周期额度",
            },
          ],
        },
        {
          ...bucket,
          limitId: "review",
          limitName: "代码审查",
          windows: [weekly],
        },
      ],
    };
    const presentation = resolveQuotaPresentation(snapshot, {
      limitId: "codex",
      slot: "primary",
    });

    expect(presentation.bucket?.limitId).toBe("codex");
    expect(presentation.primaryWindow).toEqual(shortTerm);
    expect(presentation.weeklyWindow).toBeNull();
  });

  it("重置机会到期时间会去重并过滤空值", () => {
    const base = createMockSnapshot("ok");
    const expiration = "2026-07-20T00:00:00Z";
    const snapshot: QuotaSnapshot = {
      ...base,
      bankedResets: {
        availableCount: 3,
        details: [
          {
            resetType: "manual",
            status: "available",
            grantedAt: "2026-07-14T00:00:00Z",
            expiresAt: expiration,
            title: null,
            description: null,
          },
          {
            resetType: "manual",
            status: "available",
            grantedAt: "2026-07-14T00:00:00Z",
            expiresAt: expiration,
            title: null,
            description: null,
          },
          {
            resetType: "manual",
            status: "available",
            grantedAt: "2026-07-14T00:00:00Z",
            expiresAt: null,
            title: null,
            description: null,
          },
        ],
      },
    };

    expect(
      resolveQuotaPresentation(snapshot, { limitId: null, slot: null }).resetCreditExpirations,
    ).toEqual([expiration]);
    expect(formatExpirationAt("invalid")).toBe("到期时间未知");
  });
});
