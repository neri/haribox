// Messages exchanged between the main thread and rustTask.worker.ts.
// The worker has its own definitions; these describe what it actually sends and accepts.

export const WriteFileMode = {
  Update: 0,
  Create: 1,
  Upsert: 2,
} as const;
export type WriteFileMode = (typeof WriteFileMode)[keyof typeof WriteFileMode];

export type WorkerCommand =
  | { type: 'openWindow'; windowId: string; width: number; height: number; title: string }
  | { type: 'moveWindow'; windowId: string; x: number; y: number }
  | { type: 'activateWindow'; windowId: string }
  | { type: 'closeWindow'; windowId: string }
  | { type: 'drawImage'; windowId: string; x: number; y: number; width: number; height: number; pixels: ArrayBuffer }
  | { type: 'print'; windowId: string; text: string }
  | { type: 'println'; windowId: string; text: string }
  | { type: 'fileWritten'; filename: string; data: ArrayBuffer; mode: WriteFileMode }
  | { type: 'playSound'; workerId: string; frequency: number; timestamp: number }
  | { type: 'error'; message: string; stack?: string }
  | { type: 'done' };

export type UpdateFileSystemSnapshotMessage = {
  type: 'updateFileSystemSnapshot';
  fileSystemSnapshot: Array<{ name: string; content: ArrayBuffer }>;
};

export type WorkerStartMessage = {
  type: 'startWithCommand';
  titleBarHeight: number;
  terminalWindowId: string;
  fileName: string;
  commandLine: string;
  fileSystemSnapshot: Array<{ name: string; content: ArrayBuffer }>;
  environmentVariables: Record<string, string>;
};

export type KeyboardEventMessage = {
  type: 'keyboardEvent';
  windowId: string;
  eventType: 'keydown' | 'keyup';
  key: string;
  code: string;
  keyCode: number;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  isAutoRepeat: boolean;
  modifierBitmap: number;
};

export type WindowCloseMessage = {
  type: 'windowClose';
  windowId: string;
};
