import type { WindowId, WindowKind, WindowModel } from './state';

/**
 * What the window manager needs to know about a kind of window.
 * Each app registers its own kind, so the window manager never imports apps.
 */
export type WindowKindDefinition = {
  /** SVG markup for the title bar and the taskbar button */
  icon: string;
  /** Label of the taskbar group button */
  typeLabel: string;
  /**
   * Shown as a system modal: above every normal window, on a dimming overlay,
   * without title bar and without a taskbar button.
   */
  modal?: boolean;
  /** Builds the content below the title bar. Called on every render. */
  renderContent: (win: WindowModel) => HTMLElement;
  /** Called after the window became active or was re-rendered while active (e.g. to move focus) */
  onActivated?: (id: WindowId) => void;
  /** Called before the window model is removed, to release per-window state */
  onClosing?: (win: WindowModel) => void;
};

const definitions = new Map<WindowKind, WindowKindDefinition>();

export const registerWindowKind = (kind: WindowKind, definition: WindowKindDefinition): void => {
  definitions.set(kind, definition);
};

export const getWindowKindDefinition = (kind: WindowKind): WindowKindDefinition => {
  const definition = definitions.get(kind);
  if (!definition) {
    throw new Error(`Window kind is not registered: ${kind}`);
  }
  return definition;
};
