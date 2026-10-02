import { MODIFIER_BITS } from './modifiers';

// Codes of the keys that have no printable character, by KeyboardEvent.code.
// 0x80 and above are custom codes of this emulator.
const SPECIAL_KEY_CODES: Record<string, number> = {
  Backspace: 0x08,
  Enter: 0x0a,
  Escape: 0x1b,
  PageUp: 0x80,
  PageDown: 0x81,
  End: 0x82,
  Home: 0x83,
  ArrowLeft: 0x84,
  ArrowRight: 0x85,
  ArrowUp: 0x86,
  ArrowDown: 0x87,
  Insert: 0x88,
  Delete: 0x89,
};

// USB HID modifier bit -> bit in the upper byte of the event code (legacy haribote order: Shift comes before Ctrl)
const TASK_MODIFIER_BITS: ReadonlyArray<readonly [number, number]> = [
  [MODIFIER_BITS.LEFT_SHIFT, 0x0100],
  [MODIFIER_BITS.LEFT_CTRL, 0x0200],
  [MODIFIER_BITS.LEFT_ALT, 0x0400],
  [MODIFIER_BITS.LEFT_GUI, 0x0800],
  [MODIFIER_BITS.RIGHT_SHIFT, 0x1000],
  [MODIFIER_BITS.RIGHT_CTRL, 0x2000],
  [MODIFIER_BITS.RIGHT_ALT, 0x4000],
  [MODIFIER_BITS.RIGHT_GUI, 0x8000],
];

/**
 * Convert a pressed key to the event code that a task receives.
 * @param key - KeyboardEvent.key
 * @param code - KeyboardEvent.code
 * @param modifierBitmap - Modifier state in USB HID format (see modifiers.ts)
 * @returns Key code in bit 0-7 and modifier state in bit 8-15, or null if the key is not delivered to tasks
 */
export const toTaskKeyCode = (key: string, code: string, modifierBitmap: number): number | null => {
  let keyCode: number | undefined;
  if (key.length === 1) {
    // Only printable ASCII characters (0x20-0x7E) are delivered
    const charCode = key.charCodeAt(0);
    if (charCode >= 0x20 && charCode <= 0x7e) {
      keyCode = charCode;
    }
  } else {
    keyCode = SPECIAL_KEY_CODES[code];
  }

  if (keyCode === undefined) {
    return null;
  }

  let modifiers = 0;
  for (const [hidBit, taskBit] of TASK_MODIFIER_BITS) {
    if (modifierBitmap & hidBit) {
      modifiers |= taskBit;
    }
  }

  return keyCode | modifiers;
};
