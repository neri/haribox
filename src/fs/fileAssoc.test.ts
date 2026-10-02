import { beforeEach, describe, expect, it, vi } from 'vitest';
import iconAppWindow from '../assets/icons/app-window.svg?raw';
import iconFileMusic from '../assets/icons/file-music.svg?raw';
import iconFileText from '../assets/icons/file-text.svg?raw';
import iconFile from '../assets/icons/file.svg?raw';
import iconPhoto from '../assets/icons/photo.svg?raw';
import { getFileIcon, resolveSelectedIndex, searchExecutableFile } from './fileAssoc';
import { loadFileSystem, upsertFile } from './fileSystem';

describe('getFileIcon', () => {
  it('picks the icon from the extension, ignoring case', () => {
    expect(getFileIcon('CALC.HRB', '.hrb')).toBe(iconAppWindow);
    expect(getFileIcon('readme.TXT', '.hrb')).toBe(iconFileText);
    expect(getFileIcon('a.bmp', '.hrb')).toBe(iconPhoto);
    expect(getFileIcon('a.jpg', '.hrb')).toBe(iconPhoto);
    expect(getFileIcon('a.mml', '.hrb')).toBe(iconFileMusic);
    expect(getFileIcon('a.bin', '.hrb')).toBe(iconFile);
    expect(getFileIcon('noext', '.hrb')).toBe(iconFile);
  });

  it('treats every extension in PATH_EXT (colon separated) as executable', () => {
    expect(getFileIcon('a.exe', '.hrb:.EXE')).toBe(iconAppWindow);
    // PATH_EXT wins over the built-in file types
    expect(getFileIcon('a.txt', '.hrb:.txt')).toBe(iconAppWindow);
  });
});

describe('searchExecutableFile', () => {
  beforeEach(() => {
    vi.stubGlobal('localStorage', {
      getItem: () => null,
      setItem: () => undefined,
      removeItem: () => undefined,
    });
    loadFileSystem();
  });

  it('finds a file by its exact name, ignoring case', () => {
    expect(searchExecutableFile('calc.hrb', '.hrb')).toBe('CALC.HRB');
  });

  it('tries each PATH_EXT extension in order', () => {
    expect(searchExecutableFile('calc', '.hrb')).toBe('CALC.HRB');
    expect(searchExecutableFile('calc', '.exe:.hrb')).toBe('CALC.HRB');
    expect(searchExecutableFile('calc', '.exe')).toBeNull();
  });

  it('prefers the exact name over an extension match', () => {
    upsertFile('calc', new Uint8Array([1]));
    expect(searchExecutableFile('calc', '.hrb')).toBe('calc');
  });

  it('returns null when nothing matches', () => {
    expect(searchExecutableFile('nothing', '.hrb')).toBeNull();
  });
});

describe('resolveSelectedIndex', () => {
  it('selects the first row when nothing was selected yet', () => {
    expect(resolveSelectedIndex(['A', 'B', 'C'], { index: 0, key: null })).toBe(0);
  });

  it('follows the selected file when a file is added before it', () => {
    expect(resolveSelectedIndex(['A', 'AA', 'B', 'C'], { index: 1, key: 'B' })).toBe(2);
  });

  it('follows the selected file when a file before it is removed', () => {
    expect(resolveSelectedIndex(['B', 'C'], { index: 1, key: 'B' })).toBe(0);
  });

  it('keeps the row position when the selected file is gone', () => {
    expect(resolveSelectedIndex(['A', 'C', 'D'], { index: 1, key: 'B' })).toBe(1);
  });

  it('moves to the last row when the selected last file is gone', () => {
    expect(resolveSelectedIndex(['A', 'B'], { index: 2, key: 'C' })).toBe(1);
  });

  it('returns 0 for an empty list', () => {
    expect(resolveSelectedIndex([], { index: 3, key: 'C' })).toBe(0);
  });
});
