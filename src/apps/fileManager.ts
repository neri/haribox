import iconFolder from '../assets/icons/folder.svg?raw';
import { resumeAudioContext } from '../audio/audio';
import { APP_IDS, INITIAL_TERMINAL_Y } from '../constants';
import { getFileExtension, getFileIcon, parsePathExt, resolveSelectedIndex, searchExecutableFile } from '../fs/fileAssoc';
import type { FileSelection } from '../fs/fileAssoc';
import { toCanonicalFileKey } from '../fs/fileName';
import { listFilesSorted, onFileSystemChanged } from '../fs/fileSystem';
import type { FileEntry } from '../fs/fileSystem';
import { desktop } from '../shell/dom';
import { getPathExt, launchRustTaskWithCommand } from '../task/taskRunner';
import { registerWindowKind } from '../wm/registry';
import { renderWindows } from '../wm/render';
import { state } from '../wm/state';
import type { WindowId, WindowModel } from '../wm/state';
import { bringToFrontIfNeeded, createWindowId, createWindowModel, findWindowById } from '../wm/windowManager';
import { terminalState } from './terminal/state';

const fileManagerSelectionByWindowId = new Map<WindowId, FileSelection>();
const fileManagerListScrollByWindowId = new Map<WindowId, number>();
let fileManagerWindowId: WindowId | null = null;
let lastFileManagerPosition: { x: number; y: number } | null = null;
// Redraws the list of the currently rendered panel. Replaced on every render of the window.
let refreshOpenFileList: (() => void) | null = null;

const createFileManagerPanel = (win: WindowModel): HTMLElement => {
  const panel = document.createElement('section');
  panel.className = 'filemanager-panel';

  const list = document.createElement('div');
  list.className = 'filemanager-list';
  list.dataset.fileManagerListWindowId = win.id;

  // Restore selection from map, default to the first file
  let selection: FileSelection = fileManagerSelectionByWindowId.get(win.id) ?? { index: 0, key: null };
  let lastClickTime = 0;

  const select = (files: FileEntry[], index: number): void => {
    selection = { index, key: files[index] ? toCanonicalFileKey(files[index].name) : null };
    fileManagerSelectionByWindowId.set(win.id, selection);
  };

  const renderList = (): void => {
    // Save current scroll position before clearing
    const savedScroll = list.scrollTop;
    list.textContent = '';
    const files = listFilesSorted();

    if (files.length === 0) {
      const emptyMsg = document.createElement('div');
      emptyMsg.className = 'filemanager-empty';
      emptyMsg.textContent = '(no files)';
      list.appendChild(emptyMsg);
      return;
    }

    // The list may have changed since the last render (files added, removed or renamed)
    select(files, resolveSelectedIndex(files.map((file) => toCanonicalFileKey(file.name)), selection));
    const selectedIndex = selection.index;

    let selectedItem: HTMLElement | null = null;

    files.forEach((file, index) => {
      const item = document.createElement('div');
      item.className = 'filemanager-item';
      if (index === selectedIndex) {
        item.classList.add('filemanager-item-selected');
        // Add data attribute to indicate active state when rendered
        item.dataset.isSelected = 'true';
        selectedItem = item;
      }
      item.dataset.fileIndex = String(index);

      // Create icon element
      const iconContainer = document.createElement('span');
      iconContainer.className = 'filemanager-item-icon';
      iconContainer.innerHTML = getFileIcon(file.name, getPathExt());

      // Create filename element
      const nameContainer = document.createElement('span');
      nameContainer.className = 'filemanager-item-name';
      nameContainer.textContent = file.name;

      item.appendChild(iconContainer);
      item.appendChild(nameContainer);

      item.addEventListener('mousedown', (event) => {
        event.stopPropagation();
        event.preventDefault();
        list.focus({ preventScroll: true });
      });

      item.addEventListener('click', (event) => {
        event.stopPropagation();
        event.preventDefault();
        const now = Date.now();
        const isDoubleClick = now - lastClickTime < 300;
        lastClickTime = now;

        if (isDoubleClick) {
          executeFileFromFileManager(index);
        } else {
          select(files, index);
          renderList();
        }
      });

      list.appendChild(item);
    });

    // Auto-scroll to keep selected item in view
    if (selectedItem) {
      (selectedItem as HTMLElement).scrollIntoView({ block: 'nearest', behavior: 'auto' });
    }

    // Restore scroll position if it was saved
    const storedScroll = fileManagerListScrollByWindowId.get(win.id);
    if (storedScroll !== undefined) {
      requestAnimationFrame(() => {
        list.scrollTop = storedScroll;
      });
    } else if (savedScroll > 0) {
      // Keep the previously saved scroll position from before the clear
      requestAnimationFrame(() => {
        list.scrollTop = savedScroll;
      });
    }
  };

  const handleKeydown = (event: KeyboardEvent): void => {
    // Only handle keyboard input if this file manager window is active
    if (state.activeWindowId !== win.id) {
      return;
    }

    const files = listFilesSorted();

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      event.stopPropagation();
      select(files, Math.min(selection.index + 1, files.length - 1));
      renderList();
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      event.stopPropagation();
      select(files, Math.max(selection.index - 1, 0));
      renderList();
    } else if (event.key === 'Enter') {
      event.preventDefault();
      event.stopPropagation();
      if (selection.index >= 0) {
        executeFileFromFileManager(selection.index);
      }
    } else if (['ArrowLeft', 'ArrowRight', ' '].includes(event.key)) {
      // Prevent other arrow keys and space from causing scroll
      event.preventDefault();
      event.stopPropagation();
    }
  };

  const handleKeyup = (event: KeyboardEvent): void => {
    // Prevent default for keys that might cause scroll
    if (['ArrowDown', 'ArrowUp', 'ArrowLeft', 'ArrowRight', ' '].includes(event.key)) {
      event.preventDefault();
      event.stopPropagation();
    }
  };

  list.addEventListener('keydown', handleKeydown);
  list.addEventListener('keyup', handleKeyup);
  list.addEventListener('scroll', () => {
    fileManagerListScrollByWindowId.set(win.id, list.scrollTop);
  });
  list.addEventListener('focusout', () => {
    // Re-focus the list when focus is lost (but not when switching windows)
    if (state.activeWindowId === win.id) {
      setTimeout(() => list.focus({ preventScroll: true }), 0);
    }
  });
  list.addEventListener('click', () => {
    // Ensure focus is retained on click
    list.focus({ preventScroll: true });
  });
  list.setAttribute('tabindex', '0');
  list.setAttribute('role', 'listbox');
  list.style.outline = 'none';

  panel.appendChild(list);
  renderList();
  refreshOpenFileList = renderList;

  return panel;
};

const executeFileFromFileManager = (index: number): void => {
  const files = listFilesSorted();
  if (index < 0 || index >= files.length) {
    return;
  }

  const file = files[index];
  const ext = getFileExtension(file.name);
  const pathExt = getPathExt();
  const extensions = parsePathExt(pathExt);

  // Check if file can be executed
  if (extensions.includes(ext)) {
    // Launch as task
    launchRustTaskWithCommand(terminalState.defaultTerminalWindowId ?? '', file.name, file.name);
    return;
  }

  // Check if it's an image file
  if (ext === '.bmp' || ext === '.jpg') {
    // Search for gview file
    const gviewSearch = searchExecutableFile('gview', pathExt);
    if (gviewSearch) {
      launchRustTaskWithCommand(terminalState.defaultTerminalWindowId ?? '', gviewSearch, `gview ${file.name}`);
    } else {
      showErrorDialog(`このファイルは実行できません: gview`);
    }
    return;
  }

  // Check if it's a text file
  if (ext === '.txt') {
    // Search for tview file
    const tviewSearch = searchExecutableFile('tview', pathExt);
    if (tviewSearch) {
      launchRustTaskWithCommand(terminalState.defaultTerminalWindowId ?? '', tviewSearch, `tview -w80 -h24 ${file.name}`);
    } else {
      showErrorDialog(`このファイルは実行できません: tview`);
    }
    return;
  }

  // Check if it's a music file
  if (ext === '.mml') {
    // Resume audio context for playback
    resumeAudioContext();

    // Search for mmlplay file
    const mmlplaySearch = searchExecutableFile('mmlplay', pathExt);
    if (mmlplaySearch) {
      launchRustTaskWithCommand(terminalState.defaultTerminalWindowId ?? '', mmlplaySearch, `mmlplay ${file.name}`);
    } else {
      showErrorDialog(`このファイルは実行できません: mmlplay`);
    }
    return;
  }

  // Show error dialog
  showErrorDialog(`このファイルは実行できません: ${file.name}`);
};

const showErrorDialog = (message: string): void => {
  window.alert(message);
};

const focusFileManagerList = (windowId: WindowId): void => {
  const list = desktop.querySelector<HTMLDivElement>(`[data-file-manager-list-window-id="${windowId}"]`);
  if (list) {
    // Use setTimeout to ensure focus is set after render completes
    setTimeout(() => {
      list.focus({ preventScroll: true });
    }, 0);
  }
};

export const createFileManagerWindow = (): void => {
  // If file manager window already exists, bring it to front
  if (fileManagerWindowId && findWindowById(fileManagerWindowId)) {
    bringToFrontIfNeeded(fileManagerWindowId);
    focusFileManagerList(fileManagerWindowId);
    return;
  }

  const id = createWindowId();

  // Calculate position: use last position if available, otherwise right-top
  let position: { x: number; y: number };
  if (lastFileManagerPosition) {
    position = lastFileManagerPosition;
  } else {
    // Right-top position (20px margin from right edge)
    const rect = desktop.getBoundingClientRect();
    const desktopWidth = Math.max(0, Math.floor(rect.width));
    const rightMargin = 20;
    position = {
      x: Math.max(0, desktopWidth - 300 - rightMargin),
      y: INITIAL_TERMINAL_Y,
    };
  }

  const windowModel: WindowModel = {
    id,
    appId: APP_IDS.FILE_MANAGER,
    kind: 'filemanager',
    title: 'File System',
    x: position.x,
    y: position.y,
    width: 300,
    height: 400,
    zIndex: state.nextZIndex,
    isActive: true,
  };

  fileManagerWindowId = id;
  state.nextZIndex += 1;
  state.activeWindowId = id;
  createWindowModel(windowModel);
  renderWindows();
  focusFileManagerList(id);
};

export const registerFileManagerKind = (): void => {
  registerWindowKind('filemanager', {
    icon: iconFolder,
    typeLabel: 'File System',
    renderContent: createFileManagerPanel,
    onActivated: focusFileManagerList,
    onClosing: (win) => {
      fileManagerSelectionByWindowId.delete(win.id);
      fileManagerListScrollByWindowId.delete(win.id);

      if (fileManagerWindowId === win.id) {
        // Remember the position so the window reopens where it was closed
        lastFileManagerPosition = { x: win.x, y: win.y };
        fileManagerWindowId = null;
        refreshOpenFileList = null;
      }
    },
  });

  // ファイルシステム監視: 他のウィンドウやタスクによる作成・削除・リネームを一覧に反映する
  onFileSystemChanged(() => {
    refreshOpenFileList?.();
  });
};
