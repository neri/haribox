use crate::prelude::*;

/// 16bit Far Pointer Type on Real Mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Far16Ptr(pub u32);

impl Far16Ptr {
    #[inline]
    pub const fn sel(&self) -> SegmentSelector {
        SegmentSelector((self.0 >> 16) as u16)
    }

    #[inline]
    pub const fn offset(&self) -> Offset16 {
        Offset16(self.0 as u16)
    }

    /// Returns the linear address
    #[inline]
    pub const fn as_linear(&self) -> Linear32 {
        let sel = self.sel().0 as u32;
        let offset = self.offset().0 as u32;
        Linear32(sel * 16 + offset)
    }
}

/// 32bit Linear Address Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Linear32(pub u32);

impl Linear32 {
    #[inline]
    pub const fn wrapping_add(self, rhs: u32) -> Self {
        Linear32(self.0.wrapping_add(rhs))
    }
}

/// Segment Limit Type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limit(pub u32);

/// I/O Port Address Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IoPort(pub u16);
