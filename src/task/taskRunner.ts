import { applyDrawImage, openRustWindow } from '../apps/canvas';
import { appendTerminalLine, appendTerminalText, terminalOutputByWindow, terminalState } from '../apps/terminal/state';
import { playSound, stopOscillator } from '../audio/audio';
import { TITLE_BAR_HEIGHT } from '../constants';
import { searchExecutableFile } from '../fs/fileAssoc';
import { normalizeFileName, toCanonicalFileKey } from '../fs/fileName';
import { createFileSystemSnapshot, fileSystem, onFileSystemChanged, upsertFile } from '../fs/fileSystem';
import { WriteFileMode } from '../protocol';
import type { UpdateFileSystemSnapshotMessage, WindowCloseMessage, WorkerCommand, WorkerStartMessage } from '../protocol';
import type { WindowId } from '../wm/state';
import {
  addWindowClosingListener,
  bringToFrontIfNeeded,
  closeWindow,
  createWindowId,
  setWindowPositionById,
} from '../wm/windowManager';

// Environment variables passed to launched tasks. The terminal's SET command edits them.
export const env: Record<string, string> = {
  PATH_EXT: '.hrb',
};

/** Colon separated list of extensions that are treated as executable */
export const getPathExt = (): string => {
  return env['PATH_EXT'] ?? '.hrb';
};

const activeWorkers = new Set<Worker>();
const workerByWindowId = new Map<WindowId, Worker>();
const workerIdsByWorker = new Map<Worker, Set<string>>(); // Track workerId (for oscillators) by Worker
const tasksSkipDefaultTerminalFallback = new Set<WindowId>();

export const getWorkerByWindowId = (windowId: WindowId): Worker | undefined => {
  return workerByWindowId.get(windowId);
};

const broadcastFileSystemSnapshot = (): void => {
  if (activeWorkers.size === 0) {
    return;
  }

  const message: UpdateFileSystemSnapshotMessage = {
    type: 'updateFileSystemSnapshot',
    fileSystemSnapshot: createFileSystemSnapshot(),
  };

  for (const worker of activeWorkers) {
    worker.postMessage(message);
  }
};

/**
 * Terminal that shows a task's output: the one it was started from, otherwise the default terminal.
 * No-display tasks (START/NCST/OPEN) never fall back to the default terminal.
 */
const resolveOutputTerminal = (windowId: WindowId): WindowId | null => {
  if (terminalOutputByWindow.has(windowId)) {
    return windowId;
  }

  const defaultId = terminalState.defaultTerminalWindowId;
  if (defaultId && terminalOutputByWindow.has(defaultId) && !tasksSkipDefaultTerminalFallback.has(windowId)) {
    return defaultId;
  }

  return null;
};

// Also output to default terminal if available for visibility in Chromium
const reportToDefaultTerminal = (text: string): void => {
  const defaultId = terminalState.defaultTerminalWindowId;
  if (defaultId && terminalOutputByWindow.has(defaultId)) {
    appendTerminalLine(defaultId, text);
  }
};

const handleWorkerCommand = (command: WorkerCommand, worker: Worker): void => {
  switch (command.type) {
    case 'openWindow':
      openRustWindow(command.windowId, command.width, command.height, command.title);
      // Register Canvas window to Worker mapping when window is created
      workerByWindowId.set(command.windowId, worker);
      return;
    case 'moveWindow':
      setWindowPositionById(command.windowId, command.x, command.y);
      return;
    case 'activateWindow':
      bringToFrontIfNeeded(command.windowId);
      return;
    case 'closeWindow':
      closeWindow(command.windowId);
      return;
    case 'drawImage':
      applyDrawImage(command);
      return;
    case 'print': {
      const terminalId = resolveOutputTerminal(command.windowId);
      if (terminalId) {
        appendTerminalText(terminalId, command.text);
      }
      return;
    }
    case 'println': {
      const terminalId = resolveOutputTerminal(command.windowId);
      if (terminalId) {
        appendTerminalLine(terminalId, command.text);
      }
      return;
    }
    case 'fileWritten': {
      const normalizedResult = normalizeFileName(command.filename);
      if (!normalizedResult.ok) {
        return;
      }
      const exists = fileSystem.has(toCanonicalFileKey(normalizedResult.name));
      if (command.mode === WriteFileMode.Update && !exists) {
        return;
      }
      if (command.mode === WriteFileMode.Create && exists) {
        return;
      }
      // Persists, then the change notification sends the new snapshot to all active workers
      upsertFile(command.filename, new Uint8Array(command.data));
      return;
    }
    case 'playSound': {
      // Use the workerId from the worker to track oscillators
      playSound(command.workerId, command.frequency, command.timestamp);
      // Record this workerId for cleanup when the worker terminates
      if (!workerIdsByWorker.has(worker)) {
        workerIdsByWorker.set(worker, new Set());
      }
      workerIdsByWorker.get(worker)!.add(command.workerId);
      return;
    }
    case 'error':
      console.error(`Rust worker error: ${command.message}`);
      return;
    case 'done':
      return;
  }
};

/** Releases everything held for a worker and terminates it. Used by every way a task can end. */
const releaseWorker = (worker: Worker, terminalWindowId: WindowId): void => {
  activeWorkers.delete(worker);

  // Clean up all Oscillators created by this worker
  const workerIds = workerIdsByWorker.get(worker);
  if (workerIds) {
    for (const workerId of workerIds) {
      stopOscillator(workerId);
    }
    workerIdsByWorker.delete(worker);
    console.log(`[Audio] Cleaned up ${workerIds.size} oscillator(s) for terminated worker`);
  }

  // Clean up all Canvas window mappings for this worker
  for (const [windowId, w] of workerByWindowId.entries()) {
    if (w === worker) {
      workerByWindowId.delete(windowId);
    }
  }

  tasksSkipDefaultTerminalFallback.delete(terminalWindowId);
  worker.terminate();
};

const startRustWorker = (startMessage: WorkerStartMessage): void => {
  // Vite resolves this URL at build time, so it must stay a literal
  const worker = new Worker(new URL('../rustTask.worker.ts', import.meta.url), { type: 'module' });
  activeWorkers.add(worker);

  worker.addEventListener('message', (event: MessageEvent<WorkerCommand>) => {
    handleWorkerCommand(event.data, worker);
    if (event.data.type === 'done' || event.data.type === 'error') {
      releaseWorker(worker, startMessage.terminalWindowId);
    }
  });
  worker.addEventListener('error', (event) => {
    const errorMsg = `Worker error: ${event.error?.message ?? event.message ?? 'Unknown error'}`;
    console.error(errorMsg, event.error);
    reportToDefaultTerminal(`[worker error] ${errorMsg}`);
    releaseWorker(worker, startMessage.terminalWindowId);
  });

  try {
    worker.postMessage(startMessage);
  } catch (error) {
    const errorMsg = `Failed to start worker: ${error instanceof Error ? error.message : String(error)}`;
    console.error(errorMsg);
    reportToDefaultTerminal(`[worker] ${errorMsg}`);
    releaseWorker(worker, startMessage.terminalWindowId);
  }
};

export const launchRustTaskWithCommand = (terminalWindowId: WindowId, fileName: string, commandLine: string): void => {
  startRustWorker({
    type: 'startWithCommand',
    titleBarHeight: TITLE_BAR_HEIGHT,
    terminalWindowId,
    fileName,
    commandLine,
    fileSystemSnapshot: createFileSystemSnapshot(),
    environmentVariables: env,
  });
};

/**
 * Launch Rust task with dummy terminal (no output display).
 * Errors are only logged to console.error
 */
export const launchNoDisplayTask = (normalizedInput: string, tokens: string[]): void => {
  if (tokens.length < 2) {
    console.error('Usage: <command> <filename> [args...]');
    return;
  }

  // Extract command line after command name (without command name itself)
  const afterCommandIndex = normalizedInput.indexOf(tokens[1]);
  const commandLine = normalizedInput.slice(afterCommandIndex);

  const fileName = searchExecutableFile(tokens[1], getPathExt());
  if (!fileName) {
    console.error('Bad command or file name');
    return;
  }

  const dummyWindowId = createWindowId();
  tasksSkipDefaultTerminalFallback.add(dummyWindowId);
  launchRustTaskWithCommand(dummyWindowId, fileName, commandLine);
};

export const initTaskRunner = (): void => {
  // Notify Worker if closing a Canvas window
  addWindowClosingListener((closingWindow) => {
    if (closingWindow.kind !== 'canvas') {
      return;
    }

    const worker = workerByWindowId.get(closingWindow.id);
    if (worker) {
      const closeMessage: WindowCloseMessage = {
        type: 'windowClose',
        windowId: closingWindow.id,
      };
      worker.postMessage(closeMessage);
      workerByWindowId.delete(closingWindow.id);
    }
  });

  // Running tasks read files from their own snapshot, so every change is sent to them
  onFileSystemChanged(broadcastFileSystemSnapshot);
};
