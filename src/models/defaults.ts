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
    locale: "system",
    theme: "system",
    widget: {
      mode: "card",
      alwaysOnTop: false,
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
      enabled: true,
      warningRemainingPercent: 25,
      criticalRemainingPercent: 10,
      notifyWhenRecovered: true,
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
  alwaysOnTop: false,
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
