import iconTerminal from '../../assets/icons/terminal-2.svg?raw';
import {
  APP_IDS,
  INITIAL_TERMINAL_WINDOW_HEIGHT,
  INITIAL_TERMINAL_WINDOW_WIDTH,
  INITIAL_TERMINAL_X,
  INITIAL_TERMINAL_Y,
  TERMINAL_WINDOW_OFFSET_STEP,
} from '../../constants';
import { registerWindowKind } from '../../wm/registry';
import { renderWindows } from '../../wm/render';
import { state } from '../../wm/state';
import type { WindowModel } from '../../wm/state';
import { bringToFrontIfNeeded, createWindowId, createWindowModel } from '../../wm/windowManager';
import { runTerminalCommand } from './commands';
import {
  focusTerminalInput,
  terminalHistoryByWindow,
  terminalHistoryIndexByWindow,
  terminalOutputByWindow,
  terminalPendingInputByWindow,
  terminalState,
} from './state';

let nextTerminalSpawnIndex = 0;

const createTerminalPanel = (win: WindowModel): HTMLElement => {
  const panel = document.createElement('section');
  panel.className = 'terminal-panel';

  const screen = document.createElement('pre');
  screen.className = 'terminal-screen';
  screen.dataset.terminalScreenWindowId = win.id;
  screen.setAttribute('aria-live', 'polite');
  screen.textContent = (terminalOutputByWindow.get(win.id) ?? []).join('\n');

  const commandForm = document.createElement('form');
  commandForm.className = 'terminal-command-form';

  const commandInput = document.createElement('input');
  commandInput.className = 'terminal-command-input';
  commandInput.dataset.terminalInputWindowId = win.id;
  commandInput.type = 'text';
  commandInput.placeholder = 'Command';
  commandInput.setAttribute('aria-label', 'Command input');
  commandInput.addEventListener('focus', (event) => {
    event.preventDefault();
  });

  // Activate window and focus input when tapped in inactive terminal
  commandInput.addEventListener('pointerdown', (event) => {
    event.stopPropagation();
    if (!win.isActive) {
      bringToFrontIfNeeded(win.id);
      // Focus after render completes
      requestAnimationFrame(() => {
        const input = document.querySelector(`input[data-terminal-input-window-id="${win.id}"]`) as HTMLInputElement;
        input?.focus({ preventScroll: true });
      });
    } else {
      commandInput.focus({ preventScroll: true });
    }
  });

  // キーボード処理：上下キーで履歴操作
  commandInput.addEventListener('keydown', (event) => {
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      const history = terminalHistoryByWindow.get(win.id) ?? [];
      if (history.length === 0) return;

      let currentIndex = terminalHistoryIndexByWindow.get(win.id) ?? -1;

      // 初回ブラウズ時に入力途中のテキストを保存
      if (currentIndex === -1) {
        terminalPendingInputByWindow.set(win.id, commandInput.value);
      }

      const nextIndex = Math.min(currentIndex + 1, history.length - 1);
      terminalHistoryIndexByWindow.set(win.id, nextIndex);
      commandInput.value = history[nextIndex];
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      const history = terminalHistoryByWindow.get(win.id) ?? [];
      let currentIndex = terminalHistoryIndexByWindow.get(win.id) ?? -1;

      if (currentIndex > -1) {
        const nextIndex = currentIndex - 1;
        terminalHistoryIndexByWindow.set(win.id, nextIndex);

        // 履歴から抜けて入力中に戻る場合、保存されたテキストを復元
        if (nextIndex === -1) {
          commandInput.value = terminalPendingInputByWindow.get(win.id) ?? '';
        } else {
          commandInput.value = history[nextIndex];
        }
      }
    }
  });

  const executeButton = document.createElement('button');
  executeButton.className = 'terminal-command-button window-primary-button';
  executeButton.type = 'submit';
  executeButton.textContent = '実行';

  commandForm.addEventListener('submit', (event) => {
    event.preventDefault();
    runTerminalCommand(win.id, commandInput.value);
    commandInput.value = '';
    commandInput.focus({ preventScroll: true });
  });

  commandForm.append(commandInput, executeButton);
  panel.append(screen, commandForm);

  requestAnimationFrame(() => {
    screen.scrollTop = screen.scrollHeight;
  });

  return panel;
};

export const createTerminalWindow = (): void => {
  const id = createWindowId();
  const offset = nextTerminalSpawnIndex * TERMINAL_WINDOW_OFFSET_STEP;

  const windowModel: WindowModel = {
    id,
    appId: APP_IDS.TERMINAL,
    kind: 'terminal',
    title: 'Terminal',
    x: INITIAL_TERMINAL_X + offset,
    y: INITIAL_TERMINAL_Y + offset,
    width: INITIAL_TERMINAL_WINDOW_WIDTH,
    height: INITIAL_TERMINAL_WINDOW_HEIGHT,
    zIndex: state.nextZIndex,
    isActive: true,
  };

  nextTerminalSpawnIndex += 1;
  state.nextZIndex += 1;
  state.activeWindowId = id;
  createWindowModel(windowModel);
  terminalOutputByWindow.set(id, []);
  runTerminalCommand(id, 'VER', { echoInput: false, skipHistory: true });

  // Set as default terminal if none is set
  if (terminalState.defaultTerminalWindowId === null) {
    terminalState.defaultTerminalWindowId = id;
  }

  renderWindows();
  requestAnimationFrame(() => {
    focusTerminalInput(id);
  });
};

export const registerTerminalKind = (): void => {
  registerWindowKind('terminal', {
    icon: iconTerminal,
    typeLabel: 'Terminal',
    renderContent: createTerminalPanel,
    onActivated: focusTerminalInput,
    onClosing: (win) => {
      const id = win.id;
      terminalOutputByWindow.delete(id);
      terminalHistoryByWindow.delete(id);
      terminalHistoryIndexByWindow.delete(id);
      terminalPendingInputByWindow.delete(id);

      // Update default terminal if the closed window was the default
      if (terminalState.defaultTerminalWindowId === id) {
        const remainingTerminals = state.windows
          .filter((item) => item.kind === 'terminal' && item.id !== id)
          .sort((a, b) => {
            // Get the order they were created
            const terminalA = Array.from(terminalOutputByWindow.keys()).indexOf(a.id);
            const terminalB = Array.from(terminalOutputByWindow.keys()).indexOf(b.id);
            return terminalA - terminalB;
          });
        terminalState.defaultTerminalWindowId = remainingTerminals[0]?.id ?? null;
      }
    },
  });
};
