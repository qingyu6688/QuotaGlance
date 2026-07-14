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

export interface QuotaGlanceApi {
  getQuotaSnapshot(): Promise<QuotaSnapshot>;
  getPreferences(): Promise<PreferencesEnvelope>;
  setTheme(theme: Theme): Promise<PreferencesEnvelope>;
  refreshQuota(): Promise<RefreshReceipt>;
  setWidgetMode(mode: WidgetMode): Promise<WindowState>;
  setAlwaysOnTop(enabled: boolean): Promise<WindowState>;
  setClickThrough(enabled: boolean): Promise<WindowState>;
  setLaunchAtLogin(enabled: boolean): Promise<PreferencesEnvelope>;
  quitApp(): Promise<void>;
  onQuotaSnapshot(listener: (snapshot: QuotaSnapshot) => void): Promise<Unsubscribe>;
  onRefreshState(listener: (state: RefreshState) => void): Promise<Unsubscribe>;
  onPreferences(listener: (envelope: PreferencesEnvelope) => void): Promise<Unsubscribe>;
  onWindowState(listener: (state: WindowState) => void): Promise<Unsubscribe>;
}
