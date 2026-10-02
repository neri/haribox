// USB HID Keyboard modifier bits
export const MODIFIER_BITS = {
  LEFT_CTRL: 0x01,   // Bit 0
  LEFT_SHIFT: 0x02,  // Bit 1
  LEFT_ALT: 0x04,    // Bit 2
  LEFT_GUI: 0x08,    // Bit 3
  RIGHT_CTRL: 0x10,  // Bit 4
  RIGHT_SHIFT: 0x20, // Bit 5
  RIGHT_ALT: 0x40,   // Bit 6
  RIGHT_GUI: 0x80,   // Bit 7
} as const;

// KeyboardEvent.code distinguishes left/right
const MODIFIER_BIT_BY_CODE: Record<string, number> = {
  ControlLeft: MODIFIER_BITS.LEFT_CTRL,
  ControlRight: MODIFIER_BITS.RIGHT_CTRL,
  ShiftLeft: MODIFIER_BITS.LEFT_SHIFT,
  ShiftRight: MODIFIER_BITS.RIGHT_SHIFT,
  AltLeft: MODIFIER_BITS.LEFT_ALT,
  AltRight: MODIFIER_BITS.RIGHT_ALT,
  MetaLeft: MODIFIER_BITS.LEFT_GUI,
  MetaRight: MODIFIER_BITS.RIGHT_GUI,
};

/**
 * Get the modifier bitmap with left/right distinction
 * @param bitmap - Current modifier bitmap
 * @param code - KeyboardEvent.code
 * @param isPressed - true for keydown, false for keyup
 * @returns Updated modifier bitmap
 */
export const applyModifierKey = (bitmap: number, code: string, isPressed: boolean): number => {
  const bit = MODIFIER_BIT_BY_CODE[code];
  if (bit === undefined) {
    return bitmap;
  }

  return isPressed ? bitmap | bit : bitmap & ~bit;
};
