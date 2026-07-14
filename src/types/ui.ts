import type {
  Preferences,
  QuotaSnapshot,
  RefreshState,
  Theme,
  WidgetMode,
  WindowState,
} from "./quota";

export type PendingActionName =
  | "refresh"
  | "mode"
  | "pin"
  | "clickThrough"
  | "theme"
  | "startup"
  | null;

export type { Preferences, QuotaSnapshot, RefreshState, Theme, WidgetMode, WindowState };
