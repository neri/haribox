// The copy of the file system that a task worker reads and writes.
// The main thread owns the real one (fileSystem.ts); the worker starts from a snapshot and is kept
// up to date with the changes that the main thread sends.

import { WriteFileMode } from '../protocol';
import type { FileSystemChangeMessage, FileSystemSnapshot } from '../protocol';
import { normalizeFileName, toCanonicalFileKey } from './fileName';

export type TaskFile = {
  name: string;
  content: Uint8Array;
};

export type TaskFileSystem = Map<string, TaskFile>;

export const createTaskFileSystem = (snapshot: FileSystemSnapshot): TaskFileSystem => {
  const files: TaskFileSystem = new Map();
  for (const entry of snapshot) {
    files.set(toCanonicalFileKey(entry.name), { name: entry.name, content: new Uint8Array(entry.content) });
  }
  return files;
};

export const applyFileSystemChanges = (files: TaskFileSystem, changes: FileSystemChangeMessage['changes']): void => {
  for (const change of changes) {
    const key = toCanonicalFileKey(change.name);
    if (change.type === 'put') {
      files.set(key, { name: change.name, content: new Uint8Array(change.content) });
    } else {
      files.delete(key);
    }
  }
};

/** Looks up a file the same way the main thread does: by the normalized name, ignoring case */
export const readTaskFile = (files: TaskFileSystem, rawName: string): Uint8Array | undefined => {
  const normalized = normalizeFileName(rawName);
  if (!normalized.ok) {
    return undefined;
  }
  return files.get(toCanonicalFileKey(normalized.name))?.content;
};

/**
 * Writes a file. The name is normalized with the rules of the main thread, so a write that succeeds here
 * is also accepted there.
 * @returns the name the file was stored under, or null if the name is invalid or the mode does not allow the write
 */
export const writeTaskFile = (
  files: TaskFileSystem,
  rawName: string,
  content: Uint8Array,
  mode: number,
): string | null => {
  const normalized = normalizeFileName(rawName);
  if (!normalized.ok) {
    return null;
  }

  const name = normalized.name;
  const key = toCanonicalFileKey(name);
  const exists = files.has(key);
  if (mode === WriteFileMode.Update && !exists) {
    return null;
  }
  if (mode === WriteFileMode.Create && exists) {
    return null;
  }

  files.set(key, { name, content });
  return name;
};
