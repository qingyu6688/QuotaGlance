import type {
  PreferencesEnvelope,
  QuotaSnapshot,
  RefreshState,
  WindowState,
} from "../types/quota";

export const INITIAL_QUOTA_SNAPSHOT: QuotaSnapshot = {
  schemaVersion: 1,
  revision: 0,
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

export const INITIAL_PREFERENCES: PreferencesEnvelope = {
  preferences: {
    schemaVersion: 1,
    revision: 0,
    locale: "zh-CN",
    theme: "system",
    widget: {
      mode: "card",
      alwaysOnTop: true,
      clickThrough: false,
      selectedQuota: {
        limitId: null,
        slot: null,
      },
      boundsByMode: {
        orb: null,
        card: null,
      },
    },
    notifications: {
      enabled: false,
      warningRemainingPercent: 50,
      criticalRemainingPercent: 10,
      notifyWhenRecovered: false,
    },
    startup: {
      launchAtLogin: false,
    },
    updates: {
      autoCheck: true,
      channel: "stable",
      lastCheckedAt: null,
    },
  },
  recovery: null,
};

export const INITIAL_WINDOW_STATE: WindowState = {
  revision: 0,
  mode: "card",
  visible: true,
  alwaysOnTop: true,
  clickThrough: false,
  bounds: null,
};

export const INITIAL_REFRESH_STATE: RefreshState = {
  revision: 0,
  phase: "idle",
  reason: null,
  startedAt: null,
  nextAllowedManualRefreshAt: null,
  nextRetryAt: null,
};
