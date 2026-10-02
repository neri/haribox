// Importing files by drag and drop, and the system modal that guides and reports it
import iconAppWindow from '../assets/icons/app-window.svg?raw';
import { APP_IDS, FILE_IMPORT_SIZE_LIMIT_BYTES } from '../constants';
import { normalizePathLikeName } from '../fs/fileName';
import { fileSystem, upsertFile } from '../fs/fileSystem';
import { registerWindowKind } from '../wm/registry';
import { renderWindows } from '../wm/render';
import { state } from '../wm/state';
import type { WindowId, WindowModel } from '../wm/state';
import {
  bringToFrontIfNeeded,
  closeWindow,
  createWindowId,
  createWindowModel,
  findWindowById,
  getCenteredWindowPosition,
} from '../wm/windowManager';

// ---- System modal

let systemModalWindowId: WindowId | null = null;
const systemModalModeByWindowId = new Map<WindowId, { mode: 'import' | 'complete'; filename?: string; error?: string }>();

const createSystemModalPanel = (win: WindowModel): HTMLElement => {
  const panel = document.createElement('section');
  panel.className = 'system-modal-panel';

  const content = document.createElement('div');
  content.className = 'system-modal-content';

  const modalMode = systemModalModeByWindowId.get(win.id);
  const mode = modalMode?.mode || 'import';
  const filename = modalMode?.filename;
  const error = modalMode?.error;

  // Mode 1: Import (no title, no OK button)
  if (mode === 'import') {
    const message = document.createElement('p');
    message.className = 'system-modal-message system-modal-message-import';
    message.textContent = 'ここにファイルをドロップすると取り込めます。';
    content.appendChild(message);
  } else {
    // Mode 2: Complete or Error (title not shown, but message and OK button)
    // Add spacer for visual balance
    const spacer = document.createElement('div');
    spacer.className = 'system-modal-spacer';
    content.appendChild(spacer);

    const message = document.createElement('p');
    message.className = 'system-modal-message system-modal-message-complete';
    if (error) {
      // Error message
      message.textContent = `取り込みに失敗しました: ${error}`;
    } else if (filename) {
      // Success message
      message.textContent = `${filename} を取り込みました。`;
    } else {
      // Fallback
      message.textContent = 'ファイルを取り込みました。';
    }
    content.appendChild(message);

    const buttonContainer = document.createElement('div');
    buttonContainer.className = 'system-modal-button-container';

    const okButton = document.createElement('button');
    okButton.type = 'button';
    okButton.className = 'system-modal-button';
    okButton.textContent = 'OK';
    okButton.addEventListener('click', (event) => {
      event.stopPropagation();
      closeWindow(win.id);
    });

    buttonContainer.appendChild(okButton);
    content.appendChild(buttonContainer);
  }

  panel.appendChild(content);

  return panel;
};

const SYSTEM_MODAL_WIDTH = 480;
const SYSTEM_MODAL_HEIGHT = 200;

// Opens the modal in import mode. The drop handler switches it to complete mode in place.
const createSystemModalWindow = (): void => {
  // If system modal already exists, reset it to import mode and bring to front
  if (systemModalWindowId && findWindowById(systemModalWindowId)) {
    systemModalModeByWindowId.set(systemModalWindowId, { mode: 'import' });
    if (!bringToFrontIfNeeded(systemModalWindowId)) {
      renderWindows();
    }
    return;
  }

  const id = createWindowId();
  const centered = getCenteredWindowPosition(SYSTEM_MODAL_WIDTH, SYSTEM_MODAL_HEIGHT);

  const windowModel: WindowModel = {
    id,
    appId: APP_IDS.SYSTEM_MODAL,
    kind: 'systemmodal',
    title: '',
    x: centered.x,
    y: centered.y,
    width: SYSTEM_MODAL_WIDTH,
    height: SYSTEM_MODAL_HEIGHT,
    zIndex: state.nextZIndex,
    isActive: true,
  };

  systemModalModeByWindowId.set(id, { mode: 'import' });
  systemModalWindowId = id;
  state.nextZIndex += 1;
  state.activeWindowId = id;
  createWindowModel(windowModel);
  renderWindows();
};

export const registerSystemModalKind = (): void => {
  registerWindowKind('systemmodal', {
    modal: true,
    icon: iconAppWindow,
    typeLabel: 'Task',
    renderContent: createSystemModalPanel,
    onClosing: (win) => {
      if (systemModalWindowId === win.id) {
        systemModalWindowId = null;
        systemModalModeByWindowId.delete(win.id);
      }
    },
  });
};

// ---- Drop import

const importDroppedFiles = async (files: FileList): Promise<{ ok: boolean; filename?: string; error?: string }> => {
  // Calculate total size of existing files
  let existingFilesSize = 0;
  for (const entry of fileSystem.values()) {
    existingFilesSize += entry.content.byteLength;
  }

  // Calculate total size of files to be imported
  let importFilesSize = 0;
  const fileContents: { file: File; content: Uint8Array }[] = [];

  for (const file of files) {
    const content = new Uint8Array(await file.arrayBuffer());
    importFilesSize += content.byteLength;
    fileContents.push({ file, content });
  }

  // Check size limit (1.5 MiB)
  const totalSize = existingFilesSize + importFilesSize;
  if (totalSize > FILE_IMPORT_SIZE_LIMIT_BYTES) {
    const existingMiB = (existingFilesSize / (1024 * 1024)).toFixed(2);
    const importMiB = (importFilesSize / (1024 * 1024)).toFixed(2);
    const limitMiB = (FILE_IMPORT_SIZE_LIMIT_BYTES / (1024 * 1024)).toFixed(1);
    const errorMessage = `合計サイズ (${existingMiB} MiB + ${importMiB} MiB) が ${limitMiB} MiB の制限を超えています`;
    return { ok: false, error: errorMessage };
  }

  let firstFileName: string | null = null;

  for (const { file, content } of fileContents) {
    const sourceName = normalizePathLikeName(file.webkitRelativePath || file.name);
    const result = upsertFile(sourceName, content);
    if (!result.ok) {
      continue;
    }

    if (!firstFileName) {
      firstFileName = result.name;
    }
  }

  // Return success if at least one file was imported
  if (firstFileName) {
    return { ok: true, filename: firstFileName };
  }

  return { ok: false, error: '取り込み可能なファイルがありません' };
};

// Track if modal has been created during this drag operation
let modalCreated = false;

export const initDropImport = (): void => {
  // Use window-level listeners with capture phase to intercept drag events early
  window.addEventListener('dragenter', (event) => {
    event.preventDefault();
    event.stopPropagation();

    // Create modal if not already created, or if previous modal was closed
    if (!modalCreated || !systemModalWindowId || !findWindowById(systemModalWindowId)) {
      createSystemModalWindow();
      modalCreated = true;
    }
  }, { capture: true });

  window.addEventListener('dragover', (event) => {
    event.preventDefault();
    event.stopPropagation();

    // Ensure modal exists on drag over (in case it was closed)
    if (!modalCreated || !systemModalWindowId || !findWindowById(systemModalWindowId)) {
      createSystemModalWindow();
      modalCreated = true;
    }

    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = 'copy';
    }
  }, { capture: true });

  window.addEventListener('dragleave', (event) => {
    event.preventDefault();
    event.stopPropagation();
    // Only close modal if drag completely leaves the window
    const clientX = (event as DragEvent).clientX;
    const clientY = (event as DragEvent).clientY;
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;

    if (clientX < 0 || clientX >= viewportWidth || clientY < 0 || clientY >= viewportHeight) {
      modalCreated = false;
      if (systemModalWindowId) {
        closeWindow(systemModalWindowId);
      }
    }
  }, { capture: true });

  window.addEventListener('drop', (event) => {
    event.preventDefault();
    event.stopPropagation();
    modalCreated = false;

    const files = event.dataTransfer?.files;

    if (!files || files.length === 0) {
      // Hide modal on failed drop
      if (systemModalWindowId && findWindowById(systemModalWindowId)) {
        closeWindow(systemModalWindowId);
      }
      return;
    }

    // Create a copy of files array to ensure it's accessible in async context
    const fileArray = Array.from(files);

    // Import files asynchronously
    void (async () => {
      const fileList = {
        length: fileArray.length,
        item(index: number) {
          return fileArray[index] || null;
        },
        [Symbol.iterator]: function* () {
          yield* fileArray;
        },
      } as unknown as FileList;

      const result = await importDroppedFiles(fileList);

      // Update modal to mode 2 (complete) with result
      if (systemModalWindowId && findWindowById(systemModalWindowId)) {
        if (result.ok) {
          // Success: show filename
          systemModalModeByWindowId.set(systemModalWindowId, { mode: 'complete', filename: result.filename });
        } else {
          // Error: show error message
          systemModalModeByWindowId.set(systemModalWindowId, { mode: 'complete', error: result.error });
        }
        renderWindows();
      }
    })();
  }, { capture: true });
};
