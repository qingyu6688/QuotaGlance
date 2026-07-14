import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { QuotaGlanceApi } from "./contract";
import type {
  PreferencesEnvelope,
  QuotaSnapshot,
  RefreshReceipt,
  RefreshState,
  Theme,
  Unsubscribe,
  WidgetMode,
  WindowState,
} from "../types/quota";

export const tauriClient: QuotaGlanceApi = {
  getQuotaSnapshot: () => invoke<QuotaSnapshot>("get_quota_snapshot"),
  getPreferences: () => invoke<PreferencesEnvelope>("get_preferences"),
  setTheme: (theme: Theme) => invoke<PreferencesEnvelope>("set_theme", { theme }),
  refreshQuota: () => invoke<RefreshReceipt>("refresh_quota"),
  setWidgetMode: (mode: WidgetMode) => invoke<WindowState>("set_widget_mode", { mode }),
  setAlwaysOnTop: (enabled: boolean) =>
    invoke<WindowState>("set_always_on_top", { enabled }),
  setClickThrough: (enabled: boolean) =>
    invoke<WindowState>("set_click_through", { enabled }),
  setLaunchAtLogin: (enabled: boolean) =>
    invoke<PreferencesEnvelope>("set_launch_at_login", { enabled }),
  quitApp: () => invoke<void>("quit_app"),
  onQuotaSnapshot: (listener: (snapshot: QuotaSnapshot) => void): Promise<Unsubscribe> =>
    listen<QuotaSnapshot>("quota://snapshot-updated", (event) => listener(event.payload)),
  onRefreshState: (listener: (state: RefreshState) => void): Promise<Unsubscribe> =>
    listen<RefreshState>("quota://refresh-state-changed", (event) => listener(event.payload)),
  onPreferences: (listener: (envelope: PreferencesEnvelope) => void): Promise<Unsubscribe> =>
    listen<PreferencesEnvelope>("preferences://changed", (event) => listener(event.payload)),
  onWindowState: (listener: (state: WindowState) => void): Promise<Unsubscribe> =>
    listen<WindowState>("window://state-changed", (event) => listener(event.payload)),
};
