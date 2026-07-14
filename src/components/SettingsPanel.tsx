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
  onClose,
  onChangeMode,
  onChangeTheme,
  onToggleAlwaysOnTop,
  onToggleClickThrough,
  onToggleLaunchAtLogin,
}: SettingsPanelProps) {
  return (
    <section
      aria-labelledby="quota-settings-title"
      className="settings-panel"
      id="quota-settings-panel"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          onClose();
        }
      }}
      role="dialog"
    >
      <header className="settings-panel__header">
        <h2 id="quota-settings-title">设置</h2>
        <IconButton autoFocus icon="close" label="关闭设置" onClick={onClose} />
      </header>

      <ThemeControl
        disabled={pendingAction !== null}
        onChange={onChangeTheme}
        value={preferences.theme}
      />

      <div aria-label="显示模式" className="mode-control" role="group">
        <button
          aria-pressed={windowState.mode === "card"}
          disabled={pendingAction === "mode"}
          onClick={() => onChangeMode("card")}
          type="button"
        >
          卡片
        </button>
        <button
          aria-pressed={windowState.mode === "orb"}
          disabled={pendingAction === "mode"}
          onClick={() => onChangeMode("orb")}
          type="button"
        >
          浮球
        </button>
      </div>

      <SwitchRow
        checked={windowState.alwaysOnTop}
        description="始终显示在其他窗口上方"
        disabled={pendingAction === "pin"}
        icon="pin"
        label="窗口置顶"
        onChange={onToggleAlwaysOnTop}
      />
      <SwitchRow
        checked={windowState.clickThrough}
        description="开启后可从系统托盘恢复"
        disabled={pendingAction === "clickThrough"}
        icon="mouse"
        label="鼠标穿透"
        onChange={onToggleClickThrough}
      />
      <SwitchRow
        checked={preferences.startup.launchAtLogin}
        description="登录系统后自动启动 QuotaGlance"
        disabled={pendingAction === "startup"}
        icon="power"
        label="登录时启动"
        onChange={onToggleLaunchAtLogin}
      />

    </section>
  );
}
