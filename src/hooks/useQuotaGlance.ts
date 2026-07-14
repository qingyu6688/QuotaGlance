import { useCallback, useEffect, useState } from "react";
import { isIpcError, quotaGlanceApi } from "../api";
import {
  INITIAL_PREFERENCES,
  INITIAL_QUOTA_SNAPSHOT,
  INITIAL_REFRESH_STATE,
  INITIAL_WINDOW_STATE,
} from "../models/defaults";
import type {
  PreferencesEnvelope,
  QuotaSnapshot,
  RefreshState,
  Theme,
  Unsubscribe,
  WidgetMode,
  WindowState,
} from "../types/quota";

type PendingAction =
  | "refresh"
  | "mode"
  | "pin"
  | "clickThrough"
  | "theme"
  | "startup"
  | null;

interface QuotaGlanceController {
  snapshot: QuotaSnapshot;
  preferencesEnvelope: PreferencesEnvelope;
  windowState: WindowState;
  refreshState: RefreshState;
  pendingAction: PendingAction;
  feedback: string | null;
  refresh(): Promise<void>;
  setTheme(theme: Theme): Promise<void>;
  setMode(mode: WidgetMode): Promise<void>;
  toggleAlwaysOnTop(): Promise<void>;
  toggleClickThrough(): Promise<void>;
  toggleLaunchAtLogin(): Promise<void>;
  quit(): Promise<void>;
}

function localizedError(error: unknown): string {
  if (!isIpcError(error)) {
    return "操作没有完成，请稍后重试";
  }

  const messages: Partial<Record<typeof error.code, string>> = {
    REFRESH_COOLDOWN: "刷新过于频繁，请稍后再试",
    APP_SERVER_NOT_FOUND: "未找到可用的 Codex，请安装 ChatGPT 桌面应用或 Codex CLI",
    APP_SERVER_EXECUTION_DENIED: "Codex 运行组件无法启动，请检查安装来源与系统权限",
    APP_SERVER_VERSION_INCOMPATIBLE: "当前 Codex 版本不兼容",
    AUTH_REQUIRED: "请先登录 Codex",
    API_KEY_MODE: "API Key 模式不提供订阅额度",
    OFFLINE: "当前网络不可用，已保留最近数据",
    SERVICE_UNAVAILABLE: "额度服务暂时不可用",
    WINDOW_OPERATION_FAILED: "窗口设置没有生效",
    PREFERENCES_WRITE_FAILED: "设置已应用，但保存失败，请检查配置目录权限",
    PREFERENCES_CORRUPTED: "设置文件已损坏，当前使用安全默认值",
    PREFERENCES_VERSION_UNSUPPORTED: "设置来自更高版本，已停止写入以保护原文件",
    STARTUP_OPERATION_FAILED: "开机启动设置没有生效，请检查系统登录项权限",
    FORBIDDEN: "当前窗口没有执行此操作的权限",
  };
  return messages[error.code] ?? "操作没有完成，请稍后重试";
}

function mergeSnapshot(current: QuotaSnapshot, incoming: QuotaSnapshot): QuotaSnapshot {
  return incoming.revision < current.revision ? current : incoming;
}

function mergePreferences(
  current: PreferencesEnvelope,
  incoming: PreferencesEnvelope,
): PreferencesEnvelope {
  return incoming.preferences.revision < current.preferences.revision ? current : incoming;
}

function mergeWindowState(current: WindowState, incoming: WindowState): WindowState {
  return incoming.revision < current.revision ? current : incoming;
}

function mergeRefreshState(current: RefreshState, incoming: RefreshState): RefreshState {
  return incoming.revision < current.revision ? current : incoming;
}

export function useQuotaGlance(): QuotaGlanceController {
  const [snapshot, setSnapshot] = useState<QuotaSnapshot>(INITIAL_QUOTA_SNAPSHOT);
  const [preferencesEnvelope, setPreferencesEnvelope] =
    useState<PreferencesEnvelope>(INITIAL_PREFERENCES);
  const [windowState, setWindowState] = useState<WindowState>(INITIAL_WINDOW_STATE);
  const [refreshState, setRefreshState] = useState<RefreshState>(INITIAL_REFRESH_STATE);
  const [pendingAction, setPendingAction] = useState<PendingAction>(null);
  const [feedback, setFeedback] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    const unsubscribers: Unsubscribe[] = [];

    const initialize = async (): Promise<void> => {
      // 先注册事件，再读取当前值，避免初始化期间漏掉状态变化。
      const listeners = await Promise.all([
        quotaGlanceApi.onQuotaSnapshot((incoming) => {
          if (!disposed) {
            setSnapshot((current) => mergeSnapshot(current, incoming));
          }
        }),
        quotaGlanceApi.onRefreshState((incoming) => {
          if (!disposed) {
            setRefreshState((current) => mergeRefreshState(current, incoming));
          }
        }),
        quotaGlanceApi.onPreferences((incoming) => {
          if (!disposed) {
            setPreferencesEnvelope((current) => mergePreferences(current, incoming));
          }
        }),
        quotaGlanceApi.onWindowState((incoming) => {
          if (!disposed) {
            setWindowState((current) => mergeWindowState(current, incoming));
          }
        }),
      ]);

      if (disposed) {
        listeners.forEach((unsubscribe) => unsubscribe());
        return;
      }
      unsubscribers.push(...listeners);

      const [currentSnapshot, currentPreferences] = await Promise.all([
        quotaGlanceApi.getQuotaSnapshot(),
        quotaGlanceApi.getPreferences(),
      ]);
      if (disposed) {
        return;
      }

      setSnapshot((current) => mergeSnapshot(current, currentSnapshot));
      setPreferencesEnvelope((current) => mergePreferences(current, currentPreferences));
      setWindowState((current) => {
        if (current.revision !== INITIAL_WINDOW_STATE.revision) {
          return current;
        }
        const widget = currentPreferences.preferences.widget;
        return {
          revision: currentPreferences.preferences.revision,
          mode: widget.mode,
          visible: widget.mode !== "hidden",
          alwaysOnTop: widget.alwaysOnTop,
          clickThrough: widget.clickThrough,
          bounds: null,
        };
      });
    };

    void initialize().catch((error: unknown) => {
      if (!disposed) {
        setFeedback(localizedError(error));
      }
    });

    return () => {
      disposed = true;
      unsubscribers.forEach((unsubscribe) => unsubscribe());
    };
  }, []);

  const refresh = useCallback(async (): Promise<void> => {
    if (pendingAction === "refresh" || refreshState.phase === "refreshing") {
      return;
    }

    setPendingAction("refresh");
    setFeedback("正在刷新额度…");
    try {
      const receipt = await quotaGlanceApi.refreshQuota();
      setRefreshState((current) => mergeRefreshState(current, receipt.state));
      setFeedback(receipt.joinedExistingRequest ? "已加入正在进行的刷新" : "刷新请求已受理");
    } catch (error: unknown) {
      setFeedback(localizedError(error));
    } finally {
      setPendingAction(null);
    }
  }, [pendingAction, refreshState.phase]);

  const setMode = useCallback(async (mode: WidgetMode): Promise<void> => {
    if (pendingAction === "mode" || mode === "hidden") {
      return;
    }

    setPendingAction("mode");
    setFeedback(mode === "orb" ? "正在收起为浮球…" : "正在展开额度卡片…");
    try {
      const next = await quotaGlanceApi.setWidgetMode(mode);
      setWindowState((current) => mergeWindowState(current, next));
      setFeedback(mode === "orb" ? "已切换为浮球" : "已展开额度卡片");
    } catch (error: unknown) {
      setFeedback(localizedError(error));
    } finally {
      setPendingAction(null);
    }
  }, [pendingAction]);

  const setTheme = useCallback(async (theme: Theme): Promise<void> => {
    if (pendingAction !== null || theme === preferencesEnvelope.preferences.theme) {
      return;
    }

    setPendingAction("theme");
    setFeedback("正在切换主题…");
    try {
      const next = await quotaGlanceApi.setTheme(theme);
      setPreferencesEnvelope((current) => mergePreferences(current, next));
      setFeedback("主题已更新");
    } catch (error: unknown) {
      setFeedback(localizedError(error));
    } finally {
      setPendingAction(null);
    }
  }, [pendingAction, preferencesEnvelope.preferences.theme]);

  const toggleAlwaysOnTop = useCallback(async (): Promise<void> => {
    if (pendingAction === "pin") {
      return;
    }

    setPendingAction("pin");
    const enabled = !windowState.alwaysOnTop;
    setFeedback(enabled ? "正在置顶窗口…" : "正在取消置顶…");
    try {
      const next = await quotaGlanceApi.setAlwaysOnTop(enabled);
      setWindowState((current) => mergeWindowState(current, next));
      setFeedback(enabled ? "窗口已置顶" : "已取消窗口置顶");
    } catch (error: unknown) {
      setFeedback(localizedError(error));
    } finally {
      setPendingAction(null);
    }
  }, [pendingAction, windowState.alwaysOnTop]);

  const toggleClickThrough = useCallback(async (): Promise<void> => {
    if (pendingAction === "clickThrough") {
      return;
    }

    setPendingAction("clickThrough");
    const enabled = !windowState.clickThrough;
    setFeedback(enabled ? "正在开启鼠标穿透…" : "正在关闭鼠标穿透…");
    try {
      const next = await quotaGlanceApi.setClickThrough(enabled);
      setWindowState((current) => mergeWindowState(current, next));
      setFeedback(enabled ? "已开启鼠标穿透，可从托盘恢复" : "已关闭鼠标穿透");
    } catch (error: unknown) {
      setFeedback(localizedError(error));
    } finally {
      setPendingAction(null);
    }
  }, [pendingAction, windowState.clickThrough]);

  const toggleLaunchAtLogin = useCallback(async (): Promise<void> => {
    if (pendingAction === "startup") {
      return;
    }

    setPendingAction("startup");
    const enabled = !preferencesEnvelope.preferences.startup.launchAtLogin;
    setFeedback(enabled ? "正在开启登录时启动…" : "正在关闭登录时启动…");
    try {
      const next = await quotaGlanceApi.setLaunchAtLogin(enabled);
      setPreferencesEnvelope((current) => mergePreferences(current, next));
      setFeedback(enabled ? "已开启登录时启动" : "已关闭登录时启动");
    } catch (error: unknown) {
      setFeedback(localizedError(error));
    } finally {
      setPendingAction(null);
    }
  }, [pendingAction, preferencesEnvelope.preferences.startup.launchAtLogin]);

  const quit = useCallback(async (): Promise<void> => {
    setFeedback("正在退出…");
    try {
      await quotaGlanceApi.quitApp();
    } catch (error: unknown) {
      setFeedback(localizedError(error));
    }
  }, []);

  return {
    snapshot,
    preferencesEnvelope,
    windowState,
    refreshState,
    pendingAction,
    feedback,
    refresh,
    setMode,
    setTheme,
    toggleAlwaysOnTop,
    toggleClickThrough,
    toggleLaunchAtLogin,
    quit,
  };
}
