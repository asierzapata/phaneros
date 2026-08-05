import { isTauri } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { MainTab } from '@/types';

export const NAVIGATE_TO_TAB_EVENT = 'phaneros://navigate-to-tab';

/**
 * Called from the tray popup window: brings the main window to the front,
 * tells it which tab to show, and hides the tray popup. Each Tauri window
 * runs its own isolated React tree, so this can't be done through context
 * state alone — it has to cross the window boundary via a Tauri event.
 */
export const navigateMainWindowToTab = async (tab: MainTab): Promise<void> => {
  if (!isTauri()) return;

  const mainWindow = await WebviewWindow.getByLabel('main');
  if (mainWindow) {
    await mainWindow.show();
    await mainWindow.setFocus();
  }
  await emit(NAVIGATE_TO_TAB_EVENT, { tab });
  await getCurrentWindow().hide();
};
