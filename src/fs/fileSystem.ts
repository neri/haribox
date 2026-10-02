import Encoding from 'encoding-japanese';
import { deflate, inflate } from 'pako';
import { INITIAL_FS_ENTRIES } from '../generated-initial-fs';
import { FILE_STORAGE_KEY, FILE_STORAGE_WARNING_BYTES, TEXT_DECODER, TEXT_ENCODER } from '../constants';
import { normalizeFileName, toCanonicalFileKey } from './fileName';

export type FileEntry = {
  name: string;
  content: Uint8Array;
  isInitialFile: boolean;
};

type StoredFileSystemV1 = {
  version: 1;
  files: Array<{
    name: string;
    contentBase64: string;
  }>;
};

// ---- Base64 (storage encoding)

export const toBase64 = (bytes: Uint8Array): string => {
  let binary = '';
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
};

export const fromBase64 = (base64: string): Uint8Array => {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
};

// ---- File system

export const fileSystem = new Map<string, FileEntry>();
let hasShownStorageWarning = false;
let storageWarningHandler: ((message: string) => void) | null = null;

const changeListeners = new Set<() => void>();

/**
 * Subscribes to file creation, update, deletion and rename.
 * @returns a function that removes the listener
 */
export const onFileSystemChanged = (listener: () => void): (() => void) => {
  changeListeners.add(listener);
  return () => {
    changeListeners.delete(listener);
  };
};

/** Files in the order every listing shows them */
export const listFilesSorted = (): FileEntry[] => {
  return [...fileSystem.values()].sort((a, b) => a.name.localeCompare(b.name));
};

/** A copy of all files that can be posted to a worker */
export const createFileSystemSnapshot = (): Array<{ name: string; content: ArrayBuffer }> => {
  return [...fileSystem.values()].map((entry) => ({
    name: entry.name,
    content: entry.content.slice().buffer,
  }));
};

// The warning is shown by the UI layer; this module does not touch the DOM
export const setStorageWarningHandler = (handler: (message: string) => void): void => {
  storageWarningHandler = handler;
};

const getStoredFileSystemV1 = (): StoredFileSystemV1 => {
  return {
    version: 1,
    files: [...fileSystem.values()]
      .filter((entry) => !entry.isInitialFile)
      .map((entry) => ({
        name: entry.name,
        contentBase64: toBase64(entry.content),
      })),
  };
};

const applyInitialFiles = (): void => {
  for (const entry of INITIAL_FS_ENTRIES) {
    const compressed = fromBase64(entry.contentBase64);
    const decompressed = inflate(compressed);
    fileSystem.set(toCanonicalFileKey(entry.name), {
      name: entry.name,
      content: decompressed,
      isInitialFile: true,
    });
  }
};

const mergeStoredFiles = (files: Array<{ name: string; contentBase64: string }>): void => {
  for (const entry of files) {
    if (!entry || typeof entry.name !== 'string' || typeof entry.contentBase64 !== 'string') {
      continue;
    }

    fileSystem.set(toCanonicalFileKey(entry.name), {
      name: entry.name,
      content: fromBase64(entry.contentBase64),
      isInitialFile: false,
    });
  }
};

const persistFileSystem = (): void => {
  const storedV1 = getStoredFileSystemV1();
  const compressedPayload = deflate(TEXT_ENCODER.encode(JSON.stringify(storedV1)));
  const serialized = toBase64(compressedPayload);
  const serializedSize = TEXT_ENCODER.encode(serialized).length;

  try {
    localStorage.setItem(FILE_STORAGE_KEY, serialized);
  } catch {
    return;
  }

  if (serializedSize > FILE_STORAGE_WARNING_BYTES) {
    const warning = `Warning: filesystem uses ${serializedSize} bytes (> ${FILE_STORAGE_WARNING_BYTES} bytes).`;
    console.warn(warning);
    if (!hasShownStorageWarning) {
      hasShownStorageWarning = true;
      storageWarningHandler?.(`${warning}\nPlease remove unnecessary files.`);
    }
  }
};

// Saves the change and tells subscribers (file manager, running tasks) about it
const commitChange = (): void => {
  persistFileSystem();
  for (const listener of changeListeners) {
    listener();
  }
};

export const loadFileSystem = (): void => {
  fileSystem.clear();

  // 1. Initialize with initial files
  applyInitialFiles();

  // 2. Merge with localStorage data
  const raw = localStorage.getItem(FILE_STORAGE_KEY);
  if (!raw) {
    return;
  }

  try {
    const compressed = fromBase64(raw);
    const decompressedBytes = inflate(compressed);
    const decompressedJson = TEXT_DECODER.decode(decompressedBytes);
    const parsed = JSON.parse(decompressedJson) as StoredFileSystemV1;
    if (parsed.version === 1 && Array.isArray(parsed.files)) {
      mergeStoredFiles(parsed.files);
    }
  } catch {
    localStorage.removeItem(FILE_STORAGE_KEY);
  }
};

export const upsertFile = (rawName: string, content: Uint8Array): { ok: true; name: string } | { ok: false; reason: string } => {
  const normalized = normalizeFileName(rawName);
  if (!normalized.ok) {
    return { ok: false, reason: normalized.reason };
  }

  const name = normalized.name;
  const key = toCanonicalFileKey(name);

  fileSystem.set(key, { name, content, isInitialFile: false });
  commitChange();
  return { ok: true, name };
};

export const removeFile = (rawName: string): { ok: true; name: string } | { ok: false; reason: string } => {
  const normalized = normalizeFileName(rawName);
  if (!normalized.ok) {
    return { ok: false, reason: normalized.reason };
  }

  const name = normalized.name;
  const key = toCanonicalFileKey(name);
  if (!fileSystem.has(key)) {
    return { ok: false, reason: `File not found: ${rawName}` };
  }
  fileSystem.delete(key);
  commitChange();
  return { ok: true, name };
};

export const renameFile = (
  rawSource: string,
  rawDestination: string,
): { ok: true; source: string; destination: string } | { ok: false; reason: string } => {
  const sourceNormalized = normalizeFileName(rawSource);
  if (!sourceNormalized.ok) {
    return { ok: false, reason: sourceNormalized.reason };
  }

  const destinationNormalized = normalizeFileName(rawDestination);
  if (!destinationNormalized.ok) {
    return { ok: false, reason: destinationNormalized.reason };
  }

  const source = sourceNormalized.name;
  const destination = destinationNormalized.name;
  const sourceKey = toCanonicalFileKey(source);
  const destinationKey = toCanonicalFileKey(destination);
  const sourceEntry = fileSystem.get(sourceKey);

  if (!sourceEntry) {
    return { ok: false, reason: `File not found: ${rawSource}` };
  }

  fileSystem.delete(sourceKey);
  fileSystem.set(destinationKey, {
    name: destination,
    content: sourceEntry.content,
    isInitialFile: false,
  });
  commitChange();

  return { ok: true, source, destination };
};

// ---- Reading file content as text

export const decodeFileAsText = (entry: FileEntry): string => {
  try {
    // First, try to detect the encoding
    const detectedEnc = Encoding.detect(entry.content);

    // If Shift_JIS is detected, try that first
    if (detectedEnc === 'SJIS') {
      try {
        const converted = Encoding.convert(entry.content, { from: 'SJIS', to: 'UNICODE' });
        if (Array.isArray(converted)) {
          return String.fromCharCode(...converted);
        }
      } catch { }
    }

    // Try UTF-8
    try {
      return TEXT_DECODER.decode(entry.content);
    } catch { }

    // Fallback: Try Shift_JIS even if not detected
    try {
      const converted = Encoding.convert(entry.content, { from: 'SJIS', to: 'UNICODE' });
      if (Array.isArray(converted)) {
        return String.fromCharCode(...converted);
      }
    } catch { }

    // Last resort: Use UTF-8 with error replacement
    return TEXT_DECODER.decode(entry.content);
  } catch {
    return '[Error decoding file]';
  }
};
