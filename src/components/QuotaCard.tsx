import { useRef } from "react";
import type { KeyboardEvent, MouseEvent } from "react";
import {
  formatExpirationAt,
  formatFreshness,
  formatPercent,
  formatResetAt,
  resolveQuotaPresentation,
  resolvePlanLabel,
  resolveQuotaTone,
  resolveQuotaWindowLabel,
} from "../models/quota-view";
import type {
  PendingActionName,
  Preferences,
  QuotaSnapshot,
  RefreshState,
  Theme,
  WidgetMode,
  WindowState,
} from "../types/ui";
import type { QuotaStatus, QuotaWindow } from "../types/quota";
import { Icon } from "./Icon";
import { IconButton } from "./IconButton";
import { ProgressBar } from "./ProgressBar";
import { SettingsPanel } from "./SettingsPanel";

interface QuotaCardProps {
  snapshot: QuotaSnapshot;
  preferences: Preferences;
  windowState: WindowState;
  refreshState: RefreshState;
  pendingAction: PendingActionName;
  feedback: string | null;
  settingsOpen: boolean;
  onSettingsOpenChange: (open: boolean) => void;
  onRefresh: () => void;
  onToggleAlwaysOnTop: () => void;
  onToggleClickThrough: () => void;
  onToggleLaunchAtLogin: () => void;
  onChangeMode: (mode: WidgetMode) => void;
  onChangeTheme: (theme: Theme) => void;
}

type EmptyQuotaStatus = Exclude<QuotaStatus, "ok" | "quotaReached" | "stale" | "loading">;

const STATUS_MESSAGES: Readonly<
  Record<EmptyQuotaStatus, {
    title: string;
    description: string;
  }>
> = {
  signedOut: {
    title: "尚未登录 Codex",
    description: "请先在 Codex 中登录，然后重新读取额度。",
  },
  apiKeyMode: {
    title: "当前为 API Key 模式",
    description: "订阅额度只适用于 ChatGPT 登录模式。",
  },
  sourceBusy: {
    title: "额度源暂时繁忙",
    description: "当前请求正在排队，请稍后重新读取。",
  },
  offline: {
    title: "当前网络不可用",
    description: "连接恢复后可重新读取额度。",
  },
  serviceUnavailable: {
    title: "额度服务暂时不可用",
    description: "服务恢复后可重新读取，不会显示虚假额度。",
  },
  appServerUnavailable: {
    title: "未找到可用的 Codex",
    description: "请安装或更新 ChatGPT 桌面应用（包含 Codex），或安装 Codex CLI 后重新读取。",
  },
  incompatible: {
    title: "当前 Codex 版本不兼容",
    description: "更新 QuotaGlance 或 Codex 后再试。",
  },
};

const FALLBACK_EMPTY_MESSAGE = {
  title: "暂无可用额度",
  description: "这次读取没有返回可展示的短周期或周额度，请稍后重新读取。",
};

function isEmptyQuotaStatus(status: QuotaStatus): status is EmptyQuotaStatus {
  return status in STATUS_MESSAGES;
}

interface QuotaWindowViewProps {
  quotaWindow: QuotaWindow;
  snapshot: QuotaSnapshot;
  preferences: Preferences;
  primaryWindow: QuotaWindow | null;
  resetCreditCount: number | null;
  resetCreditExpirations: string[];
}

function QuotaWindowView({
  quotaWindow,
  snapshot,
  preferences,
  primaryWindow,
  resetCreditCount,
  resetCreditExpirations,
}: QuotaWindowViewProps) {
  const tone = resolveQuotaTone(snapshot.status, quotaWindow.remainingPercent, preferences);
  const quotaLabel = resolveQuotaWindowLabel(quotaWindow);

  return (
    <section
      aria-labelledby="primary-quota-heading"
      className="quota-window quota-window--primary"
      data-tone={tone}
    >
      <h2 id="primary-quota-heading">
        <Icon name="calendar" size={19} />
        {quotaLabel}
      </h2>
      <p className="quota-window__remaining-label">剩余</p>
      <p className="quota-window__percent" aria-label={formatPercent(quotaWindow.remainingPercent)}>
        <strong>{quotaWindow.remainingPercent}</strong>
        <span>%</span>
      </p>
      <ProgressBar label={quotaLabel} tone={tone} value={quotaWindow.remainingPercent} />
      <p className="quota-window__reset">{formatResetAt(quotaWindow.resetsAt)}</p>
      {primaryWindow !== null || resetCreditCount !== null ? (
        <div className="quota-window__reference-details">
          {primaryWindow !== null ? (
            <div className="quota-window__detail" data-testid="short-term-quota">
              <span>{resolveQuotaWindowLabel(primaryWindow)}</span>
              <strong>{primaryWindow.remainingPercent}%</strong>
              <small>{formatResetAt(primaryWindow.resetsAt)}</small>
            </div>
          ) : null}
          {resetCreditCount !== null ? (
            <details className="quota-window__credits">
              <summary>
                <span>重置机会</span>
                <strong>{resetCreditCount} 次</strong>
              </summary>
              <div className="quota-window__credit-popover">
                {resetCreditExpirations.length > 0 ? (
                  resetCreditExpirations.map((expiration, index) => (
                    <p key={expiration}>
                      第 {index + 1} 次：{formatExpirationAt(expiration)}
                    </p>
                  ))
                ) : (
                  <p>当前只返回数量，未返回到期时间。</p>
                )}
              </div>
            </details>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}

function QuotaLoading() {
  return (
    <div aria-label="正在读取额度" className="quota-loading" role="status">
      <section className="quota-window quota-window--primary">
        <span className="skeleton skeleton--title" />
        <span className="skeleton skeleton--label" />
        <span className="skeleton skeleton--number" />
        <span className="skeleton skeleton--bar" />
        <span className="skeleton skeleton--meta" />
      </section>
      <span className="sr-only">正在读取额度</span>
    </div>
  );
}

interface EmptyStateProps {
  status: QuotaStatus;
  refreshing: boolean;
  onRefresh: () => void;
}

function EmptyState({ status, refreshing, onRefresh }: EmptyStateProps) {
  const message = isEmptyQuotaStatus(status) ? STATUS_MESSAGES[status] : FALLBACK_EMPTY_MESSAGE;
  return (
    <section className="empty-state" role="status">
      <span className="empty-state__icon">
        <Icon name="alert" size={24} />
      </span>
      <h2>{message.title}</h2>
      <p>{message.description}</p>
      <button className="primary-button" disabled={refreshing} onClick={onRefresh} type="button">
        {refreshing ? "正在读取…" : "重新读取"}
      </button>
    </section>
  );
}

function shouldIgnoreModeGesture(target: EventTarget): boolean {
  return target instanceof Element && target.closest("button, [role='dialog']") !== null;
}

export function QuotaCard({
  snapshot,
  preferences,
  windowState,
  refreshState,
  pendingAction,
  feedback,
  settingsOpen,
  onSettingsOpenChange,
  onRefresh,
  onToggleAlwaysOnTop,
  onToggleClickThrough,
  onToggleLaunchAtLogin,
  onChangeMode,
  onChangeTheme,
}: QuotaCardProps) {
  const settingsButtonRef = useRef<HTMLButtonElement>(null);
  const presentation = resolveQuotaPresentation(snapshot, preferences.widget.selectedQuota);
  const selectedWindow = presentation.weeklyWindow ?? presentation.primaryWindow;
  const shortTermDetail =
    presentation.weeklyWindow !== null ? presentation.primaryWindow : null;
  const refreshing = refreshState.phase === "refreshing" || pendingAction === "refresh";
  const refreshDisabled = refreshing || refreshState.phase === "cooldown";
  const hasReadings =
    selectedWindow !== null &&
    (snapshot.status === "ok" ||
      snapshot.status === "quotaReached" ||
      snapshot.status === "stale");
  const cardTone = selectedWindow
    ? resolveQuotaTone(snapshot.status, selectedWindow.remainingPercent, preferences)
    : "neutral";

  const handleKeyDown = (event: KeyboardEvent<HTMLElement>): void => {
    if (event.target !== event.currentTarget) {
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onChangeMode("orb");
    }
  };

  const handleDoubleClick = (event: MouseEvent<HTMLElement>): void => {
    if (!shouldIgnoreModeGesture(event.target)) {
      onChangeMode("orb");
    }
  };

  const closeSettings = (): void => {
    onSettingsOpenChange(false);
    window.requestAnimationFrame(() => settingsButtonRef.current?.focus());
  };

  const footerText =
    snapshot.status === "stale"
      ? `数据可能已过期 · ${formatFreshness(snapshot)}`
      : feedback ?? formatFreshness(snapshot).replace("最后更新：", "").replace("刚刚", "刚刚更新");

  return (
    <article
      aria-label="QuotaGlance 额度卡片，双击或按 Enter 收起为浮球"
      className="quota-card"
      data-status={snapshot.status}
      data-tone={cardTone}
      onDoubleClick={handleDoubleClick}
      onKeyDown={handleKeyDown}
      tabIndex={0}
    >
      <div aria-hidden="true" className="quota-card__aurora" />
      <header className="quota-card__header" data-tauri-drag-region>
        <div className="quota-card__identity" data-tauri-drag-region>
          <h1 data-tauri-drag-region>{resolvePlanLabel(snapshot).toUpperCase()}</h1>
        </div>
        <div className="quota-card__actions">
          <IconButton
            busy={refreshing}
            disabled={refreshDisabled}
            icon="refresh"
            label={refreshing ? "正在刷新额度" : refreshState.phase === "cooldown" ? "刷新冷却中" : "刷新额度"}
            onClick={onRefresh}
          />
          <IconButton
            aria-pressed={windowState.alwaysOnTop}
            busy={pendingAction === "pin"}
            disabled={pendingAction === "pin"}
            icon="pin"
            label={windowState.alwaysOnTop ? "取消窗口置顶" : "置顶窗口"}
            onClick={onToggleAlwaysOnTop}
          />
          <IconButton
            aria-controls="quota-settings-panel"
            aria-expanded={settingsOpen}
            buttonRef={settingsButtonRef}
            className={settingsOpen ? "icon-button--active" : ""}
            icon="settings"
            label={settingsOpen ? "关闭设置" : "打开设置"}
            onClick={() => (settingsOpen ? closeSettings() : onSettingsOpenChange(true))}
          />
        </div>
      </header>

      <div className={`quota-card__body${hasReadings || snapshot.status === "loading" ? " quota-card__body--single" : ""}`}>
        {snapshot.status === "loading" ? <QuotaLoading /> : null}
        {hasReadings ? (
          <QuotaWindowView
            primaryWindow={shortTermDetail}
            preferences={preferences}
            quotaWindow={selectedWindow}
            resetCreditCount={presentation.resetCreditCount}
            resetCreditExpirations={presentation.resetCreditExpirations}
            snapshot={snapshot}
          />
        ) : null}
        {snapshot.status !== "loading" && !hasReadings ? (
          <EmptyState onRefresh={onRefresh} refreshing={refreshing} status={snapshot.status} />
        ) : null}
      </div>

      <footer aria-live="polite" className="quota-card__footer">
        <span aria-hidden="true" className="status-dot" data-status={snapshot.status} />
        <span>{footerText}</span>
      </footer>

      {settingsOpen ? (
        <SettingsPanel
          onChangeMode={onChangeMode}
          onChangeTheme={onChangeTheme}
          onClose={closeSettings}
          onToggleAlwaysOnTop={onToggleAlwaysOnTop}
          onToggleClickThrough={onToggleClickThrough}
          onToggleLaunchAtLogin={onToggleLaunchAtLogin}
          pendingAction={pendingAction}
          preferences={preferences}
          windowState={windowState}
        />
      ) : null}
    </article>
  );
}
