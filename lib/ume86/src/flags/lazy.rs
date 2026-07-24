//! lazy flags

use crate::flags::FlagsRegister;
use crate::prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum LazyOp {
    And8(u8, u8),
    And16(u16, u16),
    And32(u32, u32),
    Or8(u8, u8),
    Or16(u16, u16),
    Or32(u32, u32),
    Xor8(u8, u8),
    Xor16(u16, u16),
    Xor32(u32, u32),
    Add8(u8, u8),
    Add16(u16, u16),
    Add32(u32, u32),
    Adc8(u8, u8, bool),
    Adc16(u16, u16, bool),
    Adc32(u32, u32, bool),
    Inc8(u8),
    Inc16(u16),
    Inc32(u32),
    Sub8(u8, u8),
    Sub16(u16, u16),
    Sub32(u32, u32),
    Sbb8(u8, u8, bool),
    Sbb16(u16, u16, bool),
    Sbb32(u32, u32, bool),
    Dec8(u8),
    Dec16(u16),
    Dec32(u32),
    Mul8(u8, u8),
    Mul16(u16, u16),
    Mul32(u32, u32),
    IMul8(i8, i8),
    IMul16(i16, i16),
    IMul32(i32, i32),
    Shl8(u8, u8),
    Shl16(u16, u8),
    Shl32(u32, u8),
    Shr8(u8, u8),
    Shr16(u16, u8),
    Shr32(u32, u8),
    Sar8(i8, u8),
    Sar16(i16, u8),
    Sar32(i32, u8),
}

impl Default for LazyOp {
    #[inline]
    fn default() -> Self {
        Self::And8(0, 0)
    }
}

/// Precomputed parity table for all 8-bit values (0-255).
static PARITY_TABLE: [bool; 256] = {
    let mut table = [false; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = (i.count_ones() % 2) == 0;
        i += 1;
    }
    table
};

impl LazyOp {
    /// Resolves all flags affected by this operation and updates the `Flags` struct accordingly.
    pub fn resolve_all_flags(&self, flags: &mut FlagsRegister) {
        self.resolve_cf(flags);
        self.resolve_pf(flags);
        self.resolve_af(flags);
        // self.resolve_zf(flags);
        self.resolve_sf(flags);
        self.resolve_of(flags);
    }

    /// Resolves the Carry Flag (CF) based on the operation and its operands.
    pub fn resolve_cf(&self, flags: &mut FlagsRegister) {
        if flags.valid_mask.contains(Flags::CF) {
            // CF is already valid, no need to resolve
            return;
        }
        match self {
            Self::Mul8(_, _)
            | Self::Mul16(_, _)
            | Self::Mul32(_, _)
            | Self::IMul8(_, _)
            | Self::IMul16(_, _)
            | Self::IMul32(_, _) => {
                if self.recompute_cf(flags) {
                    flags.set(Flags::OF | Flags::CF);
                } else {
                    flags.clear(Flags::OF | Flags::CF);
                }
            }
            _ => {
                if self.recompute_cf(flags) {
                    flags.set(Flags::CF);
                } else {
                    flags.clear(Flags::CF);
                }
            }
        }
    }

    /// Computes the value of the Carry Flag (CF)
    pub fn recompute_cf(&self, flags: &mut FlagsRegister) -> bool {
        if flags.valid_mask.contains(Flags::CF) {
            // CF is already valid, no need to resolve
            return flags.dynamic_value().contains(Flags::CF);
        }
        match self {
            Self::And8(_, _)
            | Self::Or8(_, _)
            | Self::Xor8(_, _)
            | Self::And16(_, _)
            | Self::Or16(_, _)
            | Self::Xor16(_, _)
            | Self::And32(_, _)
            | Self::Or32(_, _)
            | Self::Xor32(_, _) => false,
            Self::Add8(a, b) => a.checked_add(*b).is_none(),
            Self::Add16(a, b) => a.checked_add(*b).is_none(),
            Self::Add32(a, b) => a.checked_add(*b).is_none(),
            Self::Adc8(a, b, cf) => a
                .checked_add(*b)
                .and_then(|sum| sum.checked_add(*cf as u8))
                .is_none(),
            Self::Adc16(a, b, cf) => a
                .checked_add(*b)
                .and_then(|sum| sum.checked_add(*cf as u16))
                .is_none(),
            Self::Adc32(a, b, cf) => a
                .checked_add(*b)
                .and_then(|sum| sum.checked_add(*cf as u32))
                .is_none(),
            Self::Sub8(a, b) => *a < *b,
            Self::Sub16(a, b) => *a < *b,
            Self::Sub32(a, b) => *a < *b,
            Self::Sbb8(a, b, cf) => a
                .checked_sub(*b)
                .and_then(|diff| diff.checked_sub(*cf as u8))
                .is_none(),
            Self::Sbb16(a, b, cf) => a
                .checked_sub(*b)
                .and_then(|diff| diff.checked_sub(*cf as u16))
                .is_none(),
            Self::Sbb32(a, b, cf) => a
                .checked_sub(*b)
                .and_then(|diff| diff.checked_sub(*cf as u32))
                .is_none(),
            Self::Inc8(_)
            | Self::Inc16(_)
            | Self::Inc32(_)
            | Self::Dec8(_)
            | Self::Dec16(_)
            | Self::Dec32(_) => unreachable!(),
            Self::Mul8(a, b) => {
                let (_result, carry) = a.overflowing_mul(*b);
                carry
            }
            Self::Mul16(a, b) => {
                let (_result, carry) = a.overflowing_mul(*b);
                carry
            }
            Self::Mul32(a, b) => {
                let (_result, carry) = a.overflowing_mul(*b);
                carry
            }
            Self::IMul8(a, b) => {
                let (_result, carry) = a.overflowing_mul(*b);
                carry
            }
            Self::IMul16(a, b) => {
                let (_result, carry) = a.overflowing_mul(*b);
                carry
            }
            Self::IMul32(a, b) => {
                let (_result, carry) = a.overflowing_mul(*b);
                carry
            }
            Self::Shl8(a, b) => {
                let result = (*a as u16).wrapping_shl(*b as u32);
                result & 0x100 != 0
            }
            Self::Shl16(a, b) => {
                let result = (*a as u32).wrapping_shl(*b as u32);
                result & 0x1_0000 != 0
            }
            Self::Shl32(a, b) => {
                let result = (*a as u64).wrapping_shl(*b as u32);
                result & 0x1_0000_0000 != 0
            }
            Self::Shr8(a, b) => {
                let result = a.wrapping_shr(*b as u32 - 1);
                result & 0x1 != 0
            }
            Self::Shr16(a, b) => {
                let result = a.wrapping_shr(*b as u32 - 1);
                result & 0x1 != 0
            }
            Self::Shr32(a, b) => {
                let result = a.wrapping_shr(*b as u32 - 1);
                result & 0x1 != 0
            }
            Self::Sar8(a, b) => {
                let result = a.wrapping_shr(*b as u32 - 1);
                result & 0x1 != 0
            }
            Self::Sar16(a, b) => {
                let result = a.wrapping_shr(*b as u32 - 1);
                result & 0x1 != 0
            }
            Self::Sar32(a, b) => {
                let result = a.wrapping_shr(*b as u32 - 1);
                result & 0x1 != 0
            }
        }
    }

    /// Resolves the Auxiliary Carry Flag (AF) based on the operation and its operands.
    pub fn resolve_af(&self, flags: &mut FlagsRegister) {
        if flags.valid_mask.contains(Flags::AF) {
            // AF is already valid, no need to resolve
            return;
        }
        let result = match self {
            Self::And8(_, _)
            | Self::Or8(_, _)
            | Self::Xor8(_, _)
            | Self::And16(_, _)
            | Self::Or16(_, _)
            | Self::Xor16(_, _)
            | Self::And32(_, _)
            | Self::Or32(_, _)
            | Self::Xor32(_, _)
            | Self::Mul8(_, _)
            | Self::Mul16(_, _)
            | Self::Mul32(_, _)
            | Self::IMul8(_, _)
            | Self::IMul16(_, _)
            | Self::IMul32(_, _)
            | Self::Shl8(_, _)
            | Self::Shl16(_, _)
            | Self::Shl32(_, _)
            | Self::Shr8(_, _)
            | Self::Shr16(_, _)
            | Self::Shr32(_, _)
            | Self::Sar8(_, _)
            | Self::Sar16(_, _)
            | Self::Sar32(_, _) => {
                // AF is undefined
                return;
            }
            Self::Add8(a, b) => (a & 0x0f) + (b & 0x0f) > 0x0f,
            Self::Add16(a, b) => (a & 0x0f) + (b & 0x0f) > 0x0f,
            Self::Add32(a, b) => (a & 0x0f) + (b & 0x0f) > 0x0f,
            Self::Adc8(a, b, cf) => (a & 0x0f) + (b & 0x0f) + (*cf as u8) > 0x0f,
            Self::Adc16(a, b, cf) => (a & 0x0f) + (b & 0x0f) + (*cf as u16) > 0x0f,
            Self::Adc32(a, b, cf) => (a & 0x0f) + (b & 0x0f) + (*cf as u32) > 0x0f,
            Self::Sub8(a, b) => (a & 0x0f) < (b & 0x0f),
            Self::Sub16(a, b) => (a & 0x0f) < (b & 0x0f),
            Self::Sub32(a, b) => (a & 0x0f) < (b & 0x0f),
            Self::Sbb8(a, b, cf) => (a & 0x0f) < (b & 0x0f) + (*cf as u8),
            Self::Sbb16(a, b, cf) => (a & 0x0f) < (b & 0x0f) + (*cf as u16),
            Self::Sbb32(a, b, cf) => (a & 0x0f) < (b & 0x0f) + (*cf as u32),
            Self::Inc8(a) => (*a & 0x0f) == 0x0f,
            Self::Inc16(a) => (*a & 0x0f) == 0x0f,
            Self::Inc32(a) => (*a & 0x0f) == 0x0f,
            Self::Dec8(a) => (*a & 0x0f) == 0x00,
            Self::Dec16(a) => (*a & 0x0f) == 0x00,
            Self::Dec32(a) => (*a & 0x0f) == 0x00,
        };
        flags.set_dynamic(Flags::AF, result);
    }

    /// Resolves the Parity Flag (PF) based on the operation and its operands.
    pub fn resolve_pf(&self, flags: &mut FlagsRegister) {
        if flags.valid_mask.contains(Flags::PF) {
            // PF is already valid, no need to resolve
            return;
        }
        let result = match self {
            Self::And8(a, b) => a & b,
            Self::Or8(a, b) => a | b,
            Self::Xor8(a, b) => a ^ b,
            Self::And16(a, b) => (a & b) as u8,
            Self::Or16(a, b) => (a | b) as u8,
            Self::Xor16(a, b) => (a ^ b) as u8,
            Self::And32(a, b) => (a & b) as u8,
            Self::Or32(a, b) => (a | b) as u8,
            Self::Xor32(a, b) => (a ^ b) as u8,
            Self::Add8(a, b) => a.wrapping_add(*b),
            Self::Add16(a, b) => a.wrapping_add(*b) as u8,
            Self::Add32(a, b) => a.wrapping_add(*b) as u8,
            Self::Adc8(a, b, cf) => a.wrapping_add(*b).wrapping_add(*cf as u8),
            Self::Adc16(a, b, cf) => a.wrapping_add(*b).wrapping_add(*cf as u16) as u8,
            Self::Adc32(a, b, cf) => a.wrapping_add(*b).wrapping_add(*cf as u32) as u8,
            Self::Sub8(a, b) => a.wrapping_sub(*b),
            Self::Sub16(a, b) => a.wrapping_sub(*b) as u8,
            Self::Sub32(a, b) => a.wrapping_sub(*b) as u8,
            Self::Sbb8(a, b, cf) => a.wrapping_sub(*b).wrapping_sub(*cf as u8),
            Self::Sbb16(a, b, cf) => a.wrapping_sub(*b).wrapping_sub(*cf as u16) as u8,
            Self::Sbb32(a, b, cf) => a.wrapping_sub(*b).wrapping_sub(*cf as u32) as u8,
            Self::Inc8(a) => a.wrapping_add(1),
            Self::Inc16(a) => a.wrapping_add(1) as u8,
            Self::Inc32(a) => a.wrapping_add(1) as u8,
            Self::Dec8(a) => a.wrapping_sub(1),
            Self::Dec16(a) => a.wrapping_sub(1) as u8,
            Self::Dec32(a) => a.wrapping_sub(1) as u8,
            Self::Shl8(a, b) => a.wrapping_shl(*b as u32) as u8,
            Self::Shl16(a, b) => a.wrapping_shl(*b as u32) as u8,
            Self::Shl32(a, b) => a.wrapping_shl(*b as u32) as u8,
            Self::Shr8(a, b) => a.wrapping_shr(*b as u32) as u8,
            Self::Shr16(a, b) => a.wrapping_shr(*b as u32) as u8,
            Self::Shr32(a, b) => a.wrapping_shr(*b as u32) as u8,
            Self::Sar8(a, b) => a.wrapping_shr(*b as u32) as u8,
            Self::Sar16(a, b) => a.wrapping_shr(*b as u32) as u8,
            Self::Sar32(a, b) => a.wrapping_shr(*b as u32) as u8,
            // The PF flag is undefined for MUL operations, so we can skip them here.
            Self::Mul8(_, _)
            | Self::Mul16(_, _)
            | Self::Mul32(_, _)
            | Self::IMul8(_, _)
            | Self::IMul16(_, _)
            | Self::IMul32(_, _) => return,
        };
        flags.set_dynamic(Flags::PF, PARITY_TABLE[result as usize]);
    }

    /// Resolves the Zero Flag (ZF) based on the operation and its operands.
    #[allow(dead_code)]
    fn resolve_zf(&self, flags: &mut FlagsRegister) {
        if flags.valid_mask.contains(Flags::ZF) {
            // ZF is already valid, no need to resolve
            return;
        }
        let result = match self {
            Self::And8(a, b) => (a & b) == 0,
            Self::Or8(a, b) => (a | b) == 0,
            Self::Xor8(a, b) => (a ^ b) == 0,
            Self::And16(a, b) => (a & b) == 0,
            Self::Or16(a, b) => (a | b) == 0,
            Self::Xor16(a, b) => (a ^ b) == 0,
            Self::And32(a, b) => (a & b) == 0,
            Self::Or32(a, b) => (a | b) == 0,
            Self::Xor32(a, b) => (a ^ b) == 0,
            Self::Add8(a, b) => a.wrapping_add(*b) == 0,
            Self::Add16(a, b) => a.wrapping_add(*b) == 0,
            Self::Add32(a, b) => a.wrapping_add(*b) == 0,
            Self::Adc8(a, b, cf) => a.wrapping_add(*b).wrapping_add(*cf as u8) == 0,
            Self::Adc16(a, b, cf) => a.wrapping_add(*b).wrapping_add(*cf as u16) == 0,
            Self::Adc32(a, b, cf) => a.wrapping_add(*b).wrapping_add(*cf as u32) == 0,
            Self::Sub8(a, b) => a.wrapping_sub(*b) == 0,
            Self::Sub16(a, b) => a.wrapping_sub(*b) == 0,
            Self::Sub32(a, b) => a.wrapping_sub(*b) == 0,
            Self::Sbb8(a, b, cf) => a.wrapping_sub(*b).wrapping_sub(*cf as u8) == 0,
            Self::Sbb16(a, b, cf) => a.wrapping_sub(*b).wrapping_sub(*cf as u16) == 0,
            Self::Sbb32(a, b, cf) => a.wrapping_sub(*b).wrapping_sub(*cf as u32) == 0,
            Self::Inc8(a) => a.wrapping_add(1) == 0,
            Self::Inc16(a) => a.wrapping_add(1) == 0,
            Self::Inc32(a) => a.wrapping_add(1) == 0,
            Self::Dec8(a) => a.wrapping_sub(1) == 0,
            Self::Dec16(a) => a.wrapping_sub(1) == 0,
            Self::Dec32(a) => a.wrapping_sub(1) == 0,
            Self::Shl8(a, b) => a.wrapping_shl(*b as u32) == 0,
            Self::Shl16(a, b) => a.wrapping_shl(*b as u32) == 0,
            Self::Shl32(a, b) => a.wrapping_shl(*b as u32) == 0,
            Self::Shr8(a, b) => a.wrapping_shr(*b as u32) == 0,
            Self::Shr16(a, b) => a.wrapping_shr(*b as u32) == 0,
            Self::Shr32(a, b) => a.wrapping_shr(*b as u32) == 0,
            Self::Sar8(a, b) => a.wrapping_shr(*b as u32) == 0,
            Self::Sar16(a, b) => a.wrapping_shr(*b as u32) == 0,
            Self::Sar32(a, b) => a.wrapping_shr(*b as u32) == 0,
            // The ZF flag is undefined for MUL operations, so we can skip them here.
            Self::Mul8(_, _)
            | Self::Mul16(_, _)
            | Self::Mul32(_, _)
            | Self::IMul8(_, _)
            | Self::IMul16(_, _)
            | Self::IMul32(_, _) => return,
        };
        flags.set_dynamic(Flags::ZF, result);
    }

    /// Resolves the Sign Flag (SF) based on the operation and its operands.
    pub fn resolve_sf(&self, flags: &mut FlagsRegister) {
        if flags.valid_mask.contains(Flags::SF) {
            // SF is already valid, no need to resolve
            return;
        }
        let result = match self {
            Self::And8(a, b) => ((a & b) as i8) < 0,
            Self::Or8(a, b) => ((a | b) as i8) < 0,
            Self::Xor8(a, b) => ((a ^ b) as i8) < 0,
            Self::And16(a, b) => ((a & b) as i16) < 0,
            Self::Or16(a, b) => ((a | b) as i16) < 0,
            Self::Xor16(a, b) => ((a ^ b) as i16) < 0,
            Self::And32(a, b) => ((a & b) as i32) < 0,
            Self::Or32(a, b) => ((a | b) as i32) < 0,
            Self::Xor32(a, b) => ((a ^ b) as i32) < 0,
            Self::Add8(a, b) => (a.wrapping_add(*b) as i8) < 0,
            Self::Add16(a, b) => (a.wrapping_add(*b) as i16) < 0,
            Self::Add32(a, b) => (a.wrapping_add(*b) as i32) < 0,
            Self::Adc8(a, b, cf) => (a.wrapping_add(*b).wrapping_add(*cf as u8) as i8) < 0,
            Self::Adc16(a, b, cf) => (a.wrapping_add(*b).wrapping_add(*cf as u16) as i16) < 0,
            Self::Adc32(a, b, cf) => (a.wrapping_add(*b).wrapping_add(*cf as u32) as i32) < 0,
            Self::Sub8(a, b) => (a.wrapping_sub(*b) as i8) < 0,
            Self::Sub16(a, b) => (a.wrapping_sub(*b) as i16) < 0,
            Self::Sub32(a, b) => (a.wrapping_sub(*b) as i32) < 0,
            Self::Sbb8(a, b, cf) => (a.wrapping_sub(*b).wrapping_sub(*cf as u8) as i8) < 0,
            Self::Sbb16(a, b, cf) => (a.wrapping_sub(*b).wrapping_sub(*cf as u16) as i16) < 0,
            Self::Sbb32(a, b, cf) => (a.wrapping_sub(*b).wrapping_sub(*cf as u32) as i32) < 0,
            Self::Inc8(a) => (a.wrapping_add(1) as i8) < 0,
            Self::Inc16(a) => (a.wrapping_add(1) as i16) < 0,
            Self::Inc32(a) => (a.wrapping_add(1) as i32) < 0,
            Self::Dec8(a) => (a.wrapping_sub(1) as i8) < 0,
            Self::Dec16(a) => (a.wrapping_sub(1) as i16) < 0,
            Self::Dec32(a) => (a.wrapping_sub(1) as i32) < 0,
            Self::Shl8(a, b) => (a.wrapping_shl(*b as u32) as i8) < 0,
            Self::Shl16(a, b) => (a.wrapping_shl(*b as u32) as i16) < 0,
            Self::Shl32(a, b) => (a.wrapping_shl(*b as u32) as i32) < 0,
            Self::Shr8(a, b) => (a.wrapping_shr(*b as u32) as i8) < 0,
            Self::Shr16(a, b) => (a.wrapping_shr(*b as u32) as i16) < 0,
            Self::Shr32(a, b) => (a.wrapping_shr(*b as u32) as i32) < 0,
            Self::Sar8(a, b) => a.wrapping_shr(*b as u32) < 0,
            Self::Sar16(a, b) => a.wrapping_shr(*b as u32) < 0,
            Self::Sar32(a, b) => a.wrapping_shr(*b as u32) < 0,
            // The SF flag is undefined for MUL operations, so we can skip them here.
            Self::Mul8(_, _)
            | Self::Mul16(_, _)
            | Self::Mul32(_, _)
            | Self::IMul8(_, _)
            | Self::IMul16(_, _)
            | Self::IMul32(_, _) => return,
        };
        flags.set_dynamic(Flags::SF, result);
    }

    /// Resolves the Overflow Flag (OF) based on the operation and its operands.
    pub fn resolve_of(&self, flags: &mut FlagsRegister) {
        if flags.valid_mask.contains(Flags::OF) {
            // OF is already valid, no need to resolve
            return;
        }
        let of = match self {
            Self::And8(_, _)
            | Self::Or8(_, _)
            | Self::Xor8(_, _)
            | Self::And16(_, _)
            | Self::Or16(_, _)
            | Self::Xor16(_, _)
            | Self::And32(_, _)
            | Self::Or32(_, _)
            | Self::Xor32(_, _) => false,
            Self::Add8(a, b) => (((*a) as i8).checked_add(*b as i8)).is_none(),
            Self::Add16(a, b) => (((*a) as i16).checked_add(*b as i16)).is_none(),
            Self::Add32(a, b) => (((*a) as i32).checked_add(*b as i32)).is_none(),
            Self::Adc8(a, b, cf) => (((*a) as i8)
                .checked_add(*b as i8)
                .and_then(|v| v.checked_add(*cf as i8)))
            .is_none(),
            Self::Adc16(a, b, cf) => (((*a) as i16)
                .checked_add(*b as i16)
                .and_then(|v| v.checked_add(*cf as i16)))
            .is_none(),
            Self::Adc32(a, b, cf) => (((*a) as i32)
                .checked_add(*b as i32)
                .and_then(|v| v.checked_add(*cf as i32)))
            .is_none(),
            Self::Sub8(a, b) => (((*a) as i8).checked_sub(*b as i8)).is_none(),
            Self::Sub16(a, b) => (((*a) as i16).checked_sub(*b as i16)).is_none(),
            Self::Sub32(a, b) => (((*a) as i32).checked_sub(*b as i32)).is_none(),
            Self::Sbb8(a, b, cf) => (((*a) as i8)
                .checked_sub(*b as i8)
                .and_then(|v| v.checked_sub(*cf as i8)))
            .is_none(),
            Self::Sbb16(a, b, cf) => (((*a) as i16)
                .checked_sub(*b as i16)
                .and_then(|v| v.checked_sub(*cf as i16)))
            .is_none(),
            Self::Sbb32(a, b, cf) => (((*a) as i32)
                .checked_sub(*b as i32)
                .and_then(|v| v.checked_sub(*cf as i32)))
            .is_none(),
            Self::Inc8(a) => (((*a) as i8).checked_add(1)).is_none(),
            Self::Inc16(a) => (((*a) as i16).checked_add(1)).is_none(),
            Self::Inc32(a) => (((*a) as i32).checked_add(1)).is_none(),
            Self::Dec8(a) => (((*a) as i8).checked_sub(1)).is_none(),
            Self::Dec16(a) => (((*a) as i16).checked_sub(1)).is_none(),
            Self::Dec32(a) => (((*a) as i32).checked_sub(1)).is_none(),
            Self::Shl8(a, b) => {
                let result = a.wrapping_shl(*b as u32) as i8;
                self.resolve_cf(flags);
                (result < 0) != flags.cf()
            }
            Self::Shl16(a, b) => {
                let result = a.wrapping_shl(*b as u32) as i16;
                self.resolve_cf(flags);
                (result < 0) != flags.cf()
            }
            Self::Shl32(a, b) => {
                let result = a.wrapping_shl(*b as u32) as i32;
                self.resolve_cf(flags);
                (result < 0) != flags.cf()
            }
            Self::Shr8(a, b) => {
                let result = a.wrapping_shr(*b as u32) as i8;
                self.resolve_cf(flags);
                (result < 0) != flags.cf()
            }
            Self::Shr16(a, b) => {
                let result = a.wrapping_shr(*b as u32) as i16;
                self.resolve_cf(flags);
                (result < 0) != flags.cf()
            }
            Self::Shr32(a, b) => {
                let result = a.wrapping_shr(*b as u32) as i32;
                self.resolve_cf(flags);
                (result < 0) != flags.cf()
            }
            Self::Sar8(a, b) => {
                let result = a.wrapping_shr(*b as u32);
                self.resolve_cf(flags);
                (result < 0) != flags.cf()
            }
            Self::Sar16(a, b) => {
                let result = a.wrapping_shr(*b as u32);
                self.resolve_cf(flags);
                (result < 0) != flags.cf()
            }
            Self::Sar32(a, b) => {
                let result = a.wrapping_shr(*b as u32);
                self.resolve_cf(flags);
                (result < 0) != flags.cf()
            }
            Self::Mul8(_, _)
            | Self::Mul16(_, _)
            | Self::Mul32(_, _)
            | Self::IMul8(_, _)
            | Self::IMul16(_, _)
            | Self::IMul32(_, _) => {
                self.resolve_cf(flags);
                return;
            }
        };
        flags.set_dynamic(Flags::OF, of);
    }
}
