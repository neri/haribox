import type { KeyboardEventMessage } from '../protocol';
import { closeMenu, isMenuOpen } from '../shell/taskbarWidgets';
import { getWorkerByWindowId } from '../task/taskRunner';
import { state } from '../wm/state';
import { findWindowById } from '../wm/windowManager';
import { applyModifierKey } from './modifiers';

// Global modifier state management
let modifierBitmap: number = 0;

/**
 * Forward a key event to the Worker of the active Canvas window.
 * @returns true if the event was forwarded
 */
const forwardToActiveCanvasTask = (event: KeyboardEvent, eventType: 'keydown' | 'keyup'): boolean => {
  // Update modifier bitmap
  modifierBitmap = applyModifierKey(modifierBitmap, event.code, eventType === 'keydown');

  if (!state.activeWindowId) {
    return false;
  }

  const activeWindow = findWindowById(state.activeWindowId);
  if (!activeWindow || activeWindow.kind !== 'canvas') {
    return false;
  }

  const worker = getWorkerByWindowId(state.activeWindowId);
  if (!worker) {
    return false;
  }

  const keyboardEvent: KeyboardEventMessage = {
    type: 'keyboardEvent',
    windowId: state.activeWindowId,
    eventType,
    key: event.key,
    code: event.code,
    keyCode: event.keyCode,
    ctrlKey: event.ctrlKey,
    shiftKey: event.shiftKey,
    altKey: event.altKey,
    metaKey: event.metaKey,
    isAutoRepeat: event.repeat,
    modifierBitmap,
  };
  worker.postMessage(keyboardEvent);
  event.preventDefault();
  return true;
};

export const initKeyboard = (): void => {
  // Reset modifier bitmap when window loses focus
  window.addEventListener('blur', () => {
    modifierBitmap = 0;
  });

  // Keyboard input forwarding to Canvas window Worker
  document.addEventListener('keydown', (event: KeyboardEvent) => {
    if (forwardToActiveCanvasTask(event, 'keydown')) {
      return;
    }

    // Handle menu Escape key
    if (event.key === 'Escape' && isMenuOpen()) {
      closeMenu();
    }
  });

  document.addEventListener('keyup', (event: KeyboardEvent) => {
    forwardToActiveCanvasTask(event, 'keyup');
  });
};
