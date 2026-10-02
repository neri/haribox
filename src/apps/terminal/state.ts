import { TERMINAL_MAX_LINES } from '../../constants';
import { desktop } from '../../shell/dom';
import type { WindowId } from '../../wm/state';
import { appendTextToLines } from './buffer';

export const terminalOutputByWindow = new Map<WindowId, string[]>();
export const terminalHistoryByWindow = new Map<WindowId, string[]>();
export const terminalHistoryIndexByWindow = new Map<WindowId, number>();
export const terminalPendingInputByWindow = new Map<WindowId, string>();

export const terminalState: { defaultTerminalWindowId: WindowId | null } = {
  defaultTerminalWindowId: null,
};

export const focusTerminalInput = (windowId: WindowId): void => {
  const input = desktop.querySelector<HTMLInputElement>(`[data-terminal-input-window-id="${windowId}"]`);
  input?.focus({ preventScroll: true });
};

export const syncTerminalView = (windowId: WindowId): void => {
  const output = terminalOutputByWindow.get(windowId) ?? [];
  const screen = desktop.querySelector<HTMLPreElement>(`[data-terminal-screen-window-id="${windowId}"]`);
  if (!screen) {
    return;
  }

  screen.textContent = output.join('\n');
  screen.scrollTop = screen.scrollHeight;
};

export const appendTerminalText = (windowId: WindowId, text: string): void => {
  const current = terminalOutputByWindow.get(windowId) ?? [];
  appendTextToLines(current, text, TERMINAL_MAX_LINES);
  terminalOutputByWindow.set(windowId, current);
  syncTerminalView(windowId);
};

export const appendTerminalLine = (windowId: WindowId, text: string): void => {
  appendTerminalText(windowId, `${text}\n`);
};
