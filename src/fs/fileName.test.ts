import { describe, expect, it } from 'vitest';
import { MAX_FILE_NAME_LENGTH, normalizeFileName, normalizePathLikeName, toCanonicalFileKey } from './fileName';

const ok = (name: string) => ({ ok: true, name });

describe('normalizePathLikeName', () => {
  it('keeps only the last path segment', () => {
    expect(normalizePathLikeName('dir/sub\\file.txt')).toBe('file.txt');
    expect(normalizePathLikeName('file.txt')).toBe('file.txt');
    expect(normalizePathLikeName('dir/')).toBe('dir');
    expect(normalizePathLikeName('/')).toBe('');
  });
});

describe('toCanonicalFileKey', () => {
  it('upper-cases the name', () => {
    expect(toCanonicalFileKey('Hello.Txt')).toBe('HELLO.TXT');
  });
});

describe('normalizeFileName', () => {
  it('accepts allowed characters as they are', () => {
    expect(normalizeFileName('Hello_World!.hrb')).toEqual(ok('Hello_World!.hrb'));
  });

  it('trims surrounding whitespace and strips directories', () => {
    expect(normalizeFileName('  a/b\\hello.txt ')).toEqual(ok('hello.txt'));
  });

  it('rejects empty input', () => {
    expect(normalizeFileName('')).toEqual({ ok: false, reason: 'Filename is empty.' });
    expect(normalizeFileName('   ')).toEqual({ ok: false, reason: 'Filename is empty.' });
    expect(normalizeFileName('/')).toEqual({ ok: false, reason: 'Filename is empty.' });
  });

  it('replaces disallowed characters with an underscore and collapses runs', () => {
    expect(normalizeFileName('a b#c.txt')).toEqual(ok('a_b_c.txt'));
    expect(normalizeFileName('a  b__c.txt')).toEqual(ok('a_b_c.txt'));
  });

  it('replaces non-ASCII characters when they are at most half of the name', () => {
    expect(normalizeFileName('ファイルabc.txt')).toEqual(ok('_abc.txt'));
    // exactly half is still accepted
    expect(normalizeFileName('ファイル.txt')).toEqual(ok('_.txt'));
  });

  it('rejects names that are mostly non-ASCII', () => {
    expect(normalizeFileName('ファイルa.t')).toEqual({
      ok: false,
      reason: 'Filename conversion failed: too many non-ASCII characters in ファイルa.t',
    });
  });

  it('removes leading and trailing dots', () => {
    expect(normalizeFileName('..hidden..')).toEqual(ok('hidden'));
    expect(normalizeFileName('.bashrc')).toEqual(ok('bashrc'));
    expect(normalizeFileName('...')).toEqual({ ok: false, reason: 'Filename conversion failed: ...' });
  });

  it('keeps names up to the maximum length', () => {
    const name = `${'a'.repeat(MAX_FILE_NAME_LENGTH - 4)}.txt`;
    expect(name).toHaveLength(31);
    expect(normalizeFileName(name)).toEqual(ok(name));
  });

  it('truncates a long name without extension', () => {
    expect(normalizeFileName('a'.repeat(40))).toEqual(ok('a'.repeat(31)));
  });

  it('truncates the base and keeps the extension when the base stays long enough', () => {
    expect(normalizeFileName(`${'a'.repeat(40)}.txt`)).toEqual(ok(`${'a'.repeat(27)}.txt`));
  });

  it('truncates the extension too when the base would become shorter than 8 characters', () => {
    const result = normalizeFileName(`abcdefghij.${'x'.repeat(30)}`);
    expect(result).toEqual(ok(`abcdefgh.${'x'.repeat(22)}`));
    expect(result.ok && result.name).toHaveLength(31);
  });
});
