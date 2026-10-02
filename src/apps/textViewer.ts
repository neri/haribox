import iconFileText from '../assets/icons/file-text.svg?raw';
import { APP_IDS, TASKBAR_HEIGHT_PX, TERMINAL_WINDOW_OFFSET_STEP } from '../constants';
import { toCanonicalFileKey } from '../fs/fileName';
import { decodeFileAsText, fileSystem } from '../fs/fileSystem';
import { registerWindowKind } from '../wm/registry';
import { renderWindows } from '../wm/render';
import { state } from '../wm/state';
import type { WindowId, WindowModel } from '../wm/state';
import {
  bringToFrontIfNeeded,
  createWindowId,
  createWindowModel,
  findWindowById,
  getCenteredWindowPosition,
} from '../wm/windowManager';

const textViewerScrollByWindowId = new Map<WindowId, number>();
const textViewerWindowByFilename = new Map<string, WindowId>();

const createTextViewerPanel = (win: WindowModel): HTMLElement => {
  const panel = document.createElement('section');
  panel.className = 'textviewer-panel';

  const textContainer = document.createElement('div');
  textContainer.className = 'textviewer-container';

  const textArea = document.createElement('textarea');
  textArea.className = 'textviewer-textarea';
  textArea.readOnly = true;
  textArea.setAttribute('aria-label', 'File content');
  textArea.setAttribute('aria-readonly', 'true');

  // Extract filename from window title
  const filename = win.title;

  // Read file content and populate textarea
  const entry = fileSystem.get(toCanonicalFileKey(filename));
  if (entry) {
    try {
      const content = decodeFileAsText(entry);
      textArea.value = content;
    } catch (error) {
      textArea.value = `[Error decoding file: ${filename}]`;
      console.error(`Failed to decode file ${filename}:`, error);
    }
  } else {
    textArea.value = `[File not found: ${filename}]`;
  }

  // Restore scroll position if it was saved
  const savedScrollTop = textViewerScrollByWindowId.get(win.id);
  if (savedScrollTop !== undefined) {
    requestAnimationFrame(() => {
      textArea.scrollTop = savedScrollTop;
    });
  }

  // Save scroll position when user scrolls
  textArea.addEventListener('scroll', () => {
    textViewerScrollByWindowId.set(win.id, textArea.scrollTop);
  });

  // Activate window when textarea is clicked/focused
  textArea.addEventListener('pointerdown', (event) => {
    event.stopPropagation();
    if (!win.isActive) {
      bringToFrontIfNeeded(win.id);
      // Focus after render completes
      requestAnimationFrame(() => {
        const textarea = document.querySelector(`textarea[data-textviewer-textarea-window-id="${win.id}"]`) as HTMLTextAreaElement;
        textarea?.focus({ preventScroll: true });
      });
    } else {
      textArea.focus({ preventScroll: true });
    }
  });

  textArea.setAttribute('data-textviewer-textarea-window-id', win.id);

  textContainer.appendChild(textArea);
  panel.appendChild(textContainer);

  return panel;
};

/**
 * [Deprecated] テキストビューアウィンドウの作成関数
 * 
 * 現在は未使用（.txt ファイルは tview コマンド経由で実行される）
 * 将来的にテキストビューア UI を復活させる場合に使用予定
 */
export const createTextViewerWindow = (filename: string): void => {
  // If text viewer for this file already exists, bring it to front
  const existingWindowId = textViewerWindowByFilename.get(filename);
  if (existingWindowId && findWindowById(existingWindowId)) {
    bringToFrontIfNeeded(existingWindowId);
    return;
  }

  const id = createWindowId();
  const centered = getCenteredWindowPosition(600, 400);
  const offset = textViewerWindowByFilename.size * TERMINAL_WINDOW_OFFSET_STEP;

  const windowModel: WindowModel = {
    id,
    appId: APP_IDS.TEXT_VIEWER,
    kind: 'textviewer',
    title: filename,
    x: Math.min(centered.x + offset, window.innerWidth - 200),
    y: Math.min(centered.y + offset, window.innerHeight - TASKBAR_HEIGHT_PX - 100),
    width: 600,
    height: 400,
    zIndex: state.nextZIndex,
    isActive: true,
  };

  textViewerWindowByFilename.set(filename, id);
  state.nextZIndex += 1;
  state.activeWindowId = id;
  createWindowModel(windowModel);
  renderWindows();
};

export const registerTextViewerKind = (): void => {
  registerWindowKind('textviewer', {
    icon: iconFileText,
    typeLabel: 'Text Viewer',
    renderContent: createTextViewerPanel,
    onClosing: (win) => {
      textViewerScrollByWindowId.delete(win.id);
      for (const [filename, windowId] of textViewerWindowByFilename.entries()) {
        if (windowId === win.id) {
          textViewerWindowByFilename.delete(filename);
        }
      }
    },
  });
};
