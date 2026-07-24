//! Trace based micro IR decoder

use crate::_prelude_::*;
use crate::state::{RtReg, RtReg8};
use crate::ume::uop::{AddrIndex, FuncIndex, Uop, UopMinor};

/// Trace based micro IR decoder
pub struct TraceDecoder {
    code: Box<[u8]>,
    code_base: Offset32,
    eip_to_fetch: Offset32,
    fetch_size: usize,
    max_step: usize,

    current_upc: AddrIndex,
    uop_cache: Vec<Uop>,
    address_map: BTreeMap<Offset32, AddrIndex>,

    functions: Vec<(Offset32, AddrIndex)>,
    function_map: BTreeMap<Offset32, FuncIndex>,
}

impl TraceDecoder {
    /// Creates a new instance
    #[inline]
    pub fn new(code: Box<[u8]>, code_base: Offset32, eip: Offset32) -> Self {
        let trailing_zeros = code.iter().rev().take_while(|&&b| b == 0).count();
        let trailing_nops = code.iter().rev().take_while(|&&b| b == 0x90).count();
        let fetch_size = code.len() - trailing_zeros - trailing_nops;

        let mut result = Self {
            code,
            code_base,
            eip_to_fetch: eip,
            fetch_size,
            max_step: 1024,

            current_upc: AddrIndex(0),
            uop_cache: Vec::new(),
            function_map: BTreeMap::new(),

            address_map: BTreeMap::new(),
            functions: Vec::new(),
        };
        result._fetch_and_decode(1);
        result
    }

    #[inline]
    pub fn code(&self) -> &[u8] {
        &self.code
    }

    /// Returns the current instruction and fetches if necessary
    #[inline]
    pub fn peek_uop(&mut self) -> Option<Uop> {
        if let Some(&uop) = self.uop_cache.get(self.current_upc.0 as usize) {
            match uop {
                Uop::FetchNext(eip) => {
                    return self.handle_fetch_next(eip);
                }
                _ => return Some(uop),
            }
        } else {
            self._peek_uop2()
        }
    }

    fn handle_fetch_next(&mut self, eip: Offset32) -> Option<Uop> {
        self.eip_to_fetch = eip;
        if self.uop_cache.len() == 1 + self.current_upc.0 as usize {
            self.uop_cache.pop();
        } else {
            let addr_index = AddrIndex(self.uop_cache.len() as u32);
            self.replace(Uop::Jump(addr_index));
            self.current_upc = addr_index;
        }
        self.address_map.remove(&eip);
        self.fetch_and_decode();
        self.uop_cache.get(self.current_upc.0 as usize).copied()
    }

    fn _peek_uop2(&mut self) -> Option<Uop> {
        self.fetch_and_decode();
        self.uop_cache.get(self.current_upc.0 as usize).copied()
    }

    /// Advances the current UPC to the next instruction in the cache.
    #[inline]
    pub fn advance_upc(&mut self) {
        self.current_upc.0 += 1;
    }

    /// Returns the current instruction pointer.
    #[inline]
    pub const fn current_upc(&self) -> AddrIndex {
        self.current_upc
    }

    /// Sets the current instruction pointer.
    #[inline]
    pub fn set_current_upc(&mut self, upc: AddrIndex) {
        self.current_upc = upc;
    }

    /// Returns the address map.
    ///
    /// # Note
    /// This function is intended for testing and debugging purposes. It may be removed or modified in future versions.
    #[inline]
    pub fn address_map(&self) -> &BTreeMap<Offset32, AddrIndex> {
        &self.address_map
    }

    /// Returns the function map.
    ///
    /// # Note
    /// This function is intended for testing and debugging purposes. It may be removed or modified in future versions.
    #[inline]
    pub fn functions(&self) -> &[(Offset32, AddrIndex)] {
        &self.functions
    }

    /// Returns the uop cache.
    ///
    /// # Note
    /// This function is intended for testing and debugging purposes. It may be removed or modified in future versions.
    #[inline]
    pub fn uop_cache(&self) -> &[Uop] {
        &self.uop_cache
    }

    /// Resolves the target address to an AddrIndex, fetching and decoding if necessary.
    pub fn resolve_target(&mut self, target: Offset32) -> Option<AddrIndex> {
        if let Some(&addr_index) = self.address_map.get(&target) {
            Some(addr_index)
        } else {
            let old_upc = self.current_upc;
            self.eip_to_fetch = target;
            self.fetch_and_decode();
            self.current_upc = old_upc;
            self.address_map.get(&target).copied()
        }
    }

    /// Sets the instruction pointer to the specified EIP and updates the current UPC accordingly.
    pub fn set_eip(&mut self, eip: Offset32) {
        self.eip_to_fetch = eip;
        self.current_upc = match self.resolve_target(eip) {
            Some(v) => v,
            None => {
                // TODO: Handle the case where the EIP is not found in the address map after fetching and decoding.
                todo!(
                    "Failed to fetch and decode instruction at EIP={:#010x}",
                    eip.0
                );
            }
        };
    }

    /// Replaces the current instruction with a new Uop in the cache.
    pub fn replace(&mut self, new_uop: Uop) {
        self.uop_cache[self.current_upc.0 as usize] = new_uop;
    }

    /// Resolves the function at the given index and updates the current UPC to point to it.
    pub fn resolve_and_invoke_function(&mut self, func_index: FuncIndex) {
        let function = self.functions[func_index.0 as usize];
        if function.1 != AddrIndex(u32::MAX) {
            // Function already resolved
            self.current_upc = function.1;
        } else if let Some(&target) = self.address_map.get(&function.0) {
            // Function is not yet resolved, but we have a target address in the address map
            self.functions[func_index.0 as usize].1 = target;
            self.current_upc = target;
        } else {
            // Function is not yet resolved, and we don't have a target address in the address map
            self.set_eip(function.0);
            if let Some(&target) = self.address_map.get(&function.0) {
                self.functions[func_index.0 as usize].1 = target;
                // self.current_upc = target;
            } else {
                todo!("Failed to resolve function at EIP={:#010x}", function.0.0);
            }
        }
    }

    /// Generates a best effort LEA sequence for the given memory operand and destination register.
    pub fn generate_lea(rd: RtReg, memopr: MemOpr32) -> Uop {
        match memopr.base_index {
            BaseIndex32::DispOnly => Uop::LoadConst(rd, memopr.disp.0),
            BaseIndex32::Base(base) => {
                if memopr.disp.0 == 0 {
                    Uop::Move(rd, base.into())
                } else {
                    Uop::LeaBD(rd, base.into(), memopr.disp)
                }
            }
            BaseIndex32::Sib(sib) => Uop::LeaSIB(rd, sib, memopr.disp),
        }
    }

    /// Generates a best effort load sequence for the given memory operand and destination register.
    pub fn generate_load32(rd: RtReg, memopr: MemOpr32) -> Uop {
        match memopr.segment {
            SrIndex::CS => match memopr.base_index {
                BaseIndex32::DispOnly => Uop::LoadBD32CS(rd, RtReg::Zero, memopr.disp),
                BaseIndex32::Base(base) => {
                    if memopr.disp.0 == 0 {
                        Uop::LoadR32CS(rd, base.into())
                    } else {
                        Uop::LoadBD32CS(rd, base.into(), memopr.disp)
                    }
                }
                BaseIndex32::Sib(sib) => Uop::LoadSIB32CS(rd, sib, memopr.disp),
            },
            _ => match memopr.base_index {
                BaseIndex32::DispOnly => Uop::LoadBD32(rd, RtReg::Zero, memopr.disp),
                BaseIndex32::Base(base) => {
                    if memopr.disp.0 == 0 {
                        Uop::LoadR32(rd, base.into())
                    } else {
                        Uop::LoadBD32(rd, base.into(), memopr.disp)
                    }
                }
                BaseIndex32::Sib(sib) => Uop::LoadSIB32(rd, sib, memopr.disp),
            },
        }
    }

    pub fn generate_store32(rd: RtReg, memopr: MemOpr32) -> Uop {
        match memopr.segment {
            SrIndex::CS => {
                todo!();
            }
            _ => match memopr.base_index {
                BaseIndex32::DispOnly => Uop::StoreBD32(rd, RtReg::Zero, memopr.disp),
                BaseIndex32::Base(base) => {
                    if memopr.disp.0 == 0 {
                        Uop::StoreR32(rd, base.into())
                    } else {
                        Uop::StoreBD32(rd, base.into(), memopr.disp)
                    }
                }
                BaseIndex32::Sib(sib) => Uop::StoreSIB32(rd, sib, memopr.disp),
            },
        }
    }

    /// Emits a read-modify-write sequence for a 32-bit memory operand, applying the provided closure `f` to the loaded value.
    pub fn emit_rmw32<F>(uop_cache: &mut Vec<Uop>, ma: MemOpr32, f: F)
    where
        F: FnOnce(&mut Vec<Uop>),
    {
        uop_cache.push(Self::generate_lea(RtReg::MemAddr, ma));
        uop_cache.push(Uop::LoadR32(RtReg::MemData, RtReg::MemAddr));
        f(uop_cache);
        uop_cache.push(Uop::StoreR32(RtReg::MemData, RtReg::MemAddr));
    }

    /// Emits a read-modify-write sequence for a 16-bit memory operand, applying the provided closure `f` to the loaded value.
    pub fn emit_rmw16<F>(uop_cache: &mut Vec<Uop>, ma: MemOpr32, f: F)
    where
        F: FnOnce(&mut Vec<Uop>),
    {
        uop_cache.push(Self::generate_lea(RtReg::MemAddr, ma));
        uop_cache.push(Uop::LoadR16(RtReg::MemData, RtReg::MemAddr));
        f(uop_cache);
        uop_cache.push(Uop::StoreR16(RtReg::MemData, RtReg::MemAddr));
    }

    /// Emits a read-modify-write sequence for an 8-bit memory operand, applying the provided closure `f` to the loaded value.
    pub fn emit_rmw8<F>(uop_cache: &mut Vec<Uop>, ma: MemOpr32, f: F)
    where
        F: FnOnce(&mut Vec<Uop>),
    {
        uop_cache.push(Self::generate_lea(RtReg::MemAddr, ma));
        uop_cache.push(Uop::LoadR8(RtReg8::MemData, RtReg::MemAddr));
        f(uop_cache);
        uop_cache.push(Uop::StoreR8(RtReg8::MemData, RtReg::MemAddr));
    }

    /// Fetches and decodes next chunk of instructions.
    pub fn fetch_and_decode(&mut self) {
        self._fetch_and_decode(self.max_step);
    }

    /// Fetches and decodes next chunk of instructions, up to `max_step` instructions.
    fn _fetch_and_decode(&mut self, mut max_step: usize) {
        let current_eip = self.eip_to_fetch;
        if self.address_map.get(&current_eip).is_some() {
            return;
        }

        let mut decoder = Decoder::with_use32();
        let mut fetcher = SimpleFetcher::new(&self.code, self.code_base);
        fetcher.set_pos((current_eip.0 - self.code_base.0) as usize);

        while max_step > 0 && fetcher.pos() < self.fetch_size {
            max_step -= 1;
            let source_eip = fetcher.current_eip();

            if let Some(target) = self.address_map.get(&source_eip) {
                self.uop_cache.push(Uop::Jump(*target));
                break;
            }

            let Ok(ir) = decoder.decode(&mut fetcher) else {
                break;
            };

            let addr_index = AddrIndex(self.uop_cache.len() as u32);
            self.address_map.insert(source_eip, addr_index);

            match ir {
                IrOp::ADD_MdA32_Id(ma, id) => {
                    Self::emit_rmw32(&mut self.uop_cache, ma, |uop_cache| {
                        uop_cache.push(Uop::AddI(RtReg::MemData, id))
                    });
                }
                IrOp::ADD_MdA32_Rd(ma, rs) => {
                    Self::emit_rmw32(&mut self.uop_cache, ma, |uop_cache| {
                        uop_cache.push(Uop::AddR(RtReg::MemData, rs.into()))
                    });
                }
                IrOp::ADD_Rd_Id(rd, id) => {
                    self.uop_cache.push(Uop::AddI(rd.into(), id));
                }
                IrOp::ADD_Rd_MdA32(rd, ma) => {
                    self.uop_cache
                        .push(Self::generate_load32(RtReg::MemData, ma));
                    self.uop_cache.push(Uop::AddR(rd.into(), RtReg::MemData));
                }
                IrOp::ADD_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::AddR(rd.into(), rs.into()));
                }
                IrOp::ADD_Rb_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::AddR8(rd.into(), rs.into()));
                }

                IrOp::AND_MdA32_Id(ma, id) => {
                    Self::emit_rmw32(&mut self.uop_cache, ma, |uop_cache| {
                        uop_cache.push(Uop::AndI(RtReg::MemData, id))
                    });
                }
                IrOp::AND_Rd_Id(rd, id) => {
                    self.uop_cache.push(Uop::AndI(rd.into(), id));
                }
                IrOp::AND_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::AndR(rd.into(), rs.into()));
                }
                IrOp::AND_Rb_Ib(rd, ib) => {
                    self.uop_cache.push(Uop::AndI8(rd.into(), ib));
                }

                IrOp::CALL_Jv(target) => {
                    let func_index = self.function_map.get(&target).copied().unwrap_or_else(|| {
                        let func_index = FuncIndex(self.functions.len() as u16);
                        let target2 = self
                            .address_map
                            .get(&target)
                            .copied()
                            .unwrap_or(AddrIndex(u32::MAX));
                        self.functions.push((target, target2));
                        self.function_map.insert(target, func_index);
                        func_index
                    });
                    self.uop_cache
                        .push(Uop::Call(func_index, fetcher.current_eip()));
                    break;
                }

                IrOp::CDQ => {
                    self.uop_cache.push(Uop::Minor(UopMinor::Cdq));
                }
                IrOp::CLD => {
                    self.uop_cache.push(Uop::Minor(UopMinor::Cld));
                }

                IrOp::CMP_MbA32_Ib(ma, ib) => {
                    self.uop_cache.push(Self::generate_lea(RtReg::MemAddr, ma));
                    self.uop_cache
                        .push(Uop::LoadR8(RtReg8::MemData, RtReg::MemAddr));
                    self.uop_cache.push(Uop::CmpI8(RtReg8::MemData, ib));
                }
                IrOp::CMP_MdA32_Id(ma, id) => {
                    self.uop_cache
                        .push(Self::generate_load32(RtReg::MemData, ma));
                    self.uop_cache.push(Uop::CmpI(RtReg::MemData, id));
                }
                IrOp::CMP_MdA32_Rd(ma, rs) => {
                    self.uop_cache
                        .push(Self::generate_load32(RtReg::MemData, ma));
                    self.uop_cache.push(Uop::CmpR(RtReg::MemData, rs.into()));
                }
                IrOp::CMP_Rb_Ib(rd, ib) => {
                    self.uop_cache.push(Uop::CmpI8(rd.into(), ib));
                }
                IrOp::CMP_Rd_Id(rd, id) => {
                    self.uop_cache.push(Uop::CmpI(rd.into(), id));
                }
                IrOp::CMP_Rd_MdA32(rd, ma) => {
                    self.uop_cache
                        .push(Self::generate_load32(RtReg::MemData, ma));
                    self.uop_cache.push(Uop::CmpR(rd.into(), RtReg::MemData));
                }
                IrOp::CMP_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::CmpR(rd.into(), rs.into()));
                }
                IrOp::CMP_MwA32_Iw(ma, iw) => {
                    self.uop_cache.push(Self::generate_lea(RtReg::MemAddr, ma));
                    self.uop_cache
                        .push(Uop::LoadR16(RtReg::MemData, RtReg::MemAddr));
                    self.uop_cache.push(Uop::CmpI16(RtReg::MemData, iw));
                }

                IrOp::CPUID => {
                    self.uop_cache.push(Uop::Minor(UopMinor::Cpuid));
                }
                IrOp::DEC_MdA32(ma) => {
                    Self::emit_rmw32(&mut self.uop_cache, ma, |uop_cache| {
                        uop_cache.push(Uop::DecR(RtReg::MemData))
                    });
                }
                IrOp::DEC_Rd(rd) => {
                    self.uop_cache.push(Uop::DecR(rd.into()));
                }

                IrOp::DIV_Rd(rd) => {
                    self.uop_cache.push(Uop::DivR(rd.into()));
                }
                IrOp::IDIV_MdA32(ma) => {
                    self.uop_cache
                        .push(Self::generate_load32(RtReg::MemData, ma));
                    self.uop_cache.push(Uop::IDivR(RtReg::MemData));
                }
                IrOp::IDIV_Rd(rd) => {
                    self.uop_cache.push(Uop::IDivR(rd.into()));
                }

                IrOp::IMUL_Rd_MdA32(rd, ma) => {
                    self.uop_cache
                        .push(Self::generate_load32(RtReg::MemData, ma));
                    self.uop_cache.push(Uop::IMulR(rd.into(), RtReg::MemData));
                }
                IrOp::IMUL_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::IMulR(rd.into(), rs.into()));
                }
                IrOp::IMUL_Rd_MdA32_Id(rd, ma, id) => {
                    self.uop_cache
                        .push(Self::generate_load32(RtReg::MemData, ma));
                    self.uop_cache
                        .push(Uop::IMulRI(rd.into(), RtReg::MemData, id as i32));
                }
                IrOp::IMUL_Rd_Rd_Id(rd, rs, id) => {
                    self.uop_cache
                        .push(Uop::IMulRI(rd.into(), rs.into(), id as i32));
                }

                IrOp::INC_MdA32(ma) => {
                    Self::emit_rmw32(&mut self.uop_cache, ma, |uop_cache| {
                        uop_cache.push(Uop::IncR(RtReg::MemData))
                    });
                }
                IrOp::INC_Rd(rd) => {
                    self.uop_cache.push(Uop::IncR(rd.into()));
                }
                IrOp::INT_Ib(ib) => {
                    self.uop_cache.push(Uop::Swi(ib));
                    break;
                }
                IrOp::JCC_Jv(cc, target) => {
                    if let Some(addr_index) = self.address_map.get(&target).copied() {
                        self.uop_cache.push(Uop::Jcc(cc, addr_index));
                    } else {
                        self.uop_cache.push(Uop::JccU(cc, target));
                    }
                    self.uop_cache.push(Uop::FetchNext(fetcher.current_eip()));
                    break;
                }
                IrOp::JMP_Jv(target) => {
                    if let Some(addr_index) = self.address_map.get(&target).copied() {
                        self.uop_cache.push(Uop::Jump(addr_index));
                        break;
                    } else {
                        fetcher.set_pos((target.0 - self.code_base.0) as usize);
                    }
                }
                IrOp::JMP_MdA32(ma) => {
                    self.uop_cache
                        .push(Self::generate_load32(RtReg::MemData, ma));
                    self.uop_cache.push(Uop::JumpR(RtReg::MemData));
                    break;
                }
                IrOp::LEA_Rd_MdA32(rd, ma) => {
                    self.uop_cache.push(Self::generate_lea(rd.into(), ma));
                }
                IrOp::LEAVE => {
                    self.uop_cache.push(Uop::Move(RtReg::ESP, RtReg::EBP));
                    self.uop_cache.push(Uop::PopR(RtReg::EBP));
                }

                IrOp::MOV_MbA32_Ib(ma, ib) => {
                    self.uop_cache.push(Uop::LoadConst8(RtReg8::MemData, ib));
                    self.uop_cache.push(Self::generate_lea(RtReg::MemAddr, ma));
                    self.uop_cache
                        .push(Uop::StoreR8(RtReg8::MemData, RtReg::MemAddr));
                }
                IrOp::MOV_MbA32_Rb(ma, rb) => {
                    self.uop_cache.push(Self::generate_lea(RtReg::MemAddr, ma));
                    self.uop_cache.push(Uop::StoreR8(rb.into(), RtReg::MemAddr));
                }

                IrOp::MOV_MdA32_Id(ma, id) => {
                    self.uop_cache.push(Uop::LoadConst(RtReg::MemData, id));
                    self.uop_cache
                        .push(Self::generate_store32(RtReg::MemData, ma));
                }
                IrOp::MOV_MdA32_Rd(ma, rd) => {
                    self.uop_cache.push(Self::generate_store32(rd.into(), ma));
                }
                IrOp::MOV_MwA32_Iw(ma, iw) => {
                    self.uop_cache.push(Uop::LoadConst16(RtReg::MemData, iw));
                    self.uop_cache.push(Self::generate_lea(RtReg::MemAddr, ma));
                    self.uop_cache
                        .push(Uop::StoreR16(RtReg::MemData, RtReg::MemAddr));
                }
                IrOp::MOV_MwA32_Rw(ma, rw) => {
                    self.uop_cache.push(Self::generate_lea(RtReg::MemAddr, ma));
                    self.uop_cache
                        .push(Uop::StoreR16(rw.upgrade().into(), RtReg::MemAddr));
                }

                IrOp::MOV_Rb_Ib(rd, ib) => {
                    self.uop_cache.push(Uop::LoadConst8(rd.into(), ib));
                }
                IrOp::MOV_Rb_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::Move8(rd.into(), rs.into()));
                }
                IrOp::MOV_Rb_MbA32(rd, ma) => {
                    self.uop_cache.push(Self::generate_lea(RtReg::MemAddr, ma));
                    self.uop_cache.push(Uop::LoadR8(rd.into(), RtReg::MemAddr));
                }

                IrOp::MOV_Rw_MwA32(rd, ma) => {
                    self.uop_cache.push(Self::generate_lea(RtReg::MemAddr, ma));
                    self.uop_cache
                        .push(Uop::LoadR16(rd.upgrade().into(), RtReg::MemAddr));
                }

                IrOp::MOV_Rd_Id(rd, id) => {
                    self.uop_cache.push(Uop::LoadConst(rd.into(), id));
                }
                IrOp::MOV_Rd_MdA32(rd, memopr) => {
                    self.uop_cache
                        .push(Self::generate_load32(rd.into(), memopr));
                }
                IrOp::MOV_Rd_Rd(rd, rs) => {
                    if rd != rs {
                        self.uop_cache.push(Uop::Move(rd.into(), rs.into()));
                    }
                }

                IrOp::MOVSX_Rd_MbA32(rd, ma) => {
                    self.uop_cache.push(Self::generate_lea(RtReg::MemAddr, ma));
                    self.uop_cache
                        .push(Uop::LoadR8(RtReg8::MemData, RtReg::MemAddr));
                    self.uop_cache.push(Uop::MovSx8(rd.into(), RtReg8::MemData));
                }
                IrOp::MOVSX_Rd_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::MovSx8(rd.into(), rs.into()));
                }
                IrOp::MOVZX_Rd_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::MovZx8(rd.into(), rs.into()));
                }
                IrOp::MOVZX_Rd_MbA32(rd, rs) => {
                    self.uop_cache.push(Self::generate_lea(RtReg::MemAddr, rs));
                    self.uop_cache
                        .push(Uop::LoadR8(RtReg8::MemData, RtReg::MemAddr));
                    self.uop_cache.push(Uop::MovZx8(rd.into(), RtReg8::MemData));
                }
                IrOp::MOVZX_Rd_MwA32(rd, ma) => {
                    self.uop_cache.push(Self::generate_lea(RtReg::MemAddr, ma));
                    self.uop_cache
                        .push(Uop::LoadR16(RtReg::MemData, RtReg::MemAddr));
                    self.uop_cache.push(Uop::MovZx16(rd.into(), RtReg::MemData));
                }
                IrOp::MOVZX_Rd_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::MovZx16(rd.into(), rs.upgrade().into()));
                }

                IrOp::MUL_Rd(rd) => {
                    self.uop_cache.push(Uop::MulR(rd.into()));
                }

                IrOp::NEG_MdA32(ma) => {
                    Self::emit_rmw32(&mut self.uop_cache, ma, |uop_cache| {
                        uop_cache.push(Uop::NegR(RtReg::MemData))
                    });
                }
                IrOp::NEG_Rd(rd) => self.uop_cache.push(Uop::NegR(rd.into())),

                IrOp::NOT_Rd(rd) => {
                    self.uop_cache.push(Uop::NotR(rd.into()));
                }

                IrOp::OR_MdA32_Rd(ma, rs) => {
                    Self::emit_rmw32(&mut self.uop_cache, ma, |uop_cache| {
                        uop_cache.push(Uop::OrR(RtReg::MemData, rs.into()));
                    });
                }
                IrOp::OR_Rd_Id(rd, id) => {
                    self.uop_cache.push(Uop::OrI(rd.into(), id));
                }
                IrOp::OR_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::OrR(rd.into(), rs.into()));
                }
                IrOp::POPAD => {
                    self.uop_cache.push(Uop::PopAd);
                }
                IrOp::POP_Rd(rd) => {
                    self.uop_cache.push(Uop::PopR(rd.into()));
                }
                IrOp::PUSHAD => {
                    self.uop_cache.push(Uop::PushAd);
                }
                IrOp::PUSH_MdA32(ma) => {
                    self.uop_cache
                        .push(Self::generate_load32(RtReg::MemData, ma));
                    self.uop_cache.push(Uop::PushR(RtReg::MemData));
                }
                IrOp::PUSH_Id(id) => {
                    self.uop_cache.push(Uop::PushI(id));
                }
                IrOp::PUSH_Rd(rd) => {
                    self.uop_cache.push(Uop::PushR(rd.into()));
                }
                IrOp::RDTSC => {
                    self.uop_cache.push(Uop::Minor(UopMinor::RdTsc));
                }
                IrOp::REPNZ_SCASB(_sr, _a32) => {
                    self.uop_cache.push(Uop::Minor(UopMinor::RepNzScasb));
                }
                IrOp::RET_D32(iw) => {
                    self.uop_cache.push(Uop::Ret(iw));
                    break;
                }

                IrOp::SAR_MdA32_Ib(ma, ib) => {
                    Self::emit_rmw32(&mut self.uop_cache, ma, |uop_cache| {
                        uop_cache.push(Uop::SarI(RtReg::MemData, ib))
                    });
                }
                IrOp::SAR_Rd_Cl(rd) => {
                    self.uop_cache.push(Uop::SarRCl(rd.into()));
                }
                IrOp::SAR_Rd_Ib(rd, ib) => {
                    self.uop_cache.push(Uop::SarI(rd.into(), ib));
                }

                IrOp::SHL_MdA32_Cl(ma) => {
                    Self::emit_rmw32(&mut self.uop_cache, ma, |uop_cache| {
                        uop_cache.push(Uop::ShlRCl(RtReg::MemData))
                    });
                }
                IrOp::SHL_MdA32_Ib(ma, ib) => {
                    Self::emit_rmw32(&mut self.uop_cache, ma, |uop_cache| {
                        uop_cache.push(Uop::ShlI(RtReg::MemData, ib))
                    });
                }
                IrOp::SHL_Rd_Cl(rd) => {
                    self.uop_cache.push(Uop::ShlRCl(rd.into()));
                }
                IrOp::SHL_Rd_Ib(rd, ib) => {
                    self.uop_cache.push(Uop::ShlI(rd.into(), ib));
                }
                IrOp::SHL_Rw_Cl(rd) => {
                    self.uop_cache.push(Uop::ShlRCl16(rd.upgrade().into()));
                }
                IrOp::SHL_Rw_Ib(rd, ib) => {
                    self.uop_cache.push(Uop::ShlI16(rd.upgrade().into(), ib));
                }
                IrOp::SHL_Rb_Cl(rd) => {
                    self.uop_cache.push(Uop::ShlRCl8(rd.into()));
                }
                IrOp::SHL_Rb_Ib(rd, ib) => {
                    self.uop_cache.push(Uop::ShlI8(rd.into(), ib));
                }

                IrOp::SHR_Rb_Cl(rd) => {
                    self.uop_cache.push(Uop::ShrRCl8(rd.into()));
                }
                IrOp::SHR_Rb_Ib(rd, ib) => {
                    self.uop_cache.push(Uop::ShrI8(rd.into(), ib));
                }
                IrOp::SHR_Rd_Cl(rd) => {
                    self.uop_cache.push(Uop::ShrRCl(rd.into()));
                }
                IrOp::SHR_Rd_Ib(rd, ib) => {
                    self.uop_cache.push(Uop::ShrI(rd.into(), ib));
                }

                IrOp::SUB_MdA32_Id(ma, id) => {
                    Self::emit_rmw32(&mut self.uop_cache, ma, |uop_cache| {
                        uop_cache.push(Uop::SubI(RtReg::MemData, id))
                    });
                }
                IrOp::SUB_MdA32_Rd(ma, rs) => {
                    Self::emit_rmw32(&mut self.uop_cache, ma, |uop_cache| {
                        uop_cache.push(Uop::SubR(RtReg::MemData, rs.into()))
                    });
                }
                IrOp::SUB_Rd_Id(rd, id) => {
                    self.uop_cache.push(Uop::SubI(rd.into(), id));
                }
                IrOp::SUB_Rd_MdA32(rd, ma) => {
                    self.uop_cache
                        .push(Self::generate_load32(RtReg::MemData, ma));
                    self.uop_cache.push(Uop::SubR(rd.into(), RtReg::MemData));
                }
                IrOp::SUB_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::SubR(rd.into(), rs.into()));
                }
                IrOp::TEST_MbA32_Ib(ma, ib) => {
                    self.uop_cache.push(Self::generate_lea(RtReg::MemAddr, ma));
                    self.uop_cache
                        .push(Uop::LoadR8(RtReg8::MemData, RtReg::MemAddr));
                    self.uop_cache.push(Uop::TestI8(RtReg8::MemData, ib));
                }
                IrOp::TEST_MdA32_Id(ma, id) => {
                    self.uop_cache
                        .push(Self::generate_load32(RtReg::MemData, ma));
                    self.uop_cache.push(Uop::TestI(RtReg::MemData, id));
                }
                IrOp::TEST_Rb_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::TestR8(rd.into(), rs.into()));
                }
                IrOp::TEST_Rd_Id(rd, id) => {
                    self.uop_cache.push(Uop::TestI(rd.into(), id));
                }
                IrOp::TEST_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::TestR(rd.into(), rs.into()));
                }
                IrOp::XCHG_Rd_Rd(rd, rs) => {
                    if rd != rs {
                        self.uop_cache.push(Uop::XchgR(rd.into(), rs.into()));
                    }
                }
                IrOp::XOR_Rd_Id(rd, id) => {
                    self.uop_cache.push(Uop::XorI(rd.into(), id));
                }
                IrOp::XOR_Rd_MdA32(rd, ma) => {
                    self.uop_cache
                        .push(Self::generate_load32(RtReg::MemData, ma));
                    self.uop_cache.push(Uop::XorR(rd.into(), RtReg::MemData));
                }
                IrOp::XOR_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::XorR(rd.into(), rs.into()));
                }

                //
                IrOp::SBB_Rd_Rd(_a1, _a2) => todo!(),

                //
                _ => {
                    todo!("{:08x}: {:?}", source_eip.0, ir);
                }
            }
        }
        self.eip_to_fetch = fetcher.current_eip();
    }
}
