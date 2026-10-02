import { Z_INDEX_REFRESH_THRESHOLD } from '../constants';
import { desktop } from '../shell/dom';
import { getWindowKindDefinition } from './registry';
// render.ts imports this module back; both sides only call each other at runtime, never at import time
import { renderWindows } from './render';
import { state } from './state';
import type { WindowId, WindowModel } from './state';

const closingListeners: Array<(win: WindowModel) => void> = [];

// For modules that hold per-window resources but do not own a window kind (e.g. the task runner)
export const addWindowClosingListener = (listener: (win: WindowModel) => void): void => {
  closingListeners.push(listener);
};

export const createWindowId = (): WindowId => {
  return crypto.randomUUID();
};

export const createWindowModel = (windowModel: WindowModel): void => {
  state.windows = state.windows.map((item) => ({
    ...item,
    isActive: false,
  }));
  state.windows.push(windowModel);
};

export const findWindowById = (id: WindowId): WindowModel | undefined => {
  return state.windows.find((item) => item.id === id);
};

export const syncFramePosition = (id: WindowId): void => {
  const target = findWindowById(id);
  if (!target) {
    return;
  }

  const frame = desktop.querySelector<HTMLElement>(`[data-window-id="${id}"]`);
  if (!frame) {
    return;
  }

  frame.style.left = `${target.x}px`;
  frame.style.top = `${target.y}px`;
};

const getTopZIndex = (): number => {
  if (state.windows.length === 0) {
    return 0;
  }

  return Math.max(...state.windows.map((item) => item.zIndex));
};

const refreshZIndices = (): void => {
  const sorted = [...state.windows].sort((a, b) => a.zIndex - b.zIndex);
  const nextById = new Map<WindowId, number>();

  sorted.forEach((win, index) => {
    nextById.set(win.id, index + 1);
  });

  state.windows = state.windows.map((win) => ({
    ...win,
    zIndex: nextById.get(win.id) ?? win.zIndex,
  }));
  state.nextZIndex = state.windows.length + 1;
};

const refreshZIndicesIfNeeded = (): void => {
  if (state.nextZIndex <= Z_INDEX_REFRESH_THRESHOLD) {
    return;
  }

  refreshZIndices();
};

export const getCenteredWindowPosition = (width: number, height: number): { x: number; y: number } => {
  const rect = desktop.getBoundingClientRect();
  const desktopWidth = Math.max(0, Math.floor(rect.width));
  const desktopHeight = Math.max(0, Math.floor(rect.height));

  return {
    x: Math.max(0, Math.floor((desktopWidth - width) / 2)),
    y: Math.max(0, Math.floor((desktopHeight - height) / 2)),
  };
};

/** @returns true if the windows were re-rendered */
export const bringToFrontIfNeeded = (id: WindowId): boolean => {
  const target = state.windows.find((item) => item.id === id);
  if (!target) {
    return false;
  }

  const maxZIndex = getTopZIndex();
  const shouldRaise = target.zIndex < maxZIndex;
  const shouldActivate = !target.isActive;

  if (!shouldRaise && !shouldActivate) {
    return false;
  }

  state.activeWindowId = id;
  state.windows = state.windows.map((item) => {
    if (item.id !== id) {
      return {
        ...item,
        isActive: false,
      };
    }

    return {
      ...item,
      zIndex: shouldRaise ? state.nextZIndex : item.zIndex,
      isActive: true,
    };
  });

  if (shouldRaise) {
    state.nextZIndex += 1;
    refreshZIndicesIfNeeded();
  }

  renderWindows();

  // Let the window kind focus its own content (terminal input, file manager list, ...)
  getWindowKindDefinition(target.kind).onActivated?.(id);
  return true;
};

export const closeWindow = (id: WindowId): void => {
  const closingWindow = findWindowById(id);
  if (!closingWindow) {
    return;
  }

  // Release per-window state while the model still exists (owners may read its position etc.)
  for (const listener of closingListeners) {
    listener(closingWindow);
  }
  getWindowKindDefinition(closingWindow.kind).onClosing?.(closingWindow);

  const remainingWindows = state.windows.filter((item) => item.id !== id);

  if (remainingWindows.length === 0) {
    state.windows = [];
    state.activeWindowId = null;
    renderWindows();
    return;
  }

  const currentActiveStillExists = state.activeWindowId !== id
    && state.activeWindowId !== null
    && remainingWindows.some((item) => item.id === state.activeWindowId);
  const nextActiveWindowId = currentActiveStillExists
    ? state.activeWindowId
    : remainingWindows.reduce((top, current) => {
      return current.zIndex > top.zIndex ? current : top;
    }).id;

  state.windows = remainingWindows.map((item) => ({
    ...item,
    isActive: item.id === nextActiveWindowId,
  }));
  state.activeWindowId = nextActiveWindowId;

  renderWindows();
};

export const updateWindowPosition = (id: WindowId, x: number, y: number): void => {
  state.windows = state.windows.map((item) => {
    if (item.id !== id) {
      return item;
    }

    return { ...item, x, y: Math.max(0, y) };
  });
};

export const setWindowPositionById = (id: WindowId, x: number, y: number): void => {
  updateWindowPosition(id, x, y);
  syncFramePosition(id);
};
