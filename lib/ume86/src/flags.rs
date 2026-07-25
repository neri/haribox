//! The FLAGS register

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

use crate::cpu::Generation;

// use crate::prelude::*;

mod lazy;
pub use lazy::LazyOp;

/// x86 FLAGS register representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flags(u32);

impl Flags {
    /// Carry flag
    pub const CF: Self = Self(0x0000_0001);
    /// Reserved Always 1
    // pub const _VF: Self = Self(0x0000_0002);
    /// Parity flag
    pub const PF: Self = Self(0x0000_0004);
    /// Adjust flag
    pub const AF: Self = Self(0x0000_0010);
    /// Zero flag
    pub const ZF: Self = Self(0x0000_0040);
    /// Sign flag
    pub const SF: Self = Self(0x0000_0080);
    /// Trap flag
    pub const TF: Self = Self(0x0000_0100);
    /// Interrupt enable flag
    pub const IF: Self = Self(0x0000_0200);
    /// Direction flag
    pub const DF: Self = Self(0x0000_0400);
    /// Overflow flag
    pub const OF: Self = Self(0x0000_0800);
    /// I/O privilege level
    pub const IOPL3: Self = Self(0x0000_3000);
    /// Nested task flag
    pub const NT: Self = Self(0x0000_4000);
    /// Mode flag (NEC V30)
    pub const MD: Self = Self(0x0000_8000);
    /// Resume flag
    pub const RF: Self = Self(0x0001_0000);
    /// Virtual 8086 mode flag
    pub const VM: Self = Self(0x0002_0000);
    /// Alignment check
    pub const AC: Self = Self(0x0004_0000);
    /// Virtual interrupt flag
    pub const VIF: Self = Self(0x0008_0000);
    /// Virtual interrupt pending
    pub const VIP: Self = Self(0x0010_0000);
    /// Able to use CPUID instruction
    pub const ID: Self = Self(0x0020_0000);

    /// All flags cleared (used for initialization)
    pub const ZERO: Self = Self(0);

    /// All flags set (used for testing)
    pub const NOT_ZERO: Self = Self(!0);

    #[inline]
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// Returns the raw bits of the flags.
    #[inline]
    pub const fn bits(&self) -> u32 {
        self.0
    }

    #[inline]
    pub fn contains(&self, value: Self) -> bool {
        (self.0 & value.0) == value.0
    }
}

impl BitAnd<Self> for Flags {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign<Self> for Flags {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOr<Self> for Flags {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign<Self> for Flags {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitXor<Self> for Flags {
    type Output = Self;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl BitXorAssign<Self> for Flags {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl Not for Flags {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

#[derive(Clone, Copy)]
pub struct FlagsRegister {
    dynamic_value: Flags,
    static_value: Flags,
    pub valid_mask: Flags,
    always_0_mask: Flags,
    always_1_mask: Flags,
}

impl FlagsRegister {
    pub const DYNAMIC_VALUE_MASK: Flags = Flags::from_bits(0x0000_08d5);

    #[inline]
    pub(super) const fn dynamic_value(&self) -> Flags {
        self.dynamic_value
    }

    /// Creates a new `FlagsRegister` struct with the appropriate masks based on the CPU generation.
    pub fn new(generation: Generation) -> Self {
        let mask0 = Self::mask_0_for(generation);
        let mask1 = Self::mask_1_for(generation);
        Self {
            dynamic_value: Flags::ZERO,
            static_value: Flags::ZERO,
            valid_mask: Flags::NOT_ZERO,
            always_0_mask: Flags::from_bits(mask0),
            always_1_mask: Flags::from_bits(mask1),
        }
    }

    #[inline]
    pub const fn mask_0_for(generation: Generation) -> u32 {
        let base = 0x003f_7fd5;
        match generation {
            Generation::I8086 | Generation::I186 | Generation::I286 => base & 0x0000_0fff,
            Generation::I386 => base & 0x0003_ffff,
            Generation::I486 => base & 0x0027_ffff,
            Generation::Pentium | Generation::P6 | Generation::P7 => base,
        }
    }

    #[inline]
    pub const fn mask_1_for(generation: Generation) -> u32 {
        let base = 0x0000_0002;
        match generation {
            Generation::I8086 | Generation::I186 => base | 0x0000_f000,
            Generation::I286
            | Generation::I386
            | Generation::I486
            | Generation::Pentium
            | Generation::P6
            | Generation::P7 => base,
        }
    }

    /// Sets the dynamic value of the FLAGS register and marks all flags as valid.
    #[inline]
    pub fn set(&mut self, value: Flags) {
        self.dynamic_value |= value;
        self.valid_mask |= value;
    }

    /// Clears the dynamic value of the FLAGS register and marks all flags as valid.
    #[inline]
    pub fn clear(&mut self, value: Flags) {
        self.dynamic_value &= !value;
        self.valid_mask |= value;
    }

    /// Sets the dynamic value of a specific flag in the FLAGS register.
    #[inline]
    pub fn set_dynamic(&mut self, flag: Flags, value: bool) {
        if value {
            self.set(flag);
        } else {
            self.clear(flag);
        }
    }

    /// Sets the static value of a specific flag in the FLAGS register.
    #[inline]
    pub fn set_static(&mut self, flag: Flags, value: bool) {
        if value {
            self.static_value |= flag;
        } else {
            self.static_value &= !flag;
        }
    }

    /// Adjusts the flags after generic arithmetic operations (ADD, SUB, etc.).
    #[inline]
    pub fn adjust_after_arith_op(&mut self, is_zero: bool) {
        self.dynamic_value = if is_zero { Flags::ZF } else { Flags::ZERO };
        self.valid_mask = Flags::ZF;
    }

    /// Adjusts the flags after INC and DEC operations.
    ///
    /// Note: INC and DEC do not affect CF
    #[inline]
    pub fn adjust_after_inc_dec(&mut self, lazy_op: &LazyOp, is_zero: bool) {
        let cf = if lazy_op.recompute_cf(self) {
            Flags::CF
        } else {
            Flags::ZERO
        };
        let zf = if is_zero { Flags::ZF } else { Flags::ZERO };
        self.dynamic_value = cf | zf;
        self.valid_mask = Flags::ZF | Flags::CF;
    }

    /// Adjusts the flags after logical operations (AND, OR, XOR).
    ///
    /// Note: Clear OF and CF on after logical operations, and set ZF according to the result. AF is undefined for logical operations.
    #[inline]
    pub fn adjust_after_logic_op(&mut self, is_zero: bool) {
        let mask: Flags = Flags::OF | Flags::ZF | Flags::AF | Flags::CF;
        let zf = if is_zero { Flags::ZF } else { Flags::ZERO };
        self.dynamic_value = (self.dynamic_value & !mask) | zf;
        self.valid_mask = mask;
    }

    /// Adjusts the flags after shift operations (SHL, SHR, SAR).
    pub fn adjust_after_shift(&mut self, is_zero: bool) {
        self.dynamic_value = if is_zero { Flags::ZF } else { Flags::ZERO };
        self.valid_mask = Flags::ZF;
    }

    #[inline]
    pub fn set_bits(&mut self, bits: Flags) {
        self.dynamic_value = bits & Self::DYNAMIC_VALUE_MASK;
        self.static_value = bits & !Self::DYNAMIC_VALUE_MASK;
        self.valid_mask = Flags::NOT_ZERO;
    }

    /// Resolves lazy flags and returns the final value of the FLAGS register.
    #[inline]
    pub fn resolve(&mut self, lazy_op: &LazyOp) -> Flags {
        lazy_op.resolve_all_flags(self);
        self.dynamic_value = self.dynamic_value & Self::DYNAMIC_VALUE_MASK;
        self.static_value = self.static_value & !Self::DYNAMIC_VALUE_MASK;
        (self.dynamic_value | self.static_value) & self.always_0_mask | self.always_1_mask
    }

    /// Discards the specified flags from the dynamic value
    #[inline]
    pub fn discard(&mut self, mask: Flags) {
        self.dynamic_value &= !mask;
        self.valid_mask |= mask;
    }

    #[inline]
    pub fn cf(&self) -> bool {
        self.dynamic_value.contains(Flags::CF)
    }

    #[inline]
    pub fn zf(&self) -> bool {
        self.dynamic_value.contains(Flags::ZF)
    }

    #[inline]
    pub fn sf(&self) -> bool {
        self.dynamic_value.contains(Flags::SF)
    }

    #[inline]
    pub fn of(&self) -> bool {
        self.dynamic_value.contains(Flags::OF)
    }

    #[inline]
    pub fn pf(&self) -> bool {
        self.dynamic_value.contains(Flags::PF)
    }
}
