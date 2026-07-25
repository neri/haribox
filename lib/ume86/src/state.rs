//! Processor State

use core::mem::MaybeUninit;

use ir86::prelude::registers::*;

use super::gpr::*;
use crate::flags::{FlagsRegister, LazyOp};
use crate::gpr::{GeneralPurposeRegister, SegmentRegister};
use crate::prelude::*;

/// Processor State
#[derive(Clone)]
pub struct ProcessorState {
    gpr: [GeneralPurposeRegister; 16],
    sr: [SegmentRegister; 6],

    flags: FlagsRegister,
    lazy_op: LazyOp,

    generation: Generation,
}

/// Runtime Register Index
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RtReg {
    EAX = 0,
    ECX = 1,
    EDX = 2,
    EBX = 3,
    ESP = 4,
    EBP = 5,
    ESI = 6,
    EDI = 7,
    // EIP
    EIP = 8,
    // Memory Effective Address
    MemAddr,
    // Memory Temp Data
    MemData,
    // Zero Register
    Zero,
}

/// Runtime Register Index for 8-bit registers
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RtReg8 {
    AL = 0,
    CL = 1,
    DL = 2,
    BL = 3,
    AH = 4,
    CH = 5,
    DH = 6,
    BH = 7,
    MemData,
}

impl From<GprIndex32> for RtReg {
    fn from(index: GprIndex32) -> Self {
        match index {
            EAX => RtReg::EAX,
            ECX => RtReg::ECX,
            EDX => RtReg::EDX,
            EBX => RtReg::EBX,
            ESP => RtReg::ESP,
            EBP => RtReg::EBP,
            ESI => RtReg::ESI,
            EDI => RtReg::EDI,
        }
    }
}

impl From<GprIndex8> for RtReg8 {
    fn from(index: GprIndex8) -> Self {
        match index {
            AL => RtReg8::AL,
            CL => RtReg8::CL,
            DL => RtReg8::DL,
            BL => RtReg8::BL,
            AH => RtReg8::AH,
            CH => RtReg8::CH,
            DH => RtReg8::DH,
            BH => RtReg8::BH,
        }
    }
}

impl ProcessorState {
    /// Creates a new processor state with the specified CPU generation.
    pub fn new(generation: Generation) -> Self {
        let mut state = unsafe { MaybeUninit::<Self>::zeroed().assume_init() };
        state.init(generation);
        state
    }

    /// Initializes the processor state based on the CPU generation.
    pub fn init(&mut self, generation: Generation) {
        self.generation = generation;
        self.flags = FlagsRegister::new(generation);
        self.lazy_op = LazyOp::default();
    }

    /// Initializes the processor state for real mode.
    pub fn init_rm(&mut self) {
        self.flags = FlagsRegister::new(self.generation);
        self.lazy_op = LazyOp::default();

        self.sr_mut(CS).init_code_rm();
        self.sr_mut(DS).init_data_rm();
        self.sr_mut(ES).init_data_rm();
        self.sr_mut(FS).init_data_rm();
        self.sr_mut(GS).init_data_rm();
        self.sr_mut(SS).init_data_rm();

        self.eax().write(0);
        self.ebx().write(0);
        self.ecx().write(0);
        self.edx().write(0);
        self.esp().write(0);
        self.ebp().write(0);
        self.esi().write(0);
        self.edi().write(0);

        self.eip().write(0x0000_fff0);
    }

    /// Returns the CPU generation.
    #[inline]
    pub const fn generation(&self) -> Generation {
        self.generation
    }

    /// Returns the general-purpose register at the specified index.
    #[inline]
    pub fn gpr(&self, index: GprIndex32) -> &GeneralPurposeRegister {
        self.runtime(index.into())
    }

    /// Returns the runtime register at the specified index.
    #[inline]
    pub fn runtime(&self, index: RtReg) -> &GeneralPurposeRegister {
        &self.gpr[index as usize]
    }

    #[inline]
    pub fn gpr16<'a>(&'a self, index: GprIndex16) -> Gpr16<'a> {
        self.gpr(index.upgrade()).w()
    }

    #[inline]
    pub fn gpr8<'a>(&'a self, index: GprIndex8) -> Gpr8<'a> {
        self.rt8(index.into())
    }

    #[inline]
    pub fn rt8<'a>(&'a self, index: RtReg8) -> Gpr8<'a> {
        match index {
            RtReg8::AL => self.al(),
            RtReg8::CL => self.cl(),
            RtReg8::DL => self.dl(),
            RtReg8::BL => self.bl(),
            RtReg8::AH => self.ah(),
            RtReg8::CH => self.ch(),
            RtReg8::DH => self.dh(),
            RtReg8::BH => self.bh(),
            RtReg8::MemData => self.runtime(RtReg::MemData).l(),
        }
    }

    /// Returns the segment register at the specified index.
    #[inline]
    pub fn sr(&self, index: SrIndex) -> &SegmentRegister {
        &self.sr[index as usize]
    }

    /// Returns a mutable reference to the segment register at the specified index.
    #[inline]
    pub fn sr_mut(&mut self, index: SrIndex) -> &mut SegmentRegister {
        &mut self.sr[index as usize]
    }

    #[inline]
    pub fn al<'a>(&'a self) -> Gpr8<'a> {
        self.gpr(EAX).l()
    }

    #[inline]
    pub fn ah<'a>(&'a self) -> Gpr8<'a> {
        self.gpr(EAX).h()
    }

    #[inline]
    pub fn ax<'a>(&'a self) -> Gpr16<'a> {
        self.gpr(EAX).x()
    }

    #[inline]
    pub fn eax<'a>(&'a self) -> Gpr32<'a> {
        self.gpr(EAX).e()
    }

    #[inline]
    pub fn bl<'a>(&'a self) -> Gpr8<'a> {
        self.gpr(EBX).l()
    }

    #[inline]
    pub fn bh<'a>(&'a self) -> Gpr8<'a> {
        self.gpr(EBX).h()
    }

    #[inline]
    pub fn bx<'a>(&'a self) -> Gpr16<'a> {
        self.gpr(EBX).x()
    }

    #[inline]
    pub fn ebx<'a>(&'a self) -> Gpr32<'a> {
        self.gpr(EBX).e()
    }

    #[inline]
    pub fn cl<'a>(&'a self) -> Gpr8<'a> {
        self.gpr(ECX).l()
    }

    #[inline]
    pub fn ch<'a>(&'a self) -> Gpr8<'a> {
        self.gpr(ECX).h()
    }

    #[inline]
    pub fn cx<'a>(&'a self) -> Gpr16<'a> {
        self.gpr(ECX).x()
    }

    #[inline]
    pub fn ecx<'a>(&'a self) -> Gpr32<'a> {
        self.gpr(ECX).e()
    }

    #[inline]
    pub fn dl<'a>(&'a self) -> Gpr8<'a> {
        self.gpr(EDX).l()
    }

    #[inline]
    pub fn dh<'a>(&'a self) -> Gpr8<'a> {
        self.gpr(EDX).h()
    }

    #[inline]
    pub fn dx<'a>(&'a self) -> Gpr16<'a> {
        self.gpr(EDX).x()
    }

    #[inline]
    pub fn edx<'a>(&'a self) -> Gpr32<'a> {
        self.gpr(EDX).e()
    }

    #[inline]
    pub fn sp<'a>(&'a self) -> Gpr16<'a> {
        self.gpr(ESP).x()
    }

    #[inline]
    pub fn esp<'a>(&'a self) -> Gpr32<'a> {
        self.gpr(ESP).e()
    }

    #[inline]
    pub fn bp<'a>(&'a self) -> Gpr16<'a> {
        self.gpr(EBP).x()
    }

    #[inline]
    pub fn ebp<'a>(&'a self) -> Gpr32<'a> {
        self.gpr(EBP).e()
    }

    #[inline]
    pub fn si<'a>(&'a self) -> Gpr16<'a> {
        self.gpr(ESI).x()
    }

    #[inline]
    pub fn esi<'a>(&'a self) -> Gpr32<'a> {
        self.gpr(ESI).e()
    }

    #[inline]
    pub fn di<'a>(&'a self) -> Gpr16<'a> {
        self.gpr(EDI).x()
    }

    #[inline]
    pub fn edi<'a>(&'a self) -> Gpr32<'a> {
        self.gpr(EDI).e()
    }

    #[inline]
    pub fn eip<'a>(&'a self) -> Gpr32<'a> {
        self.runtime(RtReg::EIP).e()
    }

    #[inline]
    pub fn cs(&self) -> &SegmentRegister {
        self.sr(CS)
    }

    #[inline]
    pub fn ds(&self) -> &SegmentRegister {
        self.sr(DS)
    }

    #[inline]
    pub fn es(&self) -> &SegmentRegister {
        self.sr(ES)
    }

    #[inline]
    pub fn fs(&self) -> &SegmentRegister {
        self.sr(FS)
    }

    #[inline]
    pub fn gs(&self) -> &SegmentRegister {
        self.sr(GS)
    }

    #[inline]
    pub fn ss(&self) -> &SegmentRegister {
        self.sr(SS)
    }

    #[inline]
    pub fn flags(&self) -> &FlagsRegister {
        &self.flags
    }

    #[inline]
    pub fn flags_mut(&mut self) -> &mut FlagsRegister {
        &mut self.flags
    }

    #[inline]
    pub fn compute_flags(&mut self) -> Flags {
        self.flags.resolve(&self.lazy_op)
    }

    #[inline]
    pub fn lazy_op(&self) -> &LazyOp {
        &self.lazy_op
    }

    #[inline]
    pub fn lazy_op_mut(&mut self) -> &mut LazyOp {
        &mut self.lazy_op
    }

    #[inline]
    pub fn recompute_cf(&mut self) -> bool {
        self.lazy_op.recompute_cf(&mut self.flags)
    }

    #[inline]
    pub fn resolve_cf(&mut self) -> bool {
        self.lazy_op.resolve_cf(&mut self.flags);
        self.flags.cf()
    }

    #[inline]
    pub fn resolve_sf(&mut self) -> bool {
        self.lazy_op.resolve_sf(&mut self.flags);
        self.flags.sf()
    }

    #[inline]
    pub fn resolve_of(&mut self) -> bool {
        self.lazy_op.resolve_of(&mut self.flags);
        self.flags.of()
    }

    #[inline]
    pub fn resolve_pf(&mut self) -> bool {
        self.lazy_op.resolve_pf(&mut self.flags);
        self.flags.pf()
    }

    /// Adjusts the flags after logic operations (AND, OR, XOR, etc.).
    #[inline]
    pub fn adjust_after_logic_op(&mut self, is_zero: bool) {
        self.flags.adjust_after_logic_op(is_zero);
    }

    /// Adjusts the flags after generic arithmetic operations (ADD, SUB, etc.).
    #[inline]
    pub fn adjust_after_arith_op(&mut self, is_zero: bool) {
        self.flags.adjust_after_arith_op(is_zero);
    }

    /// Adjusts the flags after INC and DEC operations.
    #[inline]
    pub fn adjust_after_inc_dec(&mut self, is_zero: bool) {
        let lazy_op = &self.lazy_op;
        self.flags.adjust_after_inc_dec(lazy_op, is_zero);
    }

    /// Adjusts the flags after shift operations (SHL, SHR, SAR).
    #[inline]
    pub fn adjust_after_shift(&mut self, is_zero: bool) {
        self.flags.adjust_after_shift(is_zero);
    }

    /// Evaluates the condition code based on the current flags.
    pub fn eval_cc(&mut self, cc: CC) -> bool {
        match cc {
            CC::O => self.resolve_of(),
            CC::NO => !self.resolve_of(),
            CC::C => self.resolve_cf(),
            CC::NC => !self.resolve_cf(),
            CC::Z => self.flags.zf(),
            CC::NZ => !self.flags.zf(),
            CC::BE => self.flags.zf() || self.resolve_cf(),
            CC::NBE => !(self.flags.zf() || self.resolve_cf()),
            CC::S => self.resolve_sf(),
            CC::NS => !self.resolve_sf(),
            CC::P => self.resolve_pf(),
            CC::NP => !self.resolve_pf(),
            CC::L => self.resolve_sf() != self.resolve_of(),
            CC::NL => self.resolve_sf() == self.resolve_of(),
            CC::LE => {
                let sf = self.resolve_sf();
                let of = self.resolve_of();
                self.flags.zf() || (sf != of)
            }
            CC::NLE => {
                let sf = self.resolve_sf();
                let of = self.resolve_of();
                !(self.flags.zf() || (sf != of))
            }
        }
    }
}
