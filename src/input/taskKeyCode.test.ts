import { describe, expect, it } from 'vitest';
import { MODIFIER_BITS } from './modifiers';
import { toTaskKeyCode } from './taskKeyCode';

describe('toTaskKeyCode', () => {
  it('uses the character code for printable ASCII keys', () => {
    expect(toTaskKeyCode('a', 'KeyA', 0)).toBe(0x61);
    expect(toTaskKeyCode('A', 'KeyA', 0)).toBe(0x41);
    expect(toTaskKeyCode(' ', 'Space', 0)).toBe(0x20);
    expect(toTaskKeyCode('~', 'Backquote', 0)).toBe(0x7e);
  });

  it('maps the keys without a character to their codes', () => {
    const codes = ['Backspace', 'Enter', 'Escape', 'PageUp', 'PageDown', 'End', 'Home'];
    expect(codes.map((code) => toTaskKeyCode(code, code, 0))).toEqual([0x08, 0x0a, 0x1b, 0x80, 0x81, 0x82, 0x83]);
    const others = ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Insert', 'Delete'];
    expect(others.map((code) => toTaskKeyCode(code, code, 0))).toEqual([0x84, 0x85, 0x86, 0x87, 0x88, 0x89]);
  });

  it('does not deliver other keys', () => {
    expect(toTaskKeyCode('Shift', 'ShiftLeft', MODIFIER_BITS.LEFT_SHIFT)).toBeNull();
    expect(toTaskKeyCode('F1', 'F1', 0)).toBeNull();
    expect(toTaskKeyCode('Tab', 'Tab', 0)).toBeNull();
  });

  it('does not deliver characters outside printable ASCII', () => {
    expect(toTaskKeyCode('あ', 'KeyA', 0)).toBeNull();
    expect(toTaskKeyCode('é', 'KeyE', 0)).toBeNull();
  });

  it('puts the modifier state in bit 8-15 with Shift before Ctrl', () => {
    const bits = [
      MODIFIER_BITS.LEFT_SHIFT,
      MODIFIER_BITS.LEFT_CTRL,
      MODIFIER_BITS.LEFT_ALT,
      MODIFIER_BITS.LEFT_GUI,
      MODIFIER_BITS.RIGHT_SHIFT,
      MODIFIER_BITS.RIGHT_CTRL,
      MODIFIER_BITS.RIGHT_ALT,
      MODIFIER_BITS.RIGHT_GUI,
    ];
    expect(bits.map((bit) => toTaskKeyCode('a', 'KeyA', bit))).toEqual([
      0x0161, 0x0261, 0x0461, 0x0861, 0x1061, 0x2061, 0x4061, 0x8061,
    ]);
  });

  it('combines modifiers', () => {
    const bitmap = MODIFIER_BITS.LEFT_CTRL | MODIFIER_BITS.RIGHT_SHIFT;
    expect(toTaskKeyCode('ArrowUp', 'ArrowUp', bitmap)).toBe(0x1286);
  });
});
