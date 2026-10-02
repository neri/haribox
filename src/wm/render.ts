import iconClose from '../assets/icons/x.svg?raw';
import { desktop } from '../shell/dom';
import { renderTaskbarButtons } from '../shell/taskbar';
import { groupWindowsByKind } from './grouping';
import { getWindowKindDefinition } from './registry';
import { state } from './state';
import type { WindowId, WindowModel } from './state';
import { bringToFrontIfNeeded, closeWindow, findWindowById, syncFramePosition, updateWindowPosition } from './windowManager';

// ---- Title bar dragging

type DragState = {
  id: WindowId;
  pointerId: number;
  startClientX: number;
  startClientY: number;
  originX: number;
  originY: number;
};

let activeDrag: DragState | null = null;

const clamp = (value: number, min: number, max: number): number => {
  if (value < min) {
    return min;
  }
  if (value > max) {
    return max;
  }
  return value;
};

const clampPointerToDesktop = (clientX: number, clientY: number): { x: number; y: number } => {
  const rect = desktop.getBoundingClientRect();
  return {
    x: clamp(clientX, rect.left, rect.right),
    y: clamp(clientY, rect.top, rect.bottom),
  };
};

const startDrag = (event: PointerEvent, windowModel: WindowModel): void => {
  const clampedPointer = clampPointerToDesktop(event.clientX, event.clientY);

  activeDrag = {
    id: windowModel.id,
    pointerId: event.pointerId,
    startClientX: clampedPointer.x,
    startClientY: clampedPointer.y,
    originX: windowModel.x,
    originY: windowModel.y,
  };

  window.addEventListener('pointermove', handlePointerMove);
  window.addEventListener('pointerup', endDrag);
  window.addEventListener('pointercancel', endDrag);
};

const handlePointerMove = (event: PointerEvent): void => {
  if (!activeDrag || activeDrag.pointerId !== event.pointerId) {
    return;
  }

  const clampedPointer = clampPointerToDesktop(event.clientX, event.clientY);
  const diffX = clampedPointer.x - activeDrag.startClientX;
  const diffY = clampedPointer.y - activeDrag.startClientY;
  updateWindowPosition(activeDrag.id, activeDrag.originX + diffX, activeDrag.originY + diffY);
  syncFramePosition(activeDrag.id);
};

const endDrag = (event: PointerEvent): void => {
  if (!activeDrag || activeDrag.pointerId !== event.pointerId) {
    return;
  }

  activeDrag = null;
  window.removeEventListener('pointermove', handlePointerMove);
  window.removeEventListener('pointerup', endDrag);
  window.removeEventListener('pointercancel', endDrag);
};

// ---- Rendering

// Modals are stacked above every normal window (whose z-index stays far below this, see Z_INDEX_REFRESH_THRESHOLD).
// The factor of 2 leaves a free slot below each modal for the overlay.
const MODAL_Z_INDEX_BASE = 100_000;

const getModalFrameZIndex = (win: WindowModel): number => {
  return MODAL_Z_INDEX_BASE + win.zIndex * 2;
};

const isInteractiveContentTarget = (target: EventTarget | null): boolean => {
  if (!(target instanceof HTMLElement)) {
    return false;
  }

  return Boolean(target.closest('input, button, textarea, select, form'));
};

export const renderWindows = (): void => {
  // Save scroll position before rendering to prevent unwanted scrolling
  const scrollX = window.scrollX;
  const scrollY = window.scrollY;

  // ウィンドウグループを再計算してタスクバーボタンを更新
  // モーダルはタスクバーに表示しない
  state.windowGroups = groupWindowsByKind(state.windows.filter(win => !getWindowKindDefinition(win.kind).modal));
  renderTaskbarButtons();

  desktop.textContent = '';

  // The overlay goes right below the topmost modal (the one opened or raised last),
  // so it also covers any other modal that is still open
  const topModal = state.windows
    .filter(win => getWindowKindDefinition(win.kind).modal)
    .reduce<WindowModel | null>((top, win) => (!top || win.zIndex > top.zIndex ? win : top), null);

  if (topModal) {
    const overlay = document.createElement('div');
    overlay.className = 'system-modal-overlay';
    overlay.style.zIndex = String(getModalFrameZIndex(topModal) - 1);
    desktop.appendChild(overlay);
  }

  const sortedWindows = [...state.windows].sort((a, b) => a.zIndex - b.zIndex);
  for (const win of sortedWindows) {
    const frame = document.createElement('article');
    const isModal = getWindowKindDefinition(win.kind).modal === true;

    // Apply different class for system modal
    if (isModal) {
      frame.className = 'system-modal-frame';
      frame.classList.add('window-frame-active');
    } else {
      frame.className = 'window-frame';
      if (win.isActive) {
        frame.classList.add('window-frame-active');
      }
    }

    frame.dataset.windowId = win.id;
    frame.style.left = `${win.x}px`;
    frame.style.top = `${win.y}px`;
    frame.style.width = `${win.width}px`;
    frame.style.height = `${win.height}px`;
    frame.style.zIndex = String(isModal ? getModalFrameZIndex(win) : win.zIndex);

    frame.addEventListener('pointerdown', (event) => {
      if (isInteractiveContentTarget(event.target)) {
        return;
      }

      bringToFrontIfNeeded(win.id);
    });

    // Only render titlebar if not system modal
    if (!isModal) {
      const titleBar = document.createElement('header');
      titleBar.className = 'window-titlebar';
      titleBar.addEventListener('pointerdown', (event) => {
        if (event.button !== 0) {
          return;
        }

        bringToFrontIfNeeded(win.id);

        const latestWindow = findWindowById(win.id);
        if (!latestWindow) {
          return;
        }

        startDrag(event, latestWindow);
        event.preventDefault();
      });

      const title = document.createElement('span');
      title.className = 'window-title';
      title.textContent = win.title;

      const icon = document.createElement('span');
      icon.className = 'window-icon';
      icon.innerHTML = getWindowKindDefinition(win.kind).icon;

      const closeButton = document.createElement('button');
      closeButton.className = 'window-close-button';
      closeButton.type = 'button';
      closeButton.innerHTML = iconClose;
      closeButton.setAttribute('aria-label', `Close ${win.title}`);
      closeButton.addEventListener('pointerdown', (event) => {
        event.stopPropagation();
        event.preventDefault();
        closeWindow(win.id);
      });
      closeButton.addEventListener('click', (event) => {
        event.stopPropagation();
        closeWindow(win.id);
      });

      titleBar.append(icon, title, closeButton);
      frame.append(titleBar);
    } else {
      // System modal: render close button in top-right corner
      const closeButton = document.createElement('button');
      closeButton.className = 'system-modal-close-button';
      closeButton.type = 'button';
      closeButton.innerHTML = iconClose;
      closeButton.setAttribute('aria-label', 'Close');
      closeButton.addEventListener('pointerdown', (event) => {
        event.stopPropagation();
        event.preventDefault();
        closeWindow(win.id);
      });
      closeButton.addEventListener('click', (event) => {
        event.stopPropagation();
        closeWindow(win.id);
      });
      frame.append(closeButton);
    }

    frame.append(getWindowKindDefinition(win.kind).renderContent(win));
    desktop.appendChild(frame);
  }

  // Focus the appropriate element in the active window after rendering
  if (state.activeWindowId) {
    const activeWindow = findWindowById(state.activeWindowId);
    if (activeWindow) {
      getWindowKindDefinition(activeWindow.kind).onActivated?.(state.activeWindowId);
    }
  }

  // Restore scroll position to prevent unwanted scrolling
  window.scrollTo(scrollX, scrollY);
};
