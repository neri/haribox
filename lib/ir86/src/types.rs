//! Data Types for x86

use crate::encoding::Scale;

/// Segment Selector Type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentSelector(pub u16);

/// 16bit Offset Type
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Offset16(pub u16);

impl Offset16 {
    #[inline]
    pub const fn wrapping_add(self, rhs: Self) -> Self {
        Offset16(self.0.wrapping_add(rhs.0))
    }

    #[inline]
    pub const fn wrapping_shl(self, rhs: Scale) -> Self {
        Self(self.0.wrapping_shl(rhs.shift()))
    }
}

impl core::fmt::Debug for Offset16 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Offset(0x{:04x})", self.0)
    }
}

/// 32bit Offset Type
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Offset32(pub u32);

impl Offset32 {
    #[inline]
    pub const fn wrapping_add(self, rhs: Self) -> Self {
        Offset32(self.0.wrapping_add(rhs.0))
    }

    #[inline]
    pub const fn wrapping_shl(self, rhs: Scale) -> Self {
        Self(self.0.wrapping_shl(rhs.shift()))
    }
}

impl core::fmt::Debug for Offset32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Offset(0x{:08x})", self.0)
    }
}
