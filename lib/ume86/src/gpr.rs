//! General Purpose Registers

use core::cell::UnsafeCell;
use core::mem::transmute;
use core::sync::atomic::{AtomicU8, AtomicU16, AtomicU32, Ordering};

use crate::prelude::*;

/// General Purpose Register
pub struct GeneralPurposeRegister(UnsafeCell<GprRepr>);

/// Internal Representation of a general purpose register
// #[derive(Clone, Copy)]
union GprRepr {
    u32: u32,
    u16: [u16; 2],
    u8: [u8; 4],
}

impl GeneralPurposeRegister {
    #[inline]
    pub const fn new(value: u32) -> Self {
        Self(UnsafeCell::new(GprRepr { u32: value }))
    }

    /// Returns a reference to a 32-bit partial register.
    #[inline]
    pub fn e<'a>(&'a self) -> Gpr32<'a> {
        // Safety: The conversion is safe
        Gpr32(unsafe { transmute(&(*self.0.get()).u32) })
    }

    /// Returns a reference to a 16-bit partial register.
    ///
    /// Alias for `x()`
    #[inline]
    pub fn w<'a>(&'a self) -> Gpr16<'a> {
        self.x()
    }

    /// Returns a reference to a 16-bit partial register.
    ///
    /// Alias for `w()`
    #[inline]
    pub fn x<'a>(&'a self) -> Gpr16<'a> {
        // Safety: The conversion is safe
        Gpr16(unsafe { transmute(&(*self.0.get()).u16[0]) })
    }

    /// Returns a reference to the lower 8-bit partial register.
    ///
    /// Alias for `l()`
    #[inline]
    pub fn b<'a>(&'a self) -> Gpr8<'a> {
        self.l()
    }

    /// Returns a reference to the lower 8-bit partial register.
    ///
    /// Alias for `b()`
    #[inline]
    pub fn l<'a>(&'a self) -> Gpr8<'a> {
        // Safety: The conversion is safe
        Gpr8(unsafe { transmute(&(*self.0.get()).u8[0]) })
    }

    /// Returns a reference to the higher 8-bit partial register.
    #[inline]
    pub fn h<'a>(&'a self) -> Gpr8<'a> {
        // Safety: The conversion is safe
        Gpr8(unsafe { transmute(&(*self.0.get()).u8[1]) })
    }
}

impl Clone for GeneralPurposeRegister {
    #[inline]
    fn clone(&self) -> Self {
        GeneralPurposeRegister(UnsafeCell::new(GprRepr {
            u32: self.e().read(),
        }))
    }
}

/// Partial Register
pub trait PartialRegister {
    type ValType: Copy + PartialEq + Eq + PartialOrd + Ord;

    fn read(&self) -> Self::ValType;

    fn write(&self, value: Self::ValType);

    #[inline]
    fn modify<F>(&self, f: F) -> Self::ValType
    where
        F: FnOnce(Self::ValType) -> Self::ValType,
    {
        let mut value = self.read();
        value = f(value);
        self.write(value);
        value
    }
}

pub struct Gpr32<'a>(&'a AtomicU32);

impl PartialRegister for Gpr32<'_> {
    type ValType = u32;

    #[inline]
    fn read(&self) -> Self::ValType {
        self.0.load(Ordering::Relaxed).to_le()
    }

    #[inline]
    fn write(&self, value: Self::ValType) {
        self.0.store(value.to_le(), Ordering::Relaxed);
    }
}

pub struct Gpr16<'a>(&'a AtomicU16);

impl PartialRegister for Gpr16<'_> {
    type ValType = u16;

    #[inline]
    fn read(&self) -> Self::ValType {
        self.0.load(Ordering::Relaxed).to_le()
    }

    #[inline]
    fn write(&self, value: Self::ValType) {
        self.0.store(value.to_le(), Ordering::Relaxed);
    }
}

pub struct Gpr8<'a>(&'a AtomicU8);

impl PartialRegister for Gpr8<'_> {
    type ValType = u8;

    #[inline]
    fn read(&self) -> Self::ValType {
        self.0.load(Ordering::Relaxed)
    }

    #[inline]
    fn write(&self, value: Self::ValType) {
        self.0.store(value, Ordering::Relaxed);
    }
}

/// Segment Register
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentRegister {
    pub sel: SegmentSelector,
    pub base: Linear32,
    pub limit: Limit,
    pub ar: u16,
}

impl SegmentRegister {
    #[inline]
    pub fn load_rm(&mut self, sel: SegmentSelector) {
        self.sel = sel;
        self.base = Linear32(sel.0 as u32 * 16);
    }

    #[inline]
    pub fn init_data_rm(&mut self) {
        self.sel = SegmentSelector(0);
        self.base = Linear32(0);
        self.limit = Limit(0xffff);
        self.ar = 0x92;
    }

    #[inline]
    pub fn init_code_rm(&mut self) {
        self.sel = SegmentSelector(0xf000);
        self.base = Linear32(0xffff_0000);
        self.limit = Limit(0xffff);
        self.ar = 0x9a;
    }

    #[inline]
    pub fn ea16(&self, offset: Offset16) -> Linear32 {
        Linear32(self.base.0.wrapping_add(offset.0 as u32))
    }

    #[inline]
    pub fn ea32(&self, offset: Offset32) -> Linear32 {
        Linear32(self.base.0.wrapping_add(offset.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::prelude::*;

    #[test]
    fn gpr() {
        let ax = GeneralPurposeRegister::new(0x12345678);

        assert_eq!(ax.e().read(), 0x12345678);
        assert_eq!(ax.x().read(), 0x5678);
        assert_eq!(ax.l().read(), 0x78);
        assert_eq!(ax.h().read(), 0x56);
        assert_eq!(ax.clone().e().read(), 0x12345678);

        ax.x().write(0x9abc);
        assert_eq!(ax.e().read(), 0x12349abc);
        assert_eq!(ax.x().read(), 0x9abc);
        assert_eq!(ax.l().read(), 0xbc);
        assert_eq!(ax.h().read(), 0x9a);
        assert_eq!(ax.clone().e().read(), 0x12349abc);

        ax.l().write(0xef);
        assert_eq!(ax.e().read(), 0x12349aef);
        assert_eq!(ax.x().read(), 0x9aef);
        assert_eq!(ax.l().read(), 0xef);
        assert_eq!(ax.h().read(), 0x9a);
        assert_eq!(ax.clone().e().read(), 0x12349aef);

        ax.h().write(0xcd);
        assert_eq!(ax.e().read(), 0x1234cdef);
        assert_eq!(ax.x().read(), 0xcdef);
        assert_eq!(ax.l().read(), 0xef);
        assert_eq!(ax.h().read(), 0xcd);
        assert_eq!(ax.clone().e().read(), 0x1234cdef);
    }
}
