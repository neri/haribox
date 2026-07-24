//! Arithmetic Logic Unit (ALU)

use core::ops::{BitAnd, BitOr, BitXor};

use paste::paste;

use crate::prelude::*;
use crate::state::ProcessorState;

/// Arithmetic Logic Unit (ALU)
#[repr(transparent)]
pub struct Alu<'a>(pub &'a mut ProcessorState);

macro_rules! add_sub {
    ($name:ident, $method:ident, $lazy_op:ident, $op_size:expr) => {
        paste! {
            #[inline]
            pub fn [< $name $op_size >] (&mut self, dst: [< u $op_size >], src: [< u $op_size >]) -> [< u $op_size >] {
                let result = dst.$method(src);
                self.0.adjust_after_arith_op(result == 0);
                *self.0.lazy_op_mut() = LazyOp::[< $lazy_op $op_size >](dst, src);
                result
            }
        }
    };
    ($name:ident, $method:ident, $lazy_op:ident) => {
        add_sub!($name, $method, $lazy_op, 8);
        add_sub!($name, $method, $lazy_op, 16);
        add_sub!($name, $method, $lazy_op, 32);
    }
}

macro_rules! adc_sbb {
    ($name:ident, $method:ident, $lazy_op:ident, $op_size:expr) => {
        paste! {
            #[inline]
            pub fn [< $name $op_size >] (&mut self, dst: [< u $op_size >], src: [< u $op_size >]) -> [< u $op_size >] {
                let cf = self.0.recompute_cf();
                let result = dst.$method(src).$method(cf as [< u $op_size >]);
                self.0.adjust_after_arith_op(result == 0);
                *self.0.lazy_op_mut() = LazyOp::[< $lazy_op $op_size >](dst, src, cf);
                result
            }
        }
    };
    ($name:ident, $method:ident, $lazy_op:ident) => {
        adc_sbb!($name, $method, $lazy_op, 8);
        adc_sbb!($name, $method, $lazy_op, 16);
        adc_sbb!($name, $method, $lazy_op, 32);
    }
}

macro_rules! inc_dec {
    ($name:ident, $method:ident, $lazy_op:ident, $op_size:expr) => {
        paste! {
            #[inline]
            pub fn [< $name $op_size >] (&mut self, dst: [< u $op_size >]) -> [< u $op_size >] {
                let result = dst.$method(1);
                self.0.adjust_after_inc_dec(result == 0);
                *self.0.lazy_op_mut() = LazyOp::[< $lazy_op $op_size >](dst);
                result
            }
        }
    };
    ($name:ident, $method:ident, $lazy_op:ident) => {
        inc_dec!($name, $method, $lazy_op, 8);
        inc_dec!($name, $method, $lazy_op, 16);
        inc_dec!($name, $method, $lazy_op, 32);
    };
}

macro_rules! logic_op {
    ($name:ident, $method:ident, $lazy_op:ident, $op_size:expr) => {
        paste! {
            #[inline]
            pub fn [< $name $op_size >] (&mut self, dst: [< u $op_size >], src: [< u $op_size >]) -> [< u $op_size >] {
                let result = dst.$method(src);
                self.0.adjust_after_logic_op(result == 0);
                *self.0.lazy_op_mut() = LazyOp::[< $lazy_op $op_size >](dst, src);
                result
            }
        }
    };
    ($name:ident, $method:ident, $lazy_op:ident) => {
        logic_op!($name, $method, $lazy_op, 8);
        logic_op!($name, $method, $lazy_op, 16);
        logic_op!($name, $method, $lazy_op, 32);
    }
}

macro_rules! shift_op {
    ($name:ident, $method:ident, $lazy_op:ident, $op_size:expr) => {
        paste! {
            #[inline]
            pub fn [< $name $op_size >] (&mut self, dst: [< u $op_size >], count: u8) -> [< u $op_size >] {
                let count = count & 0x1f;
                if count > 0 {
                    let result = dst.$method(count as u32);
                    self.0.adjust_after_shift(result == 0);
                    *self.0.lazy_op_mut() = LazyOp::[< $lazy_op $op_size >](dst, count);
                    result
                } else {
                    dst
                }
            }
        }
    };
    ($name:ident, $method:ident, $lazy_op:ident) => {
        shift_op!($name, $method, $lazy_op, 8);
        shift_op!($name, $method, $lazy_op, 16);
        shift_op!($name, $method, $lazy_op, 32);
    }
}

impl<'a> Alu<'a> {
    #[inline]
    pub fn new(state: &'a mut ProcessorState) -> Self {
        Self(state)
    }
}

impl Alu<'_> {
    add_sub!(add, wrapping_add, Add);
    add_sub!(sub, wrapping_sub, Sub);

    adc_sbb!(adc, wrapping_add, Adc);
    adc_sbb!(sbb, wrapping_sub, Sbb);

    inc_dec!(inc, wrapping_add, Inc);
    inc_dec!(dec, wrapping_sub, Dec);

    logic_op!(and, bitand, And);
    logic_op!(or, bitor, Or);
    logic_op!(xor, bitxor, Xor);

    pub fn mul8(&mut self, dst: u8, src: u8) -> u16 {
        let result = (dst as u16).wrapping_mul(src as u16);
        self.0.flags_mut().unresolve(Flags::CF | Flags::OF);
        *self.0.lazy_op_mut() = LazyOp::Mul8(dst, src);
        result
    }

    pub fn mul16(&mut self, dst: u16, src: u16) -> u32 {
        let result = (dst as u32).wrapping_mul(src as u32);
        self.0.flags_mut().unresolve(Flags::CF | Flags::OF);
        *self.0.lazy_op_mut() = LazyOp::Mul16(dst, src);
        result
    }

    pub fn mul32(&mut self, dst: u32, src: u32) -> u64 {
        let result = (dst as u64).wrapping_mul(src as u64);
        self.0.flags_mut().unresolve(Flags::CF | Flags::OF);
        *self.0.lazy_op_mut() = LazyOp::Mul32(dst, src);
        result
    }

    pub fn imul8(&mut self, dst: i8, src: i8) -> i8 {
        let result = dst.wrapping_mul(src);
        self.0.flags_mut().unresolve(Flags::CF | Flags::OF);
        *self.0.lazy_op_mut() = LazyOp::IMul8(dst, src);
        result
    }

    pub fn imul16(&mut self, dst: i16, src: i16) -> i16 {
        let result = dst.wrapping_mul(src);
        self.0.flags_mut().unresolve(Flags::CF | Flags::OF);
        *self.0.lazy_op_mut() = LazyOp::IMul16(dst, src);
        result
    }

    pub fn imul32(&mut self, dst: i32, src: i32) -> i32 {
        let result = dst.wrapping_mul(src);
        self.0.flags_mut().unresolve(Flags::CF | Flags::OF);
        *self.0.lazy_op_mut() = LazyOp::IMul32(dst, src);
        result
    }

    shift_op!(shl, wrapping_shl, Shl);
    shift_op!(shr, wrapping_shr, Shr);

    pub fn sar8(&mut self, dst: u8, count: u8) -> u8 {
        let count = count & 0x1f;
        if count > 0 {
            let dst = dst as i8;
            let result = (dst.wrapping_shr(count as u32)) as u8;
            self.0.adjust_after_shift(result == 0);
            *self.0.lazy_op_mut() = LazyOp::Sar8(dst, count);
            result
        } else {
            dst
        }
    }

    pub fn sar16(&mut self, dst: u16, count: u8) -> u16 {
        let count = count & 0x1f;
        if count > 0 {
            let dst = dst as i16;
            let result = (dst.wrapping_shr(count as u32)) as u16;
            self.0.adjust_after_shift(result == 0);
            *self.0.lazy_op_mut() = LazyOp::Sar16(dst, count);
            result
        } else {
            dst
        }
    }

    pub fn sar32(&mut self, dst: u32, count: u8) -> u32 {
        let count = count & 0x1f;
        if count > 0 {
            let dst = dst as i32;
            let result = (dst.wrapping_shr(count as u32)) as u32;
            self.0.adjust_after_shift(result == 0);
            *self.0.lazy_op_mut() = LazyOp::Sar32(dst, count);
            result
        } else {
            dst
        }
    }
}
