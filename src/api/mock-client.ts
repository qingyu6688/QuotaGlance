import type { QuotaGlanceApi } from "./contract";
import {
  createMockPreferences,
  createMockSnapshot,
  isMockScenario,
  type MockScenario,
} from "./fixtures";
import { INITIAL_REFRESH_STATE } from "../models/defaults";
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

type Listener<T> = (value: T) => void;

function initialScenario(): MockScenario {
  if (typeof window === "undefined") {
    return "ok";
  }

  const candidate = new URLSearchParams(window.location.search).get("state");
  return isMockScenario(candidate) ? candidate : "ok";
}

function notify<T>(listeners: ReadonlySet<Listener<T>>, value: T): void {
  listeners.forEach((listener) => listener(value));
}

function subscribe<T>(listeners: Set<Listener<T>>, listener: Listener<T>): Promise<Unsubscribe> {
  listeners.add(listener);
  return Promise.resolve(() => listeners.delete(listener));
}

export class BrowserMockClient implements QuotaGlanceApi {
  private snapshot: QuotaSnapshot = createMockSnapshot(initialScenario());
  private preferences: PreferencesEnvelope = createMockPreferences();
  private refreshState: RefreshState = INITIAL_REFRESH_STATE;
  private windowState: WindowState = {
    revision: 1,
    mode: this.preferences.preferences.widget.mode,
    visible: true,
    alwaysOnTop: this.preferences.preferences.widget.alwaysOnTop,
    clickThrough: this.preferences.preferences.widget.clickThrough,
    bounds: null,
  };

  private readonly snapshotListeners = new Set<Listener<QuotaSnapshot>>();
  private readonly refreshListeners = new Set<Listener<RefreshState>>();
  private readonly preferencesListeners = new Set<Listener<PreferencesEnvelope>>();
  private readonly windowListeners = new Set<Listener<WindowState>>();

  getQuotaSnapshot(): Promise<QuotaSnapshot> {
    return Promise.resolve(this.snapshot);
  }

  getPreferences(): Promise<PreferencesEnvelope> {
    return Promise.resolve(this.preferences);
  }

  setTheme(theme: Theme): Promise<PreferencesEnvelope> {
    if (this.preferences.preferences.theme === theme) {
      return Promise.resolve(this.preferences);
    }

    this.preferences = {
      ...this.preferences,
      preferences: {
        ...this.preferences.preferences,
        revision: this.preferences.preferences.revision + 1,
        theme,
      },
    };
    notify(this.preferencesListeners, this.preferences);
    return Promise.resolve(this.preferences);
  }

  refreshQuota(): Promise<RefreshReceipt> {
    const startedAt = new Date().toISOString();
    this.refreshState = {
      revision: this.refreshState.revision + 1,
      phase: "refreshing",
      reason: "manual",
      startedAt,
      nextAllowedManualRefreshAt: null,
      nextRetryAt: null,
    };
    notify(this.refreshListeners, this.refreshState);

    window.setTimeout(() => {
      const refreshed = createMockSnapshot("ok");
      this.snapshot = {
        ...refreshed,
        revision: this.snapshot.revision + 1,
      };
      notify(this.snapshotListeners, this.snapshot);

      this.refreshState = {
        revision: this.refreshState.revision + 1,
        phase: "idle",
        reason: null,
        startedAt: null,
        nextAllowedManualRefreshAt: null,
        nextRetryAt: null,
      };
      notify(this.refreshListeners, this.refreshState);
    }, 650);

    return Promise.resolve({
      accepted: true,
      joinedExistingRequest: false,
      requestRevision: this.refreshState.revision,
      state: this.refreshState,
    });
  }

  setWidgetMode(mode: WidgetMode): Promise<WindowState> {
    return this.updateWindowState({ mode, visible: mode !== "hidden" });
  }

  setAlwaysOnTop(enabled: boolean): Promise<WindowState> {
    return this.updateWindowState({ alwaysOnTop: enabled });
  }

  setClickThrough(enabled: boolean): Promise<WindowState> {
    return this.updateWindowState({ clickThrough: enabled });
  }

  quitApp(): Promise<void> {
    return Promise.resolve();
  }

  onQuotaSnapshot(listener: Listener<QuotaSnapshot>): Promise<Unsubscribe> {
    return subscribe(this.snapshotListeners, listener);
  }

  onRefreshState(listener: Listener<RefreshState>): Promise<Unsubscribe> {
    return subscribe(this.refreshListeners, listener);
  }

  onPreferences(listener: Listener<PreferencesEnvelope>): Promise<Unsubscribe> {
    return subscribe(this.preferencesListeners, listener);
  }

  onWindowState(listener: Listener<WindowState>): Promise<Unsubscribe> {
    return subscribe(this.windowListeners, listener);
  }

  setScenario(scenario: MockScenario): void {
    const next = createMockSnapshot(scenario);
    this.snapshot = {
      ...next,
      revision: this.snapshot.revision + 1,
    };
    notify(this.snapshotListeners, this.snapshot);
  }

  private updateWindowState(
    patch: Partial<Pick<WindowState, "mode" | "visible" | "alwaysOnTop" | "clickThrough">>,
  ): Promise<WindowState> {
    this.windowState = {
      ...this.windowState,
      ...patch,
      revision: this.windowState.revision + 1,
    };

    this.preferences = {
      ...this.preferences,
      preferences: {
        ...this.preferences.preferences,
        revision: this.preferences.preferences.revision + 1,
        widget: {
          ...this.preferences.preferences.widget,
          mode: this.windowState.mode,
          alwaysOnTop: this.windowState.alwaysOnTop,
          clickThrough: this.windowState.clickThrough,
        },
      },
    };

    notify(this.windowListeners, this.windowState);
    notify(this.preferencesListeners, this.preferences);
    return Promise.resolve(this.windowState);
  }
}
