import type { Menu } from "@tauri-apps/api/menu";
import type { QuotaGlanceApi } from "./contract";
import { BrowserMockClient } from "./mock-client";
import { tauriClient } from "./tauri-client";
import type { MockScenario } from "./fixtures";
import type { IpcError } from "../types/quota";

interface OrbContextMenuActions {
  openSettings: () => void;
  quit: () => void;
}

let activeOrbMenuActions: OrbContextMenuActions | null = null;
let orbContextMenuPromise: Promise<Menu> | null = null;

export const isTauriRuntime =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const browserMockClient = new BrowserMockClient();

export const quotaGlanceApi: QuotaGlanceApi = isTauriRuntime
  ? tauriClient
  : browserMockClient;

async function getOrbContextMenu(): Promise<Menu> {
  if (orbContextMenuPromise === null) {
    orbContextMenuPromise = import("@tauri-apps/api/menu")
      .then(({ Menu }) =>
        Menu.new({
          items: [
            {
              id: "orb-settings",
              text: "设置",
              action: () => activeOrbMenuActions?.openSettings(),
            },
            {
              id: "orb-quit",
              text: "退出",
              action: () => activeOrbMenuActions?.quit(),
            },
          ],
        }),
      )
      .catch((error: unknown) => {
        orbContextMenuPromise = null;
        throw error;
      });
  }

  return orbContextMenuPromise;
}

export async function showOrbContextMenu(actions: OrbContextMenuActions): Promise<boolean> {
  if (!isTauriRuntime) {
    return false;
  }

  activeOrbMenuActions = actions;
  try {
    const menu = await getOrbContextMenu();
    await menu.popup();
    return true;
  } catch {
    return false;
  }
}

export function setBrowserMockScenario(scenario: MockScenario): void {
  if (!isTauriRuntime) {
    browserMockClient.setScenario(scenario);
  }
}

export function isIpcError(value: unknown): value is IpcError {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    typeof value.code === "string" &&
    "messageKey" in value &&
    typeof value.messageKey === "string"
  );
}

export type { MockScenario } from "./fixtures";
