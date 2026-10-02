import { describe, expect, it } from 'vitest';
import { appendTextToLines, pushHistory, splitCommandTokens } from './buffer';

describe('appendTextToLines', () => {
  it('continues the last line and starts a new line at each newline', () => {
    const lines: string[] = [];
    appendTextToLines(lines, 'a', 100);
    expect(lines).toEqual(['a']);
    appendTextToLines(lines, 'b\nc', 100);
    expect(lines).toEqual(['ab', 'c']);
    appendTextToLines(lines, 'x\n', 100);
    expect(lines).toEqual(['ab', 'cx', '']);
  });

  it('drops the oldest lines beyond the limit', () => {
    const lines = ['1', '2'];
    appendTextToLines(lines, '\n3\n4\n5', 3);
    expect(lines).toEqual(['3', '4', '5']);
  });
});

describe('pushHistory', () => {
  it('puts the newest command first', () => {
    const history = ['a'];
    pushHistory(history, 'b');
    expect(history).toEqual(['b', 'a']);
  });

  it('moves a repeated command to the front instead of duplicating it', () => {
    const history = ['c', 'b', 'a'];
    pushHistory(history, 'a');
    expect(history).toEqual(['a', 'c', 'b']);
  });
});

describe('splitCommandTokens', () => {
  it('splits on any whitespace', () => {
    expect(splitCommandTokens('  copy   a.txt\tb.txt ')).toEqual(['copy', 'a.txt', 'b.txt']);
    expect(splitCommandTokens('   ')).toEqual([]);
  });
});
