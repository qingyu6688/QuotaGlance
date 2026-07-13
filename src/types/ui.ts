import type {
  Preferences,
  QuotaSnapshot,
  RefreshState,
  Theme,
  WidgetMode,
  WindowState,
} from "./quota";

export type PendingActionName = "refresh" | "mode" | "pin" | "clickThrough" | "theme" | null;

export type { Preferences, QuotaSnapshot, RefreshState, Theme, WidgetMode, WindowState };
