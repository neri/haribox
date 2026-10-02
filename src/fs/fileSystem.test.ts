import { inflate } from 'pako';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { FILE_STORAGE_KEY } from '../constants';
import {
  createFileSystemSnapshot,
  decodeFileAsText,
  fileSystem,
  fromBase64,
  listFilesSorted,
  loadFileSystem,
  onFileSystemChanged,
  removeFile,
  renameFile,
  setStorageWarningHandler,
  toBase64,
  upsertFile,
} from './fileSystem';
import type { FileEntry } from './fileSystem';

const storage = new Map<string, string>();

const readPersistedNames = (): string[] => {
  const raw = storage.get(FILE_STORAGE_KEY);
  if (!raw) {
    return [];
  }
  const json = new TextDecoder().decode(inflate(fromBase64(raw)));
  return (JSON.parse(json) as { files: Array<{ name: string }> }).files.map((file) => file.name);
};

const bytes = (text: string): Uint8Array => new TextEncoder().encode(text);

beforeEach(() => {
  storage.clear();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => storage.get(key) ?? null,
    setItem: (key: string, value: string) => void storage.set(key, value),
    removeItem: (key: string) => void storage.delete(key),
  });
  loadFileSystem();
});

describe('loadFileSystem', () => {
  it('starts with the bundled initial files', () => {
    const calc = fileSystem.get('CALC.HRB');
    expect(calc?.name).toBe('CALC.HRB');
    expect(calc?.isInitialFile).toBe(true);
    expect(calc?.content.byteLength).toBeGreaterThan(0);
    expect([...fileSystem.values()].every((entry) => entry.isInitialFile)).toBe(true);
  });

  it('discards unreadable saved data and keeps the initial files', () => {
    storage.set(FILE_STORAGE_KEY, 'not valid data!!');
    loadFileSystem();
    expect(storage.has(FILE_STORAGE_KEY)).toBe(false);
    expect(fileSystem.has('CALC.HRB')).toBe(true);
  });
});

describe('upsertFile', () => {
  it('stores the file under a case-insensitive key and persists it', () => {
    expect(upsertFile('dir/New.txt', bytes('hello'))).toEqual({ ok: true, name: 'New.txt' });
    expect(fileSystem.get('NEW.TXT')).toEqual({ name: 'New.txt', content: bytes('hello'), isInitialFile: false });

    loadFileSystem();
    expect(fileSystem.get('NEW.TXT')).toEqual({ name: 'New.txt', content: bytes('hello'), isInitialFile: false });
  });

  it('persists only files that are not initial files', () => {
    upsertFile('new.txt', bytes('hello'));
    expect(readPersistedNames()).toEqual(['new.txt']);
  });

  it('replaces an initial file of the same name and keeps the replacement after reload', () => {
    upsertFile('calc.hrb', bytes('replaced'));
    loadFileSystem();
    expect(fileSystem.get('CALC.HRB')).toEqual({ name: 'calc.hrb', content: bytes('replaced'), isInitialFile: false });
  });

  it('rejects an invalid name', () => {
    expect(upsertFile('', bytes('x'))).toEqual({ ok: false, reason: 'Filename is empty.' });
  });

  it('still succeeds in memory when saving fails', () => {
    vi.stubGlobal('localStorage', {
      getItem: () => null,
      setItem: () => {
        throw new Error('QuotaExceededError');
      },
      removeItem: () => undefined,
    });
    expect(upsertFile('new.txt', bytes('hello'))).toEqual({ ok: true, name: 'new.txt' });
    expect(fileSystem.has('NEW.TXT')).toBe(true);
  });
});

describe('removeFile', () => {
  it('removes a saved file for good', () => {
    upsertFile('new.txt', bytes('hello'));
    expect(removeFile('NEW.TXT')).toEqual({ ok: true, name: 'NEW.TXT' });
    loadFileSystem();
    expect(fileSystem.has('NEW.TXT')).toBe(false);
  });

  it('reports a missing file', () => {
    expect(removeFile('nope.txt')).toEqual({ ok: false, reason: 'File not found: nope.txt' });
  });

  // Specified behaviour: deletions of initial files are not saved (docs/filesystem.md 1.3)
  it('removes an initial file only until the next load', () => {
    expect(removeFile('calc.hrb')).toEqual({ ok: true, name: 'calc.hrb' });
    expect(fileSystem.has('CALC.HRB')).toBe(false);
    loadFileSystem();
    expect(fileSystem.get('CALC.HRB')?.isInitialFile).toBe(true);
  });
});

describe('renameFile', () => {
  it('moves the content to the new name', () => {
    upsertFile('new.txt', bytes('hello'));
    expect(renameFile('new.txt', 'renamed.txt')).toEqual({ ok: true, source: 'new.txt', destination: 'renamed.txt' });
    expect(fileSystem.has('NEW.TXT')).toBe(false);
    expect(fileSystem.get('RENAMED.TXT')).toEqual({ name: 'renamed.txt', content: bytes('hello'), isInitialFile: false });
    expect(readPersistedNames()).toEqual(['renamed.txt']);
  });

  it('overwrites an existing destination', () => {
    upsertFile('a.txt', bytes('A'));
    upsertFile('b.txt', bytes('B'));
    renameFile('a.txt', 'b.txt');
    expect(fileSystem.has('A.TXT')).toBe(false);
    expect(fileSystem.get('B.TXT')?.content).toEqual(bytes('A'));
  });

  it('reports a missing source', () => {
    expect(renameFile('nope.txt', 'x.txt')).toEqual({ ok: false, reason: 'File not found: nope.txt' });
  });

  it('keeps the renamed copy of an initial file and restores the original on the next load', () => {
    renameFile('CALC.HRB', 'C2.HRB');
    expect(fileSystem.has('CALC.HRB')).toBe(false);
    loadFileSystem();
    expect(fileSystem.get('CALC.HRB')?.isInitialFile).toBe(true);
    expect(fileSystem.get('C2.HRB')?.isInitialFile).toBe(false);
  });
});

describe('onFileSystemChanged', () => {
  it('notifies once per successful change and not on failures', () => {
    const listener = vi.fn();
    const unsubscribe = onFileSystemChanged(listener);

    upsertFile('a.txt', bytes('A'));
    renameFile('a.txt', 'b.txt');
    removeFile('b.txt');
    expect(listener).toHaveBeenCalledTimes(3);

    upsertFile('', bytes('x'));
    renameFile('nope.txt', 'x.txt');
    removeFile('nope.txt');
    expect(listener).toHaveBeenCalledTimes(3);

    unsubscribe();
    upsertFile('c.txt', bytes('C'));
    expect(listener).toHaveBeenCalledTimes(3);
  });

  it('describes the change, with a rename as remove and put', () => {
    const listener = vi.fn();
    const unsubscribe = onFileSystemChanged(listener);

    upsertFile('a.txt', bytes('A'));
    renameFile('a.txt', 'b.txt');
    removeFile('B.TXT');
    unsubscribe();

    expect(listener.mock.calls.map(([changes]) => changes)).toEqual([
      [{ type: 'put', name: 'a.txt', content: bytes('A') }],
      [
        { type: 'remove', name: 'a.txt' },
        { type: 'put', name: 'b.txt', content: bytes('A') },
      ],
      [{ type: 'remove', name: 'B.TXT' }],
    ]);
  });

  it('notifies after the change is visible', () => {
    let seen = false;
    const unsubscribe = onFileSystemChanged(() => {
      seen = fileSystem.has('A.TXT');
    });
    upsertFile('a.txt', bytes('A'));
    unsubscribe();
    expect(seen).toBe(true);
  });
});

describe('listFilesSorted', () => {
  it('sorts by name', () => {
    upsertFile('zzz.txt', bytes('z'));
    upsertFile('000.txt', bytes('0'));
    const names = listFilesSorted().map((entry) => entry.name);
    expect(names).toEqual([...names].sort((a, b) => a.localeCompare(b)));
    expect(names[0]).toBe('000.txt');
    expect(names.at(-1)).toBe('zzz.txt');
  });
});

describe('createFileSystemSnapshot', () => {
  it('copies the content so that the snapshot can be transferred', () => {
    upsertFile('a.txt', bytes('A'));
    const snapshot = createFileSystemSnapshot();
    const copy = snapshot.find((entry) => entry.name === 'a.txt');
    expect(new Uint8Array(copy!.content)).toEqual(bytes('A'));
    expect(copy!.content).not.toBe(fileSystem.get('A.TXT')!.content.buffer);
    expect(snapshot).toHaveLength(fileSystem.size);
  });
});

describe('storage warning', () => {
  it('is raised once when the saved data exceeds 1 MiB', () => {
    const handler = vi.fn();
    vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    setStorageWarningHandler(handler);

    // Random bytes so that compression cannot shrink the payload
    const large = new Uint8Array(1_200_000);
    for (let i = 0; i < large.length; i += 1) {
      large[i] = Math.floor(Math.random() * 256);
    }

    upsertFile('small.txt', bytes('x'));
    expect(handler).not.toHaveBeenCalled();

    upsertFile('large.bin', large);
    upsertFile('small2.txt', bytes('y'));
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler.mock.calls[0][0]).toContain('Please remove unnecessary files.');
  });
});

describe('base64', () => {
  it('encodes bytes', () => {
    expect(toBase64(new Uint8Array([72, 105]))).toBe('SGk=');
    expect(toBase64(new Uint8Array())).toBe('');
  });

  it('round-trips every byte value', () => {
    const bytes = Uint8Array.from({ length: 256 }, (_, i) => i);
    expect(fromBase64(toBase64(bytes))).toEqual(bytes);
  });
});

const entry = (bytes: number[] | Uint8Array): FileEntry => ({
  name: 'test.txt',
  content: Uint8Array.from(bytes),
  isInitialFile: false,
});

describe('decodeFileAsText', () => {
  it('decodes ASCII and empty content', () => {
    expect(decodeFileAsText(entry([0x68, 0x65, 0x6c, 0x6c, 0x6f]))).toBe('hello');
    expect(decodeFileAsText(entry([]))).toBe('');
  });

  it('decodes UTF-8', () => {
    expect(decodeFileAsText(entry(new TextEncoder().encode('こんにちは')))).toBe('こんにちは');
  });

  it('decodes Shift_JIS', () => {
    const sjis = [0x82, 0xb1, 0x82, 0xf1, 0x82, 0xc9, 0x82, 0xbf, 0x82, 0xcd];
    expect(decodeFileAsText(entry(sjis))).toBe('こんにちは');
  });
});
