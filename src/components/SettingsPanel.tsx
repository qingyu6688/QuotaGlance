import { useEffect, useRef } from "react";
import type { KeyboardEvent } from "react";
import type {
  PendingActionName,
  Preferences,
  Theme,
  WidgetMode,
  WindowState,
} from "../types/ui";
import { Icon, type IconName } from "./Icon";
import { IconButton } from "./IconButton";
import { ThemeControl } from "./ThemeControl";

interface SettingsPanelProps {
  preferences: Preferences;
  windowState: WindowState;
  pendingAction: PendingActionName;
  feedback: string | null;
  onClose: () => void;
  onChangeMode: (mode: WidgetMode) => void;
  onChangeTheme: (theme: Theme) => void;
  onToggleAlwaysOnTop: () => void;
  onToggleClickThrough: () => void;
  onToggleLaunchAtLogin: () => void;
}

interface SwitchRowProps {
  label: string;
  description: string;
  icon: IconName;
  checked: boolean;
  disabled: boolean;
  onChange: () => void;
}

const FOCUSABLE_SELECTOR =
  'button:not(:disabled), [href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])';

function SwitchRow({ label, description, icon, checked, disabled, onChange }: SwitchRowProps) {
  return (
    <div className="settings-row">
      <Icon name={icon} size={17} />
      <span className="settings-row__label">
        <strong>{label}</strong>
        <small>{description}</small>
      </span>
      <button
        aria-checked={checked}
        aria-label={label}
        className="switch"
        disabled={disabled}
        onClick={onChange}
        role="switch"
        type="button"
      >
        <span className="switch__thumb" />
      </button>
    </div>
  );
}

export function SettingsPanel({
  preferences,
  windowState,
  pendingAction,
  feedback,
  onClose,
  onChangeMode,
  onChangeTheme,
  onToggleAlwaysOnTop,
  onToggleClickThrough,
  onToggleLaunchAtLogin,
}: SettingsPanelProps) {
  const panelRef = useRef<HTMLElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (pendingAction !== null && panelRef.current?.contains(document.activeElement)) {
      closeButtonRef.current?.focus();
    }
  }, [pendingAction]);

  const handleKeyDown = (event: KeyboardEvent<HTMLElement>): void => {
    if (event.key === "Escape") {
      event.stopPropagation();
      onClose();
      return;
    }

    if (event.key !== "Tab") {
      return;
    }

    const panel = panelRef.current;
    if (panel === null) {
      return;
    }

    const focusableElements = Array.from(
      panel.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR),
    ).filter((element) => element.tabIndex >= 0);
    const firstElement = focusableElements[0];
    const lastElement = focusableElements.at(-1);
    if (firstElement === undefined || lastElement === undefined) {
      return;
    }

    const activeElement = document.activeElement;
    if (!(activeElement instanceof HTMLElement) || !focusableElements.includes(activeElement)) {
      event.preventDefault();
      (event.shiftKey ? lastElement : firstElement).focus();
      return;
    }

    if (event.shiftKey && activeElement === firstElement) {
      event.preventDefault();
      lastElement.focus();
      return;
    }

    if (!event.shiftKey && activeElement === lastElement) {
      event.preventDefault();
      firstElement.focus();
    }
  };

  return (
    <section
      aria-modal="true"
      aria-labelledby="quota-settings-title"
      className="settings-panel"
      id="quota-settings-panel"
      onKeyDown={handleKeyDown}
      ref={panelRef}
      role="dialog"
    >
      <header className="settings-panel__header">
        <h2 id="quota-settings-title">设置</h2>
        {feedback !== null ? (
          <div
            aria-live="polite"
            className="settings-panel__feedback"
            data-testid="operation-feedback"
            role="status"
          >
            <Icon name="info" size={13} />
            <span>{feedback}</span>
          </div>
        ) : null}
        <IconButton
          autoFocus
          buttonRef={closeButtonRef}
          icon="close"
          label="关闭设置"
          onClick={onClose}
        />
      </header>

      <ThemeControl
        disabled={pendingAction !== null}
        onChange={onChangeTheme}
        value={preferences.theme}
      />

      <section aria-labelledby="mode-setting-title" className="mode-setting">
        <strong id="mode-setting-title">模式</strong>
        <div aria-label="显示模式" className="mode-control" role="group">
          <button
            aria-pressed={windowState.mode === "card"}
            disabled={pendingAction !== null}
            onClick={() => onChangeMode("card")}
            type="button"
          >
            卡片
          </button>
          <button
            aria-pressed={windowState.mode === "orb"}
            disabled={pendingAction !== null}
            onClick={() => onChangeMode("orb")}
            type="button"
          >
            浮球
          </button>
        </div>
      </section>

      <SwitchRow
        checked={windowState.alwaysOnTop}
        description="始终显示在其他窗口上方"
        disabled={pendingAction !== null}
        icon="pin"
        label="窗口置顶"
        onChange={onToggleAlwaysOnTop}
      />
      <SwitchRow
        checked={windowState.clickThrough}
        description="开启后可从系统托盘恢复"
        disabled={pendingAction !== null}
        icon="mouse"
        label="鼠标穿透"
        onChange={onToggleClickThrough}
      />
      <SwitchRow
        checked={preferences.startup.launchAtLogin}
        description="登录系统后自动启动 QuotaGlance"
        disabled={pendingAction !== null}
        icon="power"
        label="登录时启动"
        onChange={onToggleLaunchAtLogin}
      />
    </section>
  );
}
