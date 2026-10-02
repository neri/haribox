import { describe, expect, it } from 'vitest';
import { applyModifierKey, MODIFIER_BITS } from './modifiers';

describe('applyModifierKey', () => {
  it('sets a separate bit for the left and right key', () => {
    let bitmap = 0;
    bitmap = applyModifierKey(bitmap, 'ShiftLeft', true);
    expect(bitmap).toBe(MODIFIER_BITS.LEFT_SHIFT);
    bitmap = applyModifierKey(bitmap, 'ShiftRight', true);
    expect(bitmap).toBe(MODIFIER_BITS.LEFT_SHIFT | MODIFIER_BITS.RIGHT_SHIFT);
  });

  it('clears only the released key', () => {
    let bitmap = MODIFIER_BITS.LEFT_CTRL | MODIFIER_BITS.RIGHT_ALT | MODIFIER_BITS.LEFT_GUI;
    bitmap = applyModifierKey(bitmap, 'AltRight', false);
    expect(bitmap).toBe(MODIFIER_BITS.LEFT_CTRL | MODIFIER_BITS.LEFT_GUI);
    bitmap = applyModifierKey(bitmap, 'MetaLeft', false);
    expect(bitmap).toBe(MODIFIER_BITS.LEFT_CTRL);
  });

  it('maps every modifier key to its USB HID bit', () => {
    const codes = ['ControlLeft', 'ShiftLeft', 'AltLeft', 'MetaLeft', 'ControlRight', 'ShiftRight', 'AltRight', 'MetaRight'];
    expect(codes.map((code) => applyModifierKey(0, code, true))).toEqual([0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80]);
  });

  it('ignores other keys', () => {
    expect(applyModifierKey(0x03, 'KeyA', true)).toBe(0x03);
    expect(applyModifierKey(0x03, 'KeyA', false)).toBe(0x03);
  });
});
