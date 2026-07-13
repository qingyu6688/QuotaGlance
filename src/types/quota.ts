export type IsoTimestamp = string;

export type QuotaStatus =
  | "loading"
  | "ok"
  | "stale"
  | "signedOut"
  | "apiKeyMode"
  | "quotaReached"
  | "sourceBusy"
  | "offline"
  | "serviceUnavailable"
  | "appServerUnavailable"
  | "incompatible";

export type AuthUiState =
  | "signedOut"
  | "authenticated"
  | "apiKeyMode"
  | "externalProvider"
  | "unknown";

export interface AuthSummary {
  state: AuthUiState;
  authMode: string | null;
  planType: string | null;
  requiresOpenaiAuth: boolean | null;
}

export type QuotaSource = "appServer" | "legacyCompat";
export type WindowSlot = "primary" | "secondary" | "other";
export type WindowKind = "shortTerm" | "weekly" | "monthly" | "unknown";

export interface QuotaWindow {
  slot: WindowSlot;
  kind: WindowKind;
  label: string;
  usedPercent: number;
  remainingPercent: number;
  windowDurationMins: number;
  resetsAt: IsoTimestamp;
}

export interface CreditSummary {
  hasCredits: boolean | null;
  unlimited: boolean | null;
  balance: string | null;
}

export interface QuotaBucket {
  limitId: string;
  limitName: string | null;
  planType: string | null;
  windows: QuotaWindow[];
  credits: CreditSummary | null;
  rateLimitReachedType: string | null;
}

export interface ResetCreditDetail {
  resetType: string;
  status: string;
  grantedAt: IsoTimestamp;
  expiresAt: IsoTimestamp | null;
  title: string | null;
  description: string | null;
}

export interface ResetCreditSummary {
  availableCount: number;
  details: ResetCreditDetail[] | null;
}

export type ErrorCode =
  | "INVALID_ARGUMENT"
  | "FORBIDDEN"
  | "NOT_READY"
  | "SHUTTING_DOWN"
  | "APP_SERVER_NOT_FOUND"
  | "APP_SERVER_EXECUTION_DENIED"
  | "APP_SERVER_VERSION_INCOMPATIBLE"
  | "APP_SERVER_HANDSHAKE_TIMEOUT"
  | "APP_SERVER_EXITED"
  | "PROTOCOL_INVALID_MESSAGE"
  | "PROTOCOL_MESSAGE_TOO_LARGE"
  | "PROTOCOL_REQUEST_TIMEOUT"
  | "AUTH_REQUIRED"
  | "API_KEY_MODE"
  | "RATE_LIMITS_UNAVAILABLE"
  | "RESPONSE_INCOMPATIBLE"
  | "SOURCE_BUSY"
  | "OFFLINE"
  | "SERVICE_UNAVAILABLE"
  | "REFRESH_COOLDOWN"
  | "PREFERENCES_CONFLICT"
  | "PREFERENCES_CORRUPTED"
  | "PREFERENCES_VERSION_UNSUPPORTED"
  | "PREFERENCES_WRITE_FAILED"
  | "WINDOW_OPERATION_FAILED"
  | "UPDATE_CHECK_FAILED"
  | "UPDATE_SIGNATURE_INVALID"
  | "UPDATE_INSTALL_FAILED";

export interface QuotaError {
  code: ErrorCode;
  messageKey: string;
  retryable: boolean;
  retryAfterMs: number | null;
}

export interface QuotaSnapshot {
  schemaVersion: 1;
  revision: number;
  source: QuotaSource | null;
  provider: "codexAppServer" | "legacyWham" | null;
  auth: AuthSummary;
  buckets: QuotaBucket[];
  bankedResets: ResetCreditSummary | null;
  status: QuotaStatus;
  fetchedAt: IsoTimestamp | null;
  lastGoodAt: IsoTimestamp | null;
  nextRetryAt: IsoTimestamp | null;
  error: QuotaError | null;
}

export type RefreshReason =
  | "startup"
  | "manual"
  | "accountNotification"
  | "quotaNotification"
  | "visibleResync"
  | "hiddenResync"
  | "resume";

export type RefreshPhase = "idle" | "refreshing" | "cooldown" | "backingOff";

export interface RefreshState {
  revision: number;
  phase: RefreshPhase;
  reason: RefreshReason | null;
  startedAt: IsoTimestamp | null;
  nextAllowedManualRefreshAt: IsoTimestamp | null;
  nextRetryAt: IsoTimestamp | null;
}

export interface RefreshReceipt {
  accepted: boolean;
  joinedExistingRequest: boolean;
  requestRevision: number;
  state: RefreshState;
}

export type Locale = "system" | "zh-CN" | "en";
export type Theme =
  | "system"
  | "aurora"
  | "graphite"
  | "paper"
  | "sunset"
  | "honey"
  | "rose";
export type WidgetMode = "orb" | "card" | "hidden";

export interface WindowBounds {
  x: number;
  y: number;
  width: number;
  height: number;
  monitorId: string | null;
  scaleFactorAtSave: number;
}

export interface SelectedQuota {
  limitId: string | null;
  slot: "primary" | "secondary" | null;
}

export interface Preferences {
  schemaVersion: 1;
  revision: number;
  locale: Locale;
  theme: Theme;
  widget: {
    mode: WidgetMode;
    alwaysOnTop: boolean;
    clickThrough: boolean;
    selectedQuota: SelectedQuota;
    boundsByMode: {
      orb: WindowBounds | null;
      card: WindowBounds | null;
    };
  };
  notifications: {
    enabled: boolean;
    warningRemainingPercent: number;
    criticalRemainingPercent: number;
    notifyWhenRecovered: boolean;
  };
  startup: {
    launchAtLogin: boolean;
  };
  updates: {
    autoCheck: boolean;
    channel: "stable";
    lastCheckedAt: IsoTimestamp | null;
  };
}

export interface PreferencesEnvelope {
  preferences: Preferences;
  recovery: null | {
    source: "backup" | "defaults";
    reasonCode: "PREFERENCES_CORRUPTED" | "PREFERENCES_VERSION_UNSUPPORTED";
  };
}

export interface WindowState {
  revision: number;
  mode: WidgetMode;
  visible: boolean;
  alwaysOnTop: boolean;
  clickThrough: boolean;
  bounds: WindowBounds | null;
}

export interface IpcError {
  code: ErrorCode;
  messageKey: string;
  retryable: boolean;
  retryAfterMs: number | null;
  context: {
    field: string | null;
    currentRevision: number | null;
    appServerPhase: string | null;
  };
}

export type Unsubscribe = () => void;
