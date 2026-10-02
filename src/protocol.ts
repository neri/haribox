// Messages exchanged between the main thread and the task worker.
// Both sides (task/taskRunner.ts and rustTask.worker.ts, wasm/env.ts) import these definitions.

export const WriteFileMode = {
  Update: 0,
  Create: 1,
  Upsert: 2,
} as const;
export type WriteFileMode = (typeof WriteFileMode)[keyof typeof WriteFileMode];

/** Worker -> Main */
export type WorkerCommand =
  // width and height are the size of the content area, without the title bar
  | { type: 'openWindow'; windowId: string; width: number; height: number; title: string }
  | { type: 'moveWindow'; windowId: string; x: number; y: number }
  | { type: 'activateWindow'; windowId: string }
  | { type: 'closeWindow'; windowId: string }
  | { type: 'drawImage'; windowId: string; x: number; y: number; width: number; height: number; pixels: ArrayBuffer }
  // Output to the terminal of the task. The main thread decides which terminal that is.
  | { type: 'print'; text: string }
  | { type: 'println'; text: string }
  // The worker has already checked the name and the write mode; the main thread stores the file as it is
  | { type: 'fileWritten'; filename: string; data: ArrayBuffer }
  | { type: 'playSound'; frequency: number; timestamp: number }
  | { type: 'error'; message: string; stack?: string }
  | { type: 'done' };

export type FileSystemSnapshot = Array<{ name: string; content: ArrayBuffer }>;

/** Files that changed after the snapshot of the start message was taken. A rename is a remove and a put. */
export type FileSystemChangeMessage = {
  type: 'fileSystemChanged';
  changes: Array<{ type: 'put'; name: string; content: ArrayBuffer } | { type: 'remove'; name: string }>;
};

export type WorkerStartMessage = {
  type: 'startWithCommand';
  fileName: string;
  commandLine: string;
  fileSystemSnapshot: FileSystemSnapshot;
};

/** A key pressed while a Canvas window of the task is active */
export type KeyMessage = {
  type: 'key';
  // Key code in bit 0-7 and modifier state in bit 8-15 (see input/taskKeyCode.ts)
  code: number;
};

export type WindowCloseMessage = {
  type: 'windowClose';
  windowId: string;
};

/** Main -> Worker */
export type MainToWorkerMessage =
  | WorkerStartMessage
  | KeyMessage
  | WindowCloseMessage
  | FileSystemChangeMessage;
