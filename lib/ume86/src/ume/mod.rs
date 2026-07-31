//! User Mode Emulation

#[allow(unused_imports)]
use alloc::format;
use core::cell::UnsafeCell;

use ir86::encoding::sib::SibIndex;

use crate::_prelude_::*;
use crate::alu::Alu;
use crate::gpr::PartialRegister;
use crate::state::ProcessorState;
use crate::ume::tracer::TraceDecoder;
use crate::ume::uop::{AddrIndex, Uop, UopMinor};

pub mod tracer;
pub mod uop;

/// User Mode Emulation
///
/// ## Limitation
/// * The code segment and data segment must not overlap.
/// * During execution, it is not possible to access the segment registers.
/// * Some 16bit features are not implemented.
pub struct UME {
    data: UnsafeCell<Box<[u8]>>,
    state: ProcessorState,
    tracer: TraceDecoder,

    #[allow(dead_code)]
    debug: Box<dyn FnMut(&str)>,
}

/// Exception Code
#[derive(Debug, Clone, Copy)]
pub enum Exception {
    DivisionError,
    Unimplemented(Uop),
    StackFault,
    SegmentationViolation(Offset32),
    OutOfCode,
    RdTsc,
    Swi(u8),
}

impl UME {
    /// Creates a new instance
    pub fn new(
        code: Box<[u8]>,
        data: Box<[u8]>,
        code_base: Offset32,
        entry: Offset32,
        stack_pointer: Offset32,
        debug: Box<dyn FnMut(&str)>,
    ) -> Self {
        let tracer = TraceDecoder::new(code, code_base, entry);

        let mut state = ProcessorState::new(Generation::LATEST);
        state.init_rm();
        state.esp().write(stack_pointer.0);

        Self {
            data: UnsafeCell::new(data),
            state,
            tracer,
            debug,
        }
    }

    /// Pushes a value onto the stack
    pub fn push(&mut self, value: u32) -> Option<()> {
        let new_esp = self.state.esp().read().wrapping_sub(4);
        let p = self
            .data()
            .get_mut(new_esp as usize..new_esp as usize + 4)?;
        p.copy_from_slice(&value.to_le_bytes());
        self.state.esp().write(new_esp);
        Some(())
    }

    /// Pushes all general-purpose registers onto the stack
    pub fn pushad(&mut self) -> Option<()> {
        let esp_temp = self.state.esp().read();
        let esp_bottom = esp_temp.wrapping_sub(4 * 8);
        let p = self
            .data()
            .get_mut(esp_bottom as usize..esp_bottom as usize + 4 * 8)?;
        unsafe {
            // Safety: We have already checked that the slice is valid and has enough space for 8 u32 values.
            let p = p.as_mut_ptr() as *mut u32;
            p.add(0).write_volatile(self.state.edi().read());
            p.add(1).write_volatile(self.state.esi().read());
            p.add(2).write_volatile(self.state.ebp().read());
            p.add(3).write_volatile(esp_temp);
            p.add(4).write_volatile(self.state.ebx().read());
            p.add(5).write_volatile(self.state.edx().read());
            p.add(6).write_volatile(self.state.ecx().read());
            p.add(7).write_volatile(self.state.eax().read());
        }
        self.state.esp().write(esp_bottom);
        Some(())
    }

    /// Pops a value from the stack
    pub fn pop(&mut self) -> Option<u32> {
        let esp = self.state.esp().read();
        let new_esp = esp.wrapping_add(4);
        let p = self.data().get(esp as usize..esp as usize + 4)?;
        let value = u32::from_le_bytes(p.try_into().unwrap());
        self.state.esp().write(new_esp);
        Some(value)
    }

    /// Pops all general-purpose registers from the stack
    pub fn popad(&mut self) -> Option<()> {
        let esp_temp = self.state.esp().read();
        let p = self
            .data()
            .get(esp_temp as usize..esp_temp as usize + 4 * 8)?;
        unsafe {
            // Safety: We have already checked that the slice is valid and has enough space for 8 u32 values.
            let p = p.as_ptr() as *const u32;
            self.state.edi().write(p.add(0).read_volatile());
            self.state.esi().write(p.add(1).read_volatile());
            self.state.ebp().write(p.add(2).read_volatile());
            // Skip ESP
            self.state.ebx().write(p.add(4).read_volatile());
            self.state.edx().write(p.add(5).read_volatile());
            self.state.ecx().write(p.add(6).read_volatile());
            self.state.eax().write(p.add(7).read_volatile());
        }
        self.state.esp().modify(|esp| esp.wrapping_add(4 * 8));
        Some(())
    }

    #[inline]
    pub fn state(&self) -> &ProcessorState {
        &self.state
    }

    #[inline]
    pub fn state_mut(&mut self) -> &mut ProcessorState {
        &mut self.state
    }

    #[inline]
    pub fn tracer(&self) -> &TraceDecoder {
        &self.tracer
    }

    #[inline]
    pub fn data<'a>(&'a self) -> &'a mut [u8] {
        // Safety: We ensure that the UnsafeCell is only accessed in a single-threaded context.
        unsafe { &mut *self.data.get() }
    }

    #[inline]
    pub fn code<'a>(&'a self) -> &'a [u8] {
        self.tracer.code()
    }

    /// Updates the EIP register from the current instruction pointer
    ///
    /// # Note
    /// This function may take O(n) time in the worst case.
    pub fn reflect_eip(&mut self) {
        let current_upc = self.tracer.current_upc();
        let mut candiates = (Offset32(0), AddrIndex(0));
        for (&k, &v) in self.tracer.address_map().iter() {
            if v.0 as u32 == current_upc.0 {
                self.state.eip().write(k.0);
                return;
            }
            if v < current_upc && v > candiates.1 {
                candiates = (k, v);
            }
        }
        self.state.eip().write(candiates.0.0);
    }

    /// Adjusts the state after an exception occurs
    #[inline]
    pub fn adjust_after_exception(&mut self) {
        self.reflect_eip();
        self.state.compute_flags();
    }

    /// Resumes execution after a pause (e.g., after a syscall)
    #[inline]
    pub fn resume_next(&mut self) {
        self.tracer.advance_upc();
    }

    #[inline]
    pub fn alu<'a>(&'a mut self) -> Alu<'a> {
        Alu::new(&mut self.state)
    }

    /// Resolves the effective address from a SIB index
    #[inline]
    pub fn resolve_sib(&self, sib: SibIndex) -> u32 {
        let (base, index, scale) = sib.to_sib();
        let base = base
            .map(|base| self.state.reg(base.into()).e().read())
            .unwrap_or(0);
        let index = self
            .state
            .reg(index.into())
            .e()
            .read()
            .wrapping_shl(scale.shift());
        base.wrapping_add(index)
    }

    #[inline]
    pub fn read_memory8(&self, addr: u32) -> Result<u8, Exception> {
        let p = self
            .data()
            .get(addr as usize)
            .ok_or(Exception::SegmentationViolation(Offset32(addr)))?;
        Ok(*p)
    }

    #[inline]
    pub fn read_memory16(&self, addr: u32) -> Result<u16, Exception> {
        let p = self
            .data()
            .get(addr as usize..addr as usize + 2)
            .ok_or(Exception::SegmentationViolation(Offset32(addr)))?;
        Ok(u16::from_le_bytes(p.try_into().unwrap()))
    }

    #[inline]
    pub fn read_memory32(&self, addr: u32) -> Result<u32, Exception> {
        let p = self
            .data()
            .get(addr as usize..addr as usize + 4)
            .ok_or(Exception::SegmentationViolation(Offset32(addr)))?;
        Ok(u32::from_le_bytes(p.try_into().unwrap()))
    }

    #[inline]
    pub fn write_memory8(&mut self, addr: u32, value: u8) -> Result<(), Exception> {
        let p = self
            .data()
            .get_mut(addr as usize)
            .ok_or(Exception::SegmentationViolation(Offset32(addr)))?;
        *p = value;
        Ok(())
    }

    #[inline]
    pub fn write_memory16(&mut self, addr: u32, value: u16) -> Result<(), Exception> {
        let p = self
            .data()
            .get_mut(addr as usize..addr as usize + 2)
            .ok_or(Exception::SegmentationViolation(Offset32(addr)))?;
        p.copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    #[inline]
    pub fn write_memory32(&mut self, addr: u32, value: u32) -> Result<(), Exception> {
        let p = self
            .data()
            .get_mut(addr as usize..addr as usize + 4)
            .ok_or(Exception::SegmentationViolation(Offset32(addr)))?;
        p.copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    /// Executes the emulation
    pub fn execute(&mut self) -> Result<(), Exception> {
        self._execute().map_err(|err| {
            self.adjust_after_exception();
            err
        })
    }

    /// Executes the emulation
    fn _execute(&mut self) -> Result<(), Exception> {
        loop {
            let uop = match self.tracer.fetch_uop() {
                Some(uop) => uop,
                None => return Err(Exception::OutOfCode),
            };

            // (self.debug)(&format!(
            //     "[rust] uop: {:?} {:?}",
            //     self.tracer.current_upc(),
            //     uop
            // ));

            match uop {
                Uop::Jump(target) => {
                    self.tracer.set_current_upc(target);
                    continue;
                }
                Uop::JumpR(rs) => {
                    let target = self.state.reg(rs).e().read();
                    self.tracer.set_eip(Offset32(target));
                    continue;
                }
                Uop::Call(func_index, return_addr) => {
                    self.push(return_addr.0).ok_or(Exception::StackFault)?;
                    self.tracer.resolve_and_invoke_function(func_index);
                    continue;
                }
                Uop::CallR(rd, return_addr) => {
                    let target = self.state.reg(rd).e().read();
                    self.push(return_addr.0).ok_or(Exception::StackFault)?;
                    self.tracer.set_eip(Offset32(target));
                    continue;
                }
                Uop::Ret(iw) => {
                    let return_addr = self.pop().ok_or(Exception::StackFault)?;
                    self.state().esp().modify(|v| v.wrapping_add(iw as u32));
                    self.tracer.set_eip(Offset32(return_addr));
                    continue;
                }
                Uop::JccU(cc, target) => {
                    if self.state.eval_cc(cc) {
                        if let Some(addr_index) = self.tracer.resolve_target(target) {
                            self.tracer.replace(TraceDecoder::jcc_opt(cc, addr_index));
                            self.tracer.set_current_upc(addr_index);
                            continue;
                        } else {
                            todo!("Failed to resolve target address: {:#x}", target.0);
                        }
                    }
                }
                Uop::Jcc(cc, target) => {
                    if self.state.eval_cc(cc) {
                        self.tracer.set_current_upc(target);
                        continue;
                    }
                }
                Uop::Jz(target) => {
                    if self.state.flags().zf() {
                        self.tracer.set_current_upc(target);
                        continue;
                    }
                }
                Uop::Jnz(target) => {
                    if !self.state.flags().zf() {
                        self.tracer.set_current_upc(target);
                        continue;
                    }
                }
                Uop::SetCC(cc, rd) => {
                    let value = self.state.eval_cc(cc) as u8;
                    self.state.reg8(rd).write(value);
                }

                Uop::LoadConst8(rd, ib) => {
                    self.state.reg8(rd).write(ib);
                }
                Uop::LoadConst16(rd, iw) => {
                    self.state.reg(rd).w().write(iw);
                }
                Uop::LoadConst(rd, id) => {
                    self.state.reg(rd).e().write(id);
                }

                Uop::Move(rd, rs) => {
                    let value = self.state.reg(rs).e().read();
                    self.state.reg(rd).e().write(value);
                }
                Uop::Move8(rd, rs) => {
                    let value = self.state.reg8(rs).read();
                    self.state.reg8(rd).write(value);
                }
                Uop::Move16(rd, rs) => {
                    let value = self.state.reg(rs).w().read();
                    self.state.reg(rd).w().write(value);
                }
                Uop::MovSx8(rd, rs) => {
                    let value = self.state.reg8(rs).read() as i8 as i32 as u32;
                    self.state.reg(rd).e().write(value);
                }
                Uop::MovSx16(rd, rs) => {
                    let value = self.state.reg(rs).w().read() as i16 as i32 as u32;
                    self.state.reg(rd).e().write(value);
                }
                Uop::MovZx8(rd, rs) => {
                    let value = self.state.reg8(rs).read();
                    self.state.reg(rd).e().write(value as u32);
                }
                Uop::MovZx16(rd, rs) => {
                    let value = self.state.reg(rs).w().read();
                    self.state.reg(rd).e().write(value as u32);
                }

                Uop::NotR(rd) => {
                    let value = self.state.reg(rd).e().read();
                    self.state.reg(rd).e().write(!value);
                }
                Uop::XchgR(rd, rs) => {
                    let value_rd = self.state.reg(rd).e().read();
                    let value_rs = self.state.reg(rs).e().read();
                    self.state.reg(rd).e().write(value_rs);
                    self.state.reg(rs).e().write(value_rd);
                }

                Uop::LoadR8(rd, rb) => {
                    let base = self.state.reg(rb).e().read();
                    let value = self.read_memory8(base)?;
                    self.state.reg8(rd).write(value);
                }
                Uop::LoadR16(rd, rb) => {
                    let base = self.state.reg(rb).e().read();
                    let value = self.read_memory16(base)?;
                    self.state.reg(rd).w().write(value);
                }
                Uop::LoadR32(rd, rb) => {
                    let base = self.state.reg(rb).e().read();
                    let value = self.read_memory32(base)?;
                    self.state.reg(rd).e().write(value);
                }
                Uop::LoadBD32(rd, rb, disp) => {
                    let base = self.state.reg(rb).e().read();
                    let addr = base.wrapping_add(disp.0);
                    let value = self.read_memory32(addr)?;
                    self.state.reg(rd).e().write(value);
                }
                Uop::LoadSIB32(rd, sib, disp) => {
                    let addr = self.resolve_sib(sib).wrapping_add(disp.0);
                    let value = self.read_memory32(addr)?;
                    self.state.reg(rd).e().write(value);
                }

                Uop::LoadR32CS(rd, rb) => {
                    let base = self.state.reg(rb).e().read();
                    let addr = base as usize;
                    let value = self
                        .code()
                        .get(addr..addr + 4)
                        .ok_or(Exception::SegmentationViolation(Offset32(addr as u32)))?;
                    let value = u32::from_le_bytes(value.try_into().unwrap());
                    self.state.reg(rd).e().write(value);
                }

                Uop::StoreR8(rd, rb) => {
                    let base = self.state.reg(rb).e().read();
                    let value = self.state.reg8(rd).read();
                    self.write_memory8(base, value)?;
                }
                Uop::StoreR16(rd, rb) => {
                    let base = self.state.reg(rb).e().read();
                    let value = self.state.reg(rd).w().read();
                    self.write_memory16(base, value)?;
                }
                Uop::StoreR32(rd, rb) => {
                    let base = self.state.reg(rb).e().read();
                    let value = self.state.reg(rd).e().read();
                    self.write_memory32(base, value)?;
                }
                Uop::StoreBD32(rd, rb, disp) => {
                    let base = self.state.reg(rb).e().read();
                    let addr = base.wrapping_add(disp.0);
                    let value = self.state.reg(rd).e().read();
                    self.write_memory32(addr, value)?;
                }
                Uop::StoreSIB32(rd, sib, disp) => {
                    let addr = self.resolve_sib(sib).wrapping_add(disp.0);
                    let value = self.state.reg(rd).e().read();
                    self.write_memory32(addr, value)?;
                }

                Uop::LeaBD(rd, rb, disp) => {
                    let base = self.state.reg(rb).e().read();
                    let addr = base.wrapping_add(disp.0);
                    self.state.reg(rd).e().write(addr);
                }
                Uop::LeaSIB(rd, sib, disp) => {
                    let addr = self.resolve_sib(sib).wrapping_add(disp.0);
                    self.state.reg(rd).e().write(addr);
                }

                Uop::PushAd => {
                    self.pushad().ok_or(Exception::StackFault)?;
                }
                Uop::PushI(id) => {
                    self.push(id).ok_or(Exception::StackFault)?;
                }
                Uop::PushR(rd) => {
                    let value = self.state.reg(rd).e().read();
                    self.push(value).ok_or(Exception::StackFault)?;
                }
                Uop::PopAd => {
                    self.popad().ok_or(Exception::StackFault)?;
                }
                Uop::PopR(rd) => {
                    let value = self.pop().ok_or(Exception::StackFault)?;
                    self.state.reg(rd).e().write(value);
                }

                Uop::AddI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().add8(dst, ib);
                    self.state.reg8(rd).write(result);
                }
                Uop::AddI16(rd, iw) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().add16(dst, iw);
                    self.state.reg(rd).w().write(result);
                }
                Uop::AddI(rd, id) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().add32(dst, id);
                    self.state.reg(rd).e().write(result);
                }
                Uop::AddR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    let result = self.alu().add8(dst, src);
                    self.state.reg8(rd).write(result);
                }
                Uop::AddR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    let result = self.alu().add16(dst, src);
                    self.state.reg(rd).w().write(result);
                }
                Uop::AddR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().add32(dst, src);
                    self.state.reg(rd).e().write(result);
                }
                Uop::AddRMW(rd, rs) => {
                    let addr = self.state.reg(rd).e().read();
                    let dst = self.read_memory32(addr)?;
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().add32(dst, src);
                    self.write_memory32(addr, result)?;
                }
                Uop::IncR(rd) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().inc32(dst);
                    self.state.reg(rd).e().write(result);
                }

                Uop::SubI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().sub8(dst, ib);
                    self.state.reg8(rd).write(result);
                }
                Uop::SubI16(rd, iw) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().sub16(dst, iw);
                    self.state.reg(rd).w().write(result);
                }
                Uop::SubI(rd, id) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().sub32(dst, id);
                    self.state.reg(rd).e().write(result);
                }
                Uop::SubR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    let result = self.alu().sub8(dst, src);
                    self.state.reg8(rd).write(result);
                }
                Uop::SubR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    let result = self.alu().sub16(dst, src);
                    self.state.reg(rd).w().write(result);
                }
                Uop::SubR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().sub32(dst, src);
                    self.state.reg(rd).e().write(result);
                }
                Uop::SubRMW(rd, rs) => {
                    let addr = self.state.reg(rd).e().read();
                    let dst = self.read_memory32(addr)?;
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().sub32(dst, src);
                    self.write_memory32(addr, result)?;
                }
                Uop::DecR(rd) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().dec32(dst);
                    self.state.reg(rd).e().write(result);
                }
                Uop::NegR(rd) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().sub32(0, dst);
                    self.state.reg(rd).e().write(result);
                }

                Uop::CmpI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    self.alu().sub8(dst, ib);
                }
                Uop::CmpI16(rd, iw) => {
                    let dst = self.state.reg(rd).w().read();
                    self.alu().sub16(dst, iw);
                }
                Uop::CmpI(rd, id) => {
                    let dst = self.state.reg(rd).e().read();
                    self.alu().sub32(dst, id);
                }
                Uop::CmpR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    self.alu().sub8(dst, src);
                }
                Uop::CmpR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    self.alu().sub16(dst, src);
                }
                Uop::CmpR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    self.alu().sub32(dst, src);
                }

                Uop::AndI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().and8(dst, ib);
                    self.state.reg8(rd).write(result);
                }
                Uop::AndI16(rd, iw) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().and16(dst, iw);
                    self.state.reg(rd).w().write(result);
                }
                Uop::AndI(rd, id) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().and32(dst, id);
                    self.state.reg(rd).e().write(result);
                }
                Uop::AndR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    let result = self.alu().and8(dst, src);
                    self.state.reg8(rd).write(result);
                }
                Uop::AndR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    let result = self.alu().and16(dst, src);
                    self.state.reg(rd).w().write(result);
                }
                Uop::AndR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().and32(dst, src);
                    self.state.reg(rd).e().write(result);
                }

                Uop::OrI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().or8(dst, ib);
                    self.state.reg8(rd).write(result);
                }
                Uop::OrI16(rd, iw) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().or16(dst, iw);
                    self.state.reg(rd).w().write(result);
                }
                Uop::OrI(rd, id) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().or32(dst, id);
                    self.state.reg(rd).e().write(result);
                }
                Uop::OrR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    let result = self.alu().or8(dst, src);
                    self.state.reg8(rd).write(result);
                }
                Uop::OrR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    let result = self.alu().or16(dst, src);
                    self.state.reg(rd).w().write(result);
                }
                Uop::OrR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().or32(dst, src);
                    self.state.reg(rd).e().write(result);
                }

                Uop::XorI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().xor8(dst, ib);
                    self.state.reg8(rd).write(result);
                }
                Uop::XorI16(rd, iw) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().xor16(dst, iw);
                    self.state.reg(rd).w().write(result);
                }
                Uop::XorI(rd, id) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().xor32(dst, id);
                    self.state.reg(rd).e().write(result);
                }
                Uop::XorR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    let result = self.alu().xor8(dst, src);
                    self.state.reg8(rd).write(result);
                }
                Uop::XorR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    let result = self.alu().xor16(dst, src);
                    self.state.reg(rd).w().write(result);
                }
                Uop::XorR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().xor32(dst, src);
                    self.state.reg(rd).e().write(result);
                }

                Uop::TestI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    self.alu().and8(dst, ib);
                }
                Uop::TestI16(rd, iw) => {
                    let dst = self.state.reg(rd).w().read();
                    self.alu().and16(dst, iw);
                }
                Uop::TestI(rd, id) => {
                    let dst = self.state.reg(rd).e().read();
                    self.alu().and32(dst, id);
                }
                Uop::TestR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    self.alu().and8(dst, src);
                }
                Uop::TestR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    self.alu().and16(dst, src);
                }
                Uop::TestR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    self.alu().and32(dst, src);
                }

                Uop::IMulR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read() as i32;
                    let src = self.state.reg(rs).e().read() as i32;
                    let result = self.alu().imul32(dst, src);
                    self.state.reg(rd).e().write(result as u32);
                }
                Uop::IMulRI(rd, rs, id) => {
                    let src = self.state.reg(rs).e().read() as i32;
                    let result = self.alu().imul32(src, id);
                    self.state.reg(rd).e().write(result as u32);
                }

                Uop::IDivR(rs) => {
                    let divisor = self.state.reg(rs).e().read() as i32;
                    if divisor == 0 {
                        return Err(Exception::DivisionError);
                    }
                    let eax = self.state.eax().read();
                    let edx = self.state.edx().read();
                    let dividend = (((edx as u64) << 32) | (eax as u64)) as i64;
                    let eax_i32 = eax as i32;
                    if dividend == (eax_i32 as i64) {
                        let quotient = eax_i32 / divisor;
                        let remainder = eax_i32 % divisor;
                        self.state.eax().write(quotient as u32);
                        self.state.edx().write(remainder as u32);
                    } else {
                        let quotient = dividend / (divisor as i64);
                        let remainder = dividend % (divisor as i64);
                        if quotient > i32::MAX as i64 || quotient < i32::MIN as i64 {
                            return Err(Exception::DivisionError);
                        }
                        self.state.eax().write(quotient as u32);
                        self.state.edx().write(remainder as u32);
                    }
                }
                Uop::DivR(rs) => {
                    let divisor = self.state.reg(rs).e().read();
                    if divisor == 0 {
                        return Err(Exception::DivisionError);
                    }
                    let eax = self.state.eax().read();
                    let edx = self.state.edx().read();
                    if edx == 0 {
                        let quotient = eax / divisor;
                        let remainder = eax % divisor;
                        self.state.eax().write(quotient);
                        self.state.edx().write(remainder);
                    } else {
                        let dividend = ((edx as u64) << 32) | (eax as u64);
                        let quotient = dividend / (divisor as u64);
                        let remainder = dividend % (divisor as u64);
                        if quotient > u32::MAX as u64 {
                            return Err(Exception::DivisionError);
                        }
                        self.state.eax().write(quotient as u32);
                        self.state.edx().write(remainder as u32);
                    }
                }

                Uop::SarI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().sar8(dst, ib);
                    self.state.reg8(rd).write(result);
                }
                Uop::SarRCl8(rd) => {
                    let dst = self.state.reg8(rd).read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().sar8(dst, cl);
                    self.state.reg8(rd).write(result);
                }
                Uop::SarI16(rd, ib) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().sar16(dst, ib);
                    self.state.reg(rd).w().write(result);
                }
                Uop::SarRCl16(rd) => {
                    let dst = self.state.reg(rd).w().read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().sar16(dst, cl);
                    self.state.reg(rd).w().write(result);
                }
                Uop::SarI(rd, ib) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().sar32(dst, ib);
                    self.state.reg(rd).e().write(result);
                }
                Uop::SarRCl(rd) => {
                    let dst = self.state.reg(rd).e().read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().sar32(dst, cl);
                    self.state.reg(rd).e().write(result);
                }
                Uop::ShlI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().shl8(dst, ib);
                    self.state.reg8(rd).write(result);
                }
                Uop::ShlI16(rd, iw) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().shl16(dst, iw);
                    self.state.reg(rd).w().write(result);
                }
                Uop::ShlI(rd, ib) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().shl32(dst, ib);
                    self.state.reg(rd).e().write(result);
                }
                Uop::ShlRCl8(rd) => {
                    let dst = self.state.reg8(rd).read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().shl8(dst, cl);
                    self.state.reg8(rd).write(result);
                }
                Uop::ShlRCl16(rd) => {
                    let dst = self.state.reg(rd).w().read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().shl16(dst, cl);
                    self.state.reg(rd).w().write(result);
                }
                Uop::ShlRCl(rd) => {
                    let dst = self.state.reg(rd).e().read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().shl32(dst, cl);
                    self.state.reg(rd).e().write(result);
                }
                Uop::ShrI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().shr8(dst, ib);
                    self.state.reg8(rd).write(result);
                }
                Uop::ShrRCl8(rd) => {
                    let dst = self.state.reg8(rd).read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().shr8(dst, cl);
                    self.state.reg8(rd).write(result);
                }
                Uop::ShrI16(rd, ib) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().shr16(dst, ib);
                    self.state.reg(rd).w().write(result);
                }
                Uop::ShrRCl16(rd) => {
                    let dst = self.state.reg(rd).w().read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().shr16(dst, cl);
                    self.state.reg(rd).w().write(result);
                }
                Uop::ShrI(rd, ib) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().shr32(dst, ib);
                    self.state.reg(rd).e().write(result);
                }
                Uop::ShrRCl(rd) => {
                    let dst = self.state.reg(rd).e().read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().shr32(dst, cl);
                    self.state.reg(rd).e().write(result);
                }

                Uop::Swi(ib) => {
                    return Err(Exception::Swi(ib));
                }

                // Invalid for here
                Uop::FetchNext(_) => return Err(Exception::Unimplemented(uop)),

                Uop::Minor(mop) => match mop {
                    UopMinor::Cdq => {
                        let eax = self.state.eax().read() as i32;
                        self.state.edx().write(if eax < 0 { u32::MAX } else { 0 });
                    }
                    UopMinor::Cld => {
                        self.state_mut().flags_mut().set_static(Flags::DF, false);
                    }
                    UopMinor::Cpuid => {
                        let eax = self.state.eax().read();
                        let ecx = self.state.ecx().read();
                        let cpuid_result = cpuid(eax, ecx);
                        self.state.eax().write(cpuid_result.eax);
                        self.state.ebx().write(cpuid_result.ebx);
                        self.state.ecx().write(cpuid_result.ecx);
                        self.state.edx().write(cpuid_result.edx);
                    }
                    UopMinor::MulR(rd) => {
                        let dst = self.state.reg(rd).e().read();
                        let eax = self.state.eax().read();
                        let result = self.alu().mul32(eax, dst);
                        self.state.eax().write(result as u32);
                        self.state.edx().write((result >> 32) as u32);
                    }
                    UopMinor::RdTsc => {
                        return Err(Exception::RdTsc);
                    }
                    UopMinor::RepMovsb => {
                        let mut ecx = self.state.ecx().read();
                        if ecx > 0 {
                            let mut esi = self.state.esi().read();
                            let mut edi = self.state.edi().read();

                            while ecx > 0 {
                                let value = self.read_memory8(esi)?;
                                self.write_memory8(edi, value)?;
                                esi = esi.wrapping_add(1);
                                edi = edi.wrapping_add(1);
                                ecx = ecx.wrapping_sub(1);
                            }

                            self.state.esi().write(esi);
                            self.state.edi().write(edi);
                            self.state.ecx().write(ecx);
                        }
                    }
                    UopMinor::RepMovsd => {
                        let mut ecx = self.state.ecx().read();
                        if ecx > 0 {
                            let mut esi = self.state.esi().read();
                            let mut edi = self.state.edi().read();

                            while ecx > 0 {
                                let value = self.read_memory32(esi)?;
                                self.write_memory32(edi, value)?;
                                esi = esi.wrapping_add(4);
                                edi = edi.wrapping_add(4);
                                ecx = ecx.wrapping_sub(1);
                            }

                            self.state.esi().write(esi);
                            self.state.edi().write(edi);
                            self.state.ecx().write(ecx);
                        }
                    }
                    UopMinor::RepStosd => {
                        let mut ecx = self.state.ecx().read();
                        if ecx > 0 {
                            let mut edi = self.state.edi().read();
                            let eax = self.state.eax().read();

                            while ecx > 0 {
                                self.write_memory32(edi, eax)?;
                                edi = edi.wrapping_add(4);
                                ecx = ecx.wrapping_sub(1);
                            }

                            self.state.edi().write(edi);
                            self.state.ecx().write(ecx);
                        }
                    }
                    UopMinor::RepStosb => {
                        let mut ecx = self.state.ecx().read();
                        if ecx > 0 {
                            let mut edi = self.state.edi().read();
                            let al = self.state.al().read();

                            while ecx > 0 {
                                self.write_memory8(edi, al)?;
                                edi = edi.wrapping_add(1);
                                ecx = ecx.wrapping_sub(1);
                            }

                            self.state.edi().write(edi);
                            self.state.ecx().write(ecx);
                        }
                    }
                    UopMinor::RepZCmpsb => {
                        let mut ecx = self.state.ecx().read();
                        if ecx > 0 {
                            let mut esi = self.state.esi().read();
                            let mut edi = self.state.edi().read();

                            let mut value1 = self.read_memory8(esi)?;
                            let mut value2 = self.read_memory8(edi)?;

                            while ecx > 0 {
                                value1 = match self.read_memory8(esi) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        self.state.esi().write(esi);
                                        self.state.edi().write(edi);
                                        self.state.ecx().write(ecx);
                                        return Err(e);
                                    }
                                };
                                value2 = match self.read_memory8(edi) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        self.state.esi().write(esi);
                                        self.state.edi().write(edi);
                                        self.state.ecx().write(ecx);
                                        return Err(e);
                                    }
                                };
                                esi = esi.wrapping_add(1);
                                edi = edi.wrapping_add(1);
                                ecx = ecx.wrapping_sub(1);
                                if value1 != value2 {
                                    break;
                                }
                            }

                            self.alu().sub8(value1, value2);
                            self.state.esi().write(esi);
                            self.state.edi().write(edi);
                            self.state.ecx().write(ecx);
                        }
                    }
                    UopMinor::RepNzScasb => {
                        let mut ecx = self.state.ecx().read();
                        if ecx > 0 {
                            let mut edi = self.state.edi().read();
                            let al = self.state.al().read();

                            let mut value = self.read_memory8(edi)?;

                            while ecx > 0 {
                                value = match self.read_memory8(edi) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        self.state.edi().write(edi);
                                        self.state.ecx().write(ecx);
                                        return Err(e);
                                    }
                                };
                                edi = edi.wrapping_add(1);
                                ecx = ecx.wrapping_sub(1);
                                if value == al {
                                    break;
                                }
                            }

                            self.alu().sub8(al, value);
                            self.state.edi().write(edi);
                            self.state.ecx().write(ecx);
                        }
                    }
                },
            }
            self.tracer.advance_upc();
        }
    }
}

pub struct Cpuid {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

pub fn cpuid(eax: u32, _ecx: u32) -> Cpuid {
    let mut cpuid = Cpuid {
        eax: 0,
        ebx: 0,
        ecx: 0,
        edx: 0,
    };

    #[rustfmt::skip]
    //                                       |--+---+---|
    const MANUFACTURER_STRING: &[u8; 12] = b"GenuineNerry";

    #[rustfmt::skip]
    //                                |--+---+---+---*---+---+---+---*---+---+---+---|
    const BRAND_STRING: &[u8; 48] = b"An x86 User Mode Emulator @ 1.00GHz             ";

    let family = 4;
    let model = 0;
    let stepping = 0;
    let efamily = 0;
    let emodel = 0;
    let processor_version =
        (efamily << 20) | (emodel << 16) | (family << 8) | (model << 4) | stepping;

    match eax {
        0x0000_0000 => {
            cpuid.eax = 1;
            cpuid.ebx = u32::from_le_bytes(MANUFACTURER_STRING[0..4].try_into().unwrap());
            cpuid.edx = u32::from_le_bytes(MANUFACTURER_STRING[4..8].try_into().unwrap());
            cpuid.ecx = u32::from_le_bytes(MANUFACTURER_STRING[8..12].try_into().unwrap());
        }
        0x0000_0001 => {
            cpuid.eax = processor_version;
            cpuid.ebx = 0;
            cpuid.ecx = 0;
            cpuid.edx = 0;
        }
        0x8000_0000 => {
            cpuid.eax = 0x8000_0004;
            cpuid.ebx = 0;
            cpuid.ecx = 0;
            cpuid.edx = 0;
        }
        0x8000_0001 => {
            cpuid.eax = processor_version;
            cpuid.ebx = 0;
            cpuid.ecx = 0;
            cpuid.edx = 0;
        }
        0x8000_0002 => {
            cpuid.eax = u32::from_le_bytes(BRAND_STRING[0..4].try_into().unwrap());
            cpuid.ebx = u32::from_le_bytes(BRAND_STRING[4..8].try_into().unwrap());
            cpuid.ecx = u32::from_le_bytes(BRAND_STRING[8..12].try_into().unwrap());
            cpuid.edx = u32::from_le_bytes(BRAND_STRING[12..16].try_into().unwrap());
        }
        0x8000_0003 => {
            cpuid.eax = u32::from_le_bytes(BRAND_STRING[16..20].try_into().unwrap());
            cpuid.ebx = u32::from_le_bytes(BRAND_STRING[20..24].try_into().unwrap());
            cpuid.ecx = u32::from_le_bytes(BRAND_STRING[24..28].try_into().unwrap());
            cpuid.edx = u32::from_le_bytes(BRAND_STRING[28..32].try_into().unwrap());
        }
        0x8000_0004 => {
            cpuid.eax = u32::from_le_bytes(BRAND_STRING[32..36].try_into().unwrap());
            cpuid.ebx = u32::from_le_bytes(BRAND_STRING[36..40].try_into().unwrap());
            cpuid.ecx = u32::from_le_bytes(BRAND_STRING[40..44].try_into().unwrap());
            cpuid.edx = u32::from_le_bytes(BRAND_STRING[44..48].try_into().unwrap());
        }
        _ => {}
    }

    cpuid
}
