import { describe, expect, it } from 'vitest';
import { WriteFileMode } from '../protocol';
import { applyFileSystemChanges, createTaskFileSystem, readTaskFile, writeTaskFile } from './taskFileSystem';

const bytes = (text: string): Uint8Array => new TextEncoder().encode(text);
const buffer = (text: string): ArrayBuffer => bytes(text).slice().buffer;

const createFiles = () => createTaskFileSystem([{ name: 'Hello.txt', content: buffer('hello') }]);

describe('createTaskFileSystem', () => {
  it('holds the files of the snapshot', () => {
    const files = createFiles();
    expect(files.size).toBe(1);
    expect(readTaskFile(files, 'Hello.txt')).toEqual(bytes('hello'));
  });
});

describe('readTaskFile', () => {
  it('ignores case', () => {
    expect(readTaskFile(createFiles(), 'HELLO.TXT')).toEqual(bytes('hello'));
  });

  it('normalizes the name like the main thread', () => {
    expect(readTaskFile(createFiles(), 'dir/Hello.txt')).toEqual(bytes('hello'));
  });

  it('returns undefined for a missing file or an invalid name', () => {
    expect(readTaskFile(createFiles(), 'nope.txt')).toBeUndefined();
    expect(readTaskFile(createFiles(), '')).toBeUndefined();
  });
});

describe('writeTaskFile', () => {
  it('updates only an existing file', () => {
    const files = createFiles();
    expect(writeTaskFile(files, 'hello.txt', bytes('new'), WriteFileMode.Update)).toBe('hello.txt');
    expect(readTaskFile(files, 'Hello.txt')).toEqual(bytes('new'));
    expect(writeTaskFile(files, 'other.txt', bytes('x'), WriteFileMode.Update)).toBeNull();
    expect(files.size).toBe(1);
  });

  it('creates only a new file', () => {
    const files = createFiles();
    expect(writeTaskFile(files, 'other.txt', bytes('x'), WriteFileMode.Create)).toBe('other.txt');
    expect(writeTaskFile(files, 'HELLO.TXT', bytes('x'), WriteFileMode.Create)).toBeNull();
    expect(readTaskFile(files, 'Hello.txt')).toEqual(bytes('hello'));
  });

  it('upserts in both cases', () => {
    const files = createFiles();
    expect(writeTaskFile(files, 'hello.txt', bytes('a'), WriteFileMode.Upsert)).toBe('hello.txt');
    expect(writeTaskFile(files, 'other.txt', bytes('b'), WriteFileMode.Upsert)).toBe('other.txt');
    expect(files.size).toBe(2);
  });

  it('stores the file under the normalized name and finds it by the original one', () => {
    const files = createFiles();
    expect(writeTaskFile(files, 'my file.txt', bytes('x'), WriteFileMode.Upsert)).toBe('my_file.txt');
    expect(readTaskFile(files, 'my file.txt')).toEqual(bytes('x'));
    expect(readTaskFile(files, 'MY_FILE.TXT')).toEqual(bytes('x'));
  });

  it('rejects a name that the main thread would reject', () => {
    const files = createFiles();
    expect(writeTaskFile(files, '', bytes('x'), WriteFileMode.Upsert)).toBeNull();
    expect(writeTaskFile(files, 'あいうえお.txt', bytes('x'), WriteFileMode.Upsert)).toBeNull();
    expect(files.size).toBe(1);
  });
});

describe('applyFileSystemChanges', () => {
  it('adds, replaces and removes files', () => {
    const files = createFiles();
    applyFileSystemChanges(files, [
      { type: 'put', name: 'new.txt', content: buffer('n') },
      { type: 'put', name: 'HELLO.txt', content: buffer('replaced') },
    ]);
    expect(readTaskFile(files, 'new.txt')).toEqual(bytes('n'));
    expect(readTaskFile(files, 'hello.txt')).toEqual(bytes('replaced'));
    expect(files.size).toBe(2);

    applyFileSystemChanges(files, [{ type: 'remove', name: 'NEW.TXT' }]);
    expect(readTaskFile(files, 'new.txt')).toBeUndefined();
  });

  it('applies a rename as remove and put, also when only the case changes', () => {
    const files = createFiles();
    applyFileSystemChanges(files, [
      { type: 'remove', name: 'Hello.txt' },
      { type: 'put', name: 'HELLO.TXT', content: buffer('hello') },
    ]);
    expect(files.size).toBe(1);
    expect(files.get('HELLO.TXT')!.name).toBe('HELLO.TXT');
  });
});
