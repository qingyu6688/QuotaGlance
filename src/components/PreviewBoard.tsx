import { useEffect, useRef, useState } from "react";
import { setBrowserMockScenario, type MockScenario } from "../api";
import { isMockScenario } from "../api/fixtures";
import { useQuotaGlance } from "../hooks/useQuotaGlance";
import type { Theme } from "../types/ui";
import { Icon } from "./Icon";
import { QuotaCard } from "./QuotaCard";
import { QuotaOrb } from "./QuotaOrb";

const SCENARIO_LABELS: ReadonlyArray<{ value: MockScenario; label: string }> = [
  { value: "ok", label: "正常" },
  { value: "warning", label: "提醒" },
  { value: "danger", label: "危险" },
  { value: "stale", label: "旧数据" },
  { value: "loading", label: "加载" },
  { value: "error", label: "错误" },
];

function initialScenario(): MockScenario {
  const candidate = new URLSearchParams(window.location.search).get("state");
  return isMockScenario(candidate) ? candidate : "ok";
}

function isTheme(value: string | null): value is Theme {
  return (
    value === "system" ||
    value === "aurora" ||
    value === "graphite" ||
    value === "paper" ||
    value === "sunset" ||
    value === "honey" ||
    value === "rose"
  );
}

export function PreviewBoard() {
  const controller = useQuotaGlance();
  const [scenario, setScenario] = useState<MockScenario>(initialScenario);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [orbMenuOpen, setOrbMenuOpen] = useState(false);
  const preferences = controller.preferencesEnvelope.preferences;
  const themeParameter = new URLSearchParams(window.location.search).get("theme");
  const initialTheme = isTheme(themeParameter) ? themeParameter : null;
  const initialThemeApplied = useRef(false);

  useEffect(() => {
    if (initialTheme === null || initialThemeApplied.current) {
      return;
    }

    initialThemeApplied.current = true;
    if (preferences.theme !== initialTheme) {
      void controller.setTheme(initialTheme);
    }
  }, [controller, initialTheme, preferences.theme]);

  const changeScenario = (nextScenario: MockScenario): void => {
    setOrbMenuOpen(false);
    setScenario(nextScenario);
    setBrowserMockScenario(nextScenario);
  };

  const openSettingsFromOrb = (): void => {
    setOrbMenuOpen(false);
    setSettingsOpen(true);
    void controller.setMode("card");
  };

  return (
    <main
      className="quota-app preview-board"
      data-current-mode={controller.windowState.mode}
      data-runtime="browser-mock"
      data-theme={preferences.theme}
    >
      <header className="preview-board__intro">
        <p>QuotaGlance · UI 基线</p>
        <div aria-label="预览状态" className="preview-board__states" role="group">
          {SCENARIO_LABELS.map((item) => (
            <button
              aria-pressed={scenario === item.value}
              key={item.value}
              onClick={() => changeScenario(item.value)}
              type="button"
            >
              {item.label}
            </button>
          ))}
        </div>
      </header>

      <div className="preview-board__stage">
        <div className="preview-board__card">
          <QuotaCard
            feedback={controller.feedback}
            onChangeMode={(nextMode) => {
              setSettingsOpen(false);
              void controller.setMode(nextMode);
            }}
            onChangeTheme={(theme) => void controller.setTheme(theme)}
            onRefresh={() => void controller.refresh()}
            onSettingsOpenChange={setSettingsOpen}
            onToggleAlwaysOnTop={() => void controller.toggleAlwaysOnTop()}
            onToggleClickThrough={() => void controller.toggleClickThrough()}
            pendingAction={controller.pendingAction}
            preferences={preferences}
            refreshState={controller.refreshState}
            settingsOpen={settingsOpen}
            snapshot={controller.snapshot}
            windowState={controller.windowState}
          />
        </div>
        <div className="preview-board__orb">
          <div className="preview-board__orb-surface">
            <QuotaOrb
              modeBusy={controller.pendingAction === "mode"}
              onExpand={() => void controller.setMode("card")}
              onOpenContextMenu={() => setOrbMenuOpen(true)}
              preferences={preferences}
              snapshot={controller.snapshot}
            />
            {orbMenuOpen ? (
              <div aria-label="浮球右键菜单预览" className="orb-context-preview" role="menu">
                <button onClick={openSettingsFromOrb} role="menuitem" type="button">
                  <Icon name="settings" size={16} />
                  <span>设置</span>
                </button>
                <button
                  onClick={() => {
                    setOrbMenuOpen(false);
                    void controller.quit();
                  }}
                  role="menuitem"
                  type="button"
                >
                  <Icon name="power" size={16} />
                  <span>退出</span>
                </button>
              </div>
            ) : null}
          </div>
          <p>双击展开 · 右键菜单</p>
        </div>
      </div>
    </main>
  );
}
