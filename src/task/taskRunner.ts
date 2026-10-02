import { applyDrawImage, openRustWindow } from '../apps/canvas';
import { appendTerminalLine, appendTerminalText, terminalOutputByWindow, terminalState } from '../apps/terminal/state';
import { playSound, stopOscillator } from '../audio/audio';
import { searchExecutableFile } from '../fs/fileAssoc';
import { createFileSystemSnapshot, onFileSystemChanged, upsertFile } from '../fs/fileSystem';
import type { FileSystemChange } from '../fs/fileSystem';
import type { FileSystemChangeMessage, WindowCloseMessage, WorkerCommand, WorkerStartMessage } from '../protocol';
import type { WindowId } from '../wm/state';
import { addWindowClosingListener, bringToFrontIfNeeded, closeWindow, setWindowPositionById } from '../wm/windowManager';

// Environment variables of the shell. The terminal's SET command edits them. They are not passed to tasks.
export const env: Record<string, string> = {
  PATH_EXT: '.hrb',
};

/** Colon separated list of extensions that are treated as executable */
export const getPathExt = (): string => {
  return env['PATH_EXT'] ?? '.hrb';
};

/** A running task. The worker never learns these; the main thread identifies the task by its worker. */
type Task = {
  worker: Worker;
  /** Terminal the task was started from, or null for tasks without output display (START/NCST/OPEN) */
  terminalWindowId: WindowId | null;
  /** Key of the task's oscillator in audio.ts */
  soundId: string;
  /** Pending check that ends the task after its last window was closed */
  windowlessTimer: number | null;
};

// A task whose last Canvas window was closed is ended, unless it opens another window within this time
const WINDOWLESS_GRACE_MS = 100;

const activeTasks = new Set<Task>();
const taskByWindowId = new Map<WindowId, Task>(); // Canvas windows opened by each task

export const getWorkerByWindowId = (windowId: WindowId): Worker | undefined => {
  return taskByWindowId.get(windowId)?.worker;
};

// Running tasks read files from their own copy, so every change is sent to them.
// The task that made the change gets it too: that keeps its copy in the same order as the real file system.
const broadcastFileSystemChanges = (changes: readonly FileSystemChange[]): void => {
  if (activeTasks.size === 0) {
    return;
  }

  const message: FileSystemChangeMessage = {
    type: 'fileSystemChanged',
    changes: changes.map((change) =>
      change.type === 'put'
        ? { type: 'put', name: change.name, content: change.content.slice().buffer }
        : { type: 'remove', name: change.name },
    ),
  };

  for (const task of activeTasks) {
    task.worker.postMessage(message);
  }
};

/**
 * Terminal that shows a task's output: the one it was started from, otherwise the default terminal.
 * No-display tasks (START/NCST/OPEN) never fall back to the default terminal.
 */
const resolveOutputTerminal = (task: Task): WindowId | null => {
  if (task.terminalWindowId === null) {
    return null;
  }

  if (terminalOutputByWindow.has(task.terminalWindowId)) {
    return task.terminalWindowId;
  }

  const defaultId = terminalState.defaultTerminalWindowId;
  if (defaultId && terminalOutputByWindow.has(defaultId)) {
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

const handleWorkerCommand = (command: WorkerCommand, task: Task): void => {
  switch (command.type) {
    case 'openWindow':
      openRustWindow(command.windowId, command.width, command.height, command.title);
      // Register Canvas window to task mapping when window is created
      taskByWindowId.set(command.windowId, task);
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
      const terminalId = resolveOutputTerminal(task);
      if (terminalId) {
        appendTerminalText(terminalId, command.text);
      }
      return;
    }
    case 'println': {
      const terminalId = resolveOutputTerminal(task);
      if (terminalId) {
        appendTerminalLine(terminalId, command.text);
      }
      return;
    }
    case 'fileWritten':
      // Persists, then the change notification sends the change to all active workers
      upsertFile(command.filename, new Uint8Array(command.data));
      return;
    case 'playSound':
      playSound(task.soundId, command.frequency, command.timestamp);
      return;
    case 'error':
      console.error(`Rust worker error: ${command.message}`);
      return;
    case 'done':
      return;
  }
};

/** Releases everything held for a task and terminates its worker. Used by every way a task can end. */
const releaseTask = (task: Task): void => {
  activeTasks.delete(task);

  if (task.windowlessTimer !== null) {
    clearTimeout(task.windowlessTimer);
    task.windowlessTimer = null;
  }

  // Stop the sound the task may have left playing
  stopOscillator(task.soundId);

  // Clean up all Canvas window mappings for this task
  for (const [windowId, t] of taskByWindowId.entries()) {
    if (t === task) {
      taskByWindowId.delete(windowId);
    }
  }

  task.worker.terminate();
};

const hasWindow = (task: Task): boolean => {
  for (const owner of taskByWindowId.values()) {
    if (owner === task) {
      return true;
    }
  }
  return false;
};

/**
 * Ends the task if it still has no window after the grace period.
 * This is decided here and not in the worker, so that it also works for a task that never yields.
 */
const scheduleWindowlessCheck = (task: Task): void => {
  if (task.windowlessTimer !== null) {
    clearTimeout(task.windowlessTimer);
  }

  task.windowlessTimer = window.setTimeout(() => {
    task.windowlessTimer = null;
    if (activeTasks.has(task) && !hasWindow(task)) {
      releaseTask(task);
    }
  }, WINDOWLESS_GRACE_MS);
};

const startRustWorker = (terminalWindowId: WindowId | null, startMessage: WorkerStartMessage): void => {
  // Vite resolves this URL at build time, so it must stay a literal
  const worker = new Worker(new URL('../rustTask.worker.ts', import.meta.url), { type: 'module' });
  const task: Task = { worker, terminalWindowId, soundId: crypto.randomUUID(), windowlessTimer: null };
  activeTasks.add(task);

  worker.addEventListener('message', (event: MessageEvent<WorkerCommand>) => {
    handleWorkerCommand(event.data, task);
    if (event.data.type === 'done' || event.data.type === 'error') {
      releaseTask(task);
    }
  });
  worker.addEventListener('error', (event) => {
    const errorMsg = `Worker error: ${event.error?.message ?? event.message ?? 'Unknown error'}`;
    console.error(errorMsg, event.error);
    reportToDefaultTerminal(`[worker error] ${errorMsg}`);
    releaseTask(task);
  });

  try {
    worker.postMessage(startMessage);
  } catch (error) {
    const errorMsg = `Failed to start worker: ${error instanceof Error ? error.message : String(error)}`;
    console.error(errorMsg);
    reportToDefaultTerminal(`[worker] ${errorMsg}`);
    releaseTask(task);
  }
};

/**
 * Launch Rust task. Its output goes to the given terminal, or to the default terminal if that one is gone.
 * Pass null for no output display.
 */
export const launchRustTaskWithCommand = (
  terminalWindowId: WindowId | null,
  fileName: string,
  commandLine: string,
): void => {
  startRustWorker(terminalWindowId, {
    type: 'startWithCommand',
    fileName,
    commandLine,
    fileSystemSnapshot: createFileSystemSnapshot(),
  });
};

/**
 * Launch Rust task with no output display.
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

  launchRustTaskWithCommand(null, fileName, commandLine);
};

export const initTaskRunner = (): void => {
  // A Canvas window is closing, either by the user or by its task
  addWindowClosingListener((closingWindow) => {
    if (closingWindow.kind !== 'canvas') {
      return;
    }

    const task = taskByWindowId.get(closingWindow.id);
    if (!task) {
      return;
    }

    const closeMessage: WindowCloseMessage = {
      type: 'windowClose',
      windowId: closingWindow.id,
    };
    task.worker.postMessage(closeMessage);
    taskByWindowId.delete(closingWindow.id);

    if (!hasWindow(task)) {
      scheduleWindowlessCheck(task);
    }
  });

  onFileSystemChanged(broadcastFileSystemChanges);
};
