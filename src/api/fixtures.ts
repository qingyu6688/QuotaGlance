import { INITIAL_PREFERENCES } from "../models/defaults";
import type { PreferencesEnvelope, QuotaSnapshot } from "../types/quota";

export type MockScenario = "ok" | "warning" | "danger" | "stale" | "loading" | "error";

const SCENARIOS: ReadonlySet<string> = new Set([
  "ok",
  "warning",
  "danger",
  "stale",
  "loading",
  "error",
]);

export function isMockScenario(value: string | null): value is MockScenario {
  return value !== null && SCENARIOS.has(value);
}

function isoAfter(now: number, minutes: number): string {
  return new Date(now + minutes * 60_000).toISOString();
}

function createHealthySnapshot(now: number, remainingPercent: number): QuotaSnapshot {
  const fetchedAt = new Date(now).toISOString();

  return {
    schemaVersion: 1,
    revision: 1,
    source: "appServer",
    provider: "codexAppServer",
    auth: {
      state: "authenticated",
      authMode: "chatgpt",
      planType: "pro",
      requiresOpenaiAuth: true,
    },
    buckets: [
      {
        limitId: "codex",
        limitName: "Codex",
        planType: "pro",
        windows: [
          {
            slot: "primary",
            kind: "shortTerm",
            label: "5 小时额度",
            usedPercent: 26,
            remainingPercent: 74,
            windowDurationMins: 300,
            resetsAt: isoAfter(now, 78),
          },
          {
            slot: "secondary",
            kind: "weekly",
            label: "周额度",
            usedPercent: 100 - remainingPercent,
            remainingPercent,
            windowDurationMins: 10_080,
            resetsAt: isoAfter(now, 10_080),
          },
        ],
        credits: null,
        rateLimitReachedType: remainingPercent === 7 ? "primary" : null,
      },
    ],
    bankedResets: {
      availableCount: 1,
      details: [
        {
          resetType: "manual",
          status: "available",
          grantedAt: fetchedAt,
          expiresAt: isoAfter(now, 9 * 24 * 60),
          title: null,
          description: null,
        },
      ],
    },
    status: remainingPercent === 7 ? "quotaReached" : "ok",
    fetchedAt,
    lastGoodAt: fetchedAt,
    nextRetryAt: null,
    error: null,
  };
}

export function createMockSnapshot(scenario: MockScenario, now = Date.now()): QuotaSnapshot {
  if (scenario === "loading") {
    return {
      schemaVersion: 1,
      revision: 1,
      source: null,
      provider: null,
      auth: {
        state: "unknown",
        authMode: null,
        planType: null,
        requiresOpenaiAuth: null,
      },
      buckets: [],
      bankedResets: null,
      status: "loading",
      fetchedAt: null,
      lastGoodAt: null,
      nextRetryAt: null,
      error: null,
    };
  }

  if (scenario === "error") {
    return {
      schemaVersion: 1,
      revision: 1,
      source: null,
      provider: null,
      auth: {
        state: "unknown",
        authMode: null,
        planType: null,
        requiresOpenaiAuth: null,
      },
      buckets: [],
      bankedResets: null,
      status: "appServerUnavailable",
      fetchedAt: null,
      lastGoodAt: null,
      nextRetryAt: isoAfter(now, 1),
      error: {
        code: "APP_SERVER_NOT_FOUND",
        messageKey: "quota.appServerUnavailable",
        retryable: true,
        retryAfterMs: 60_000,
      },
    };
  }

  const remainingPercent = scenario === "warning" ? 22 : scenario === "danger" ? 7 : 96;
  const snapshot = createHealthySnapshot(now, remainingPercent);

  if (scenario !== "stale") {
    return snapshot;
  }

  const lastGoodAt = new Date(now - 8 * 60_000).toISOString();
  return {
    ...snapshot,
    status: "stale",
    fetchedAt: lastGoodAt,
    lastGoodAt,
    nextRetryAt: isoAfter(now, 2),
    error: {
      code: "OFFLINE",
      messageKey: "quota.offline",
      retryable: true,
      retryAfterMs: 120_000,
    },
  };
}

export function createMockPreferences(): PreferencesEnvelope {
  return {
    preferences: {
      ...INITIAL_PREFERENCES.preferences,
      widget: {
        ...INITIAL_PREFERENCES.preferences.widget,
        selectedQuota: {
          limitId: "codex",
          slot: "primary",
        },
      },
    },
    recovery: null,
  };
}
