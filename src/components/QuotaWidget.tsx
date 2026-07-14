import { useState } from "react";
import { isTauriRuntime, showOrbContextMenu, startWidgetDragging } from "../api";
import { useQuotaGlance } from "../hooks/useQuotaGlance";
import { QuotaCard } from "./QuotaCard";
import { QuotaOrb } from "./QuotaOrb";

export function QuotaWidget() {
  const controller = useQuotaGlance();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const preferences = controller.preferencesEnvelope.preferences;
  const mode = controller.windowState.mode;

  const showCard = mode !== "orb";

  const openSettingsFromOrb = (): void => {
    setSettingsOpen(true);
    void controller.setMode("card");
  };

  const openOrbMenu = (): void => {
    void showOrbContextMenu({
      openSettings: openSettingsFromOrb,
      quit: () => void controller.quit(),
    }).then((shown) => {
      if (!shown) {
        openSettingsFromOrb();
      }
    });
  };

  return (
    <main
      className="quota-app quota-app--widget"
      data-runtime={isTauriRuntime ? "tauri" : "browser-mock"}
      data-theme={preferences.theme}
    >
      {showCard ? (
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
          onToggleLaunchAtLogin={() => void controller.toggleLaunchAtLogin()}
          pendingAction={controller.pendingAction}
          preferences={preferences}
          refreshState={controller.refreshState}
          settingsOpen={settingsOpen}
          snapshot={controller.snapshot}
          windowState={controller.windowState}
        />
      ) : (
        <QuotaOrb
          modeBusy={controller.pendingAction === "mode"}
          onExpand={() => void controller.setMode("card")}
          onOpenContextMenu={openOrbMenu}
          onStartDragging={() => void startWidgetDragging()}
          preferences={preferences}
          snapshot={controller.snapshot}
        />
      )}
      {!isTauriRuntime ? <p className="mock-runtime-badge">浏览器模拟数据</p> : null}
    </main>
  );
}
