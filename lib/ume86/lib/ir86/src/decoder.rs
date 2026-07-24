//! An x86 instruction decoder
use crate::_prelude_::*;

/// An x86 instruction decoder
pub struct Decoder {
    use32: bool,
}

struct DecoderContext {
    segment_override: Option<SrIndex>,
    is_opsize32: bool,
    is_addr32: bool,
    prefix_fx: PrefixFx,
}

impl Decoder {
    /// Creates a new instance of the decoder.
    #[inline]
    pub const fn new(use32: bool) -> Self {
        Self { use32 }
    }

    /// Creates a new instance of the decoder that decodes instructions in 32-bit mode by default.
    #[inline]
    pub const fn with_use32() -> Self {
        Self::new(true)
    }

    /// Creates a new instance of the decoder that decodes instructions in 16-bit mode by default.
    #[inline]
    pub const fn with_use16() -> Self {
        Self::new(false)
    }

    /// Sets whether the decoder should decode instructions in 32-bit mode by default.
    #[inline]
    pub fn set_use32(&mut self, use32: bool) {
        self.use32 = use32;
    }

    /// Decodes an instruction from the instruction stream provided by the fetcher.
    pub fn decode<FETCH: Fetch<E = E>, E>(&mut self, is: &mut FETCH) -> Result<IrOp, E> {
        let mut context = DecoderContext::new();
        let opcode = loop {
            let opcode = is.fetch_u8()?;
            match opcode {
                0x2e => {
                    // CS segment override prefix
                    context.segment_override = Some(CS);
                }
                0x36 => {
                    // SS segment override prefix
                    context.segment_override = Some(SS);
                }
                0x3e => {
                    // DS segment override prefix
                    context.segment_override = Some(DS);
                }
                0x26 => {
                    // ES segment override prefix
                    context.segment_override = Some(ES);
                }
                0x64 => {
                    // FS segment override prefix
                    context.segment_override = Some(FS);
                }
                0x65 => {
                    // GS segment override prefix
                    context.segment_override = Some(GS);
                }
                0x66 => {
                    // Operand-size override prefix
                    context.is_opsize32 = true;
                }
                0x67 => {
                    // Address-size override prefix
                    context.is_addr32 = true;
                }
                0xf0 => {
                    // LOCK prefix
                    context.prefix_fx = PrefixFx::F0_LOCK;
                }
                0xf2 => {
                    // REPNE/REPNZ prefix
                    context.prefix_fx = PrefixFx::F2_REPNZ;
                }
                0xf3 => {
                    // REP/REPE/REPZ prefix
                    context.prefix_fx = PrefixFx::F3_REPZ;
                }
                _ => {
                    break opcode;
                }
            }
        };

        // invert if use32 is true, since the default is 16-bit mode
        context.is_opsize32 ^= self.use32;
        context.is_addr32 ^= self.use32;

        match opcode {
            0x00 => {
                // ADD r/m8, reg8
                context.modrm_rb(is, IrOp::ADD_Rb_Rb, IrOp::ADD_MbA16_Rb, IrOp::ADD_MbA32_Rb)
            }
            0x01 => {
                // ADD r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::ADD_Rw_Rw,
                    IrOp::ADD_MwA16_Rw,
                    IrOp::ADD_MwA32_Rw,
                    IrOp::ADD_Rd_Rd,
                    IrOp::ADD_MdA16_Rd,
                    IrOp::ADD_MdA32_Rd,
                )
            }
            0x02 => {
                // ADD reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::ADD_Rb_Rb(r, m),
                    |m, r| IrOp::ADD_Rb_MbA16(r, m),
                    |m, r| IrOp::ADD_Rb_MbA32(r, m),
                )
            }
            0x03 => {
                // ADD reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::ADD_Rw_Rw(r, m),
                    |m, r| IrOp::ADD_Rw_MwA16(r, m),
                    |m, r| IrOp::ADD_Rw_MwA32(r, m),
                    |m, r| IrOp::ADD_Rd_Rd(r, m),
                    |m, r| IrOp::ADD_Rd_MdA16(r, m),
                    |m, r| IrOp::ADD_Rd_MdA32(r, m),
                )
            }
            0x04 => {
                // ADD AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::ADD_Rb_Ib(GprIndex8::AL, imm))
            }
            0x05 => {
                if context.is_opsize32 {
                    // ADD EAX, imm32
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::ADD_Rd_Id(EAX, imm))
                } else {
                    // ADD AX, imm16/32
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::ADD_Rw_Iw(AX, imm))
                }
            }
            0x06 => {
                // PUSH ES
                Ok(IrOp::PUSH_Sr(ES))
            }
            0x07 => {
                // POP ES
                Ok(IrOp::POP_Sr(ES))
            }
            0x08 => {
                // OR r/m8, reg8
                context.modrm_rb(is, IrOp::OR_Rb_Rb, IrOp::OR_MbA16_Rb, IrOp::OR_MbA32_Rb)
            }
            0x09 => {
                // OR r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::OR_Rw_Rw,
                    IrOp::OR_MwA16_Rw,
                    IrOp::OR_MwA32_Rw,
                    IrOp::OR_Rd_Rd,
                    IrOp::OR_MdA16_Rd,
                    IrOp::OR_MdA32_Rd,
                )
            }
            0x0a => {
                // OR reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::OR_Rb_Rb(r, m),
                    |m, r| IrOp::OR_Rb_MbA16(r, m),
                    |m, r| IrOp::OR_Rb_MbA32(r, m),
                )
            }
            0x0b => {
                // OR reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::OR_Rw_Rw(r, m),
                    |m, r| IrOp::OR_Rw_MwA16(r, m),
                    |m, r| IrOp::OR_Rw_MwA32(r, m),
                    |m, r| IrOp::OR_Rd_Rd(r, m),
                    |m, r| IrOp::OR_Rd_MdA16(r, m),
                    |m, r| IrOp::OR_Rd_MdA32(r, m),
                )
            }
            0x0c => {
                // OR AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::OR_Rb_Ib(GprIndex8::AL, imm))
            }
            0x0d => {
                if context.is_opsize32 {
                    // OR EAX, imm32
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::OR_Rd_Id(EAX, imm))
                } else {
                    // OR AX, imm16/32
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::OR_Rw_Iw(AX, imm))
                }
            }
            0x0e => {
                // PUSH CS
                Ok(IrOp::PUSH_Sr(CS))
            }

            0x10 => {
                // ADC r/m8, reg8
                context.modrm_rb(is, IrOp::ADC_Rb_Rb, IrOp::ADC_MbA16_Rb, IrOp::ADC_MbA32_Rb)
            }
            0x11 => {
                // ADC r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::ADC_Rw_Rw,
                    IrOp::ADC_MwA16_Rw,
                    IrOp::ADC_MwA32_Rw,
                    IrOp::ADC_Rd_Rd,
                    IrOp::ADC_MdA16_Rd,
                    IrOp::ADC_MdA32_Rd,
                )
            }
            0x12 => {
                // ADC reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::ADC_Rb_Rb(r, m),
                    |m, r| IrOp::ADC_Rb_MbA16(r, m),
                    |m, r| IrOp::ADC_Rb_MbA32(r, m),
                )
            }
            0x13 => {
                // ADC reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::ADC_Rw_Rw(r, m),
                    |m, r| IrOp::ADC_Rw_MwA16(r, m),
                    |m, r| IrOp::ADC_Rw_MwA32(r, m),
                    |m, r| IrOp::ADC_Rd_Rd(r, m),
                    |m, r| IrOp::ADC_Rd_MdA16(r, m),
                    |m, r| IrOp::ADC_Rd_MdA32(r, m),
                )
            }
            0x14 => {
                // ADC AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::ADC_Rb_Ib(GprIndex8::AL, imm))
            }
            0x15 => {
                if context.is_opsize32 {
                    // ADC EAX, imm32
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::ADC_Rd_Id(EAX, imm))
                } else {
                    // ADC AX, imm16/32
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::ADC_Rw_Iw(AX, imm))
                }
            }
            0x16 => {
                // PUSH SS
                Ok(IrOp::PUSH_Sr(SS))
            }
            0x17 => {
                // POP SS
                Ok(IrOp::POP_Sr(SS))
            }
            0x18 => {
                // SBB r/m8, reg8
                context.modrm_rb(is, IrOp::SBB_Rb_Rb, IrOp::SBB_MbA16_Rb, IrOp::SBB_MbA32_Rb)
            }
            0x19 => {
                // SBB r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::SBB_Rw_Rw,
                    IrOp::SBB_MwA16_Rw,
                    IrOp::SBB_MwA32_Rw,
                    IrOp::SBB_Rd_Rd,
                    IrOp::SBB_MdA16_Rd,
                    IrOp::SBB_MdA32_Rd,
                )
            }
            0x1a => {
                // SBB reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::SBB_Rb_Rb(r, m),
                    |m, r| IrOp::SBB_Rb_MbA16(r, m),
                    |m, r| IrOp::SBB_Rb_MbA32(r, m),
                )
            }
            0x1b => {
                // SBB reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::SBB_Rw_Rw(r, m),
                    |m, r| IrOp::SBB_Rw_MwA16(r, m),
                    |m, r| IrOp::SBB_Rw_MwA32(r, m),
                    |m, r| IrOp::SBB_Rd_Rd(r, m),
                    |m, r| IrOp::SBB_Rd_MdA16(r, m),
                    |m, r| IrOp::SBB_Rd_MdA32(r, m),
                )
            }
            0x1c => {
                // SBB AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::SBB_Rb_Ib(GprIndex8::AL, imm))
            }
            0x1d => {
                if context.is_opsize32 {
                    // SBB EAX, imm32
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::SBB_Rd_Id(EAX, imm))
                } else {
                    // SBB AX, imm16/32
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::SBB_Rw_Iw(AX, imm))
                }
            }
            0x1e => {
                // PUSH DS
                Ok(IrOp::PUSH_Sr(DS))
            }
            0x1f => {
                // POP DS
                Ok(IrOp::POP_Sr(DS))
            }

            0x20 => {
                // AND r/m8, reg8
                context.modrm_rb(is, IrOp::AND_Rb_Rb, IrOp::AND_MbA16_Rb, IrOp::AND_MbA32_Rb)
            }
            0x21 => {
                // AND r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::AND_Rw_Rw,
                    IrOp::AND_MwA16_Rw,
                    IrOp::AND_MwA32_Rw,
                    IrOp::AND_Rd_Rd,
                    IrOp::AND_MdA16_Rd,
                    IrOp::AND_MdA32_Rd,
                )
            }
            0x22 => {
                // AND reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::AND_Rb_Rb(r, m),
                    |m, r| IrOp::AND_Rb_MbA16(r, m),
                    |m, r| IrOp::AND_Rb_MbA32(r, m),
                )
            }
            0x23 => {
                // AND reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::AND_Rw_Rw(r, m),
                    |m, r| IrOp::AND_Rw_MwA16(r, m),
                    |m, r| IrOp::AND_Rw_MwA32(r, m),
                    |m, r| IrOp::AND_Rd_Rd(r, m),
                    |m, r| IrOp::AND_Rd_MdA16(r, m),
                    |m, r| IrOp::AND_Rd_MdA32(r, m),
                )
            }
            0x24 => {
                // AND AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::AND_Rb_Ib(GprIndex8::AL, imm))
            }
            0x25 => {
                if context.is_opsize32 {
                    // AND EAX, imm32
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::AND_Rd_Id(EAX, imm))
                } else {
                    // AND AX, imm16/32
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::AND_Rw_Iw(AX, imm))
                }
            }
            // 0x26 es:
            0x27 => {
                // DAA
                Ok(IrOp::DAA)
            }
            0x28 => {
                // SUB r/m8, reg8
                context.modrm_rb(is, IrOp::SUB_Rb_Rb, IrOp::SUB_MbA16_Rb, IrOp::SUB_MbA32_Rb)
            }
            0x29 => {
                // SUB r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::SUB_Rw_Rw,
                    IrOp::SUB_MwA16_Rw,
                    IrOp::SUB_MwA32_Rw,
                    IrOp::SUB_Rd_Rd,
                    IrOp::SUB_MdA16_Rd,
                    IrOp::SUB_MdA32_Rd,
                )
            }
            0x2a => {
                // SUB reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::SUB_Rb_Rb(r, m),
                    |m, r| IrOp::SUB_Rb_MbA16(r, m),
                    |m, r| IrOp::SUB_Rb_MbA32(r, m),
                )
            }
            0x2b => {
                // SUB reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::SUB_Rw_Rw(r, m),
                    |m, r| IrOp::SUB_Rw_MwA16(r, m),
                    |m, r| IrOp::SUB_Rw_MwA32(r, m),
                    |m, r| IrOp::SUB_Rd_Rd(r, m),
                    |m, r| IrOp::SUB_Rd_MdA16(r, m),
                    |m, r| IrOp::SUB_Rd_MdA32(r, m),
                )
            }
            0x2c => {
                // SUB AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::SUB_Rb_Ib(GprIndex8::AL, imm))
            }
            0x2d => {
                if context.is_opsize32 {
                    // SUB EAX, imm32
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::SUB_Rd_Id(EAX, imm))
                } else {
                    // SUB AX, imm16/32
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::SUB_Rw_Iw(AX, imm))
                }
            }
            // 0x2e cs:
            0x2f => {
                // DAS
                Ok(IrOp::DAS)
            }

            0x30 => {
                // XOR r/m8, reg8
                context.modrm_rb(is, IrOp::XOR_Rb_Rb, IrOp::XOR_MbA16_Rb, IrOp::XOR_MbA32_Rb)
            }
            0x31 => {
                // XOR r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::XOR_Rw_Rw,
                    IrOp::XOR_MwA16_Rw,
                    IrOp::XOR_MwA32_Rw,
                    IrOp::XOR_Rd_Rd,
                    IrOp::XOR_MdA16_Rd,
                    IrOp::XOR_MdA32_Rd,
                )
            }
            0x32 => {
                // XOR reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::XOR_Rb_Rb(r, m),
                    |m, r| IrOp::XOR_Rb_MbA16(r, m),
                    |m, r| IrOp::XOR_Rb_MbA32(r, m),
                )
            }
            0x33 => {
                // XOR reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::XOR_Rw_Rw(r, m),
                    |m, r| IrOp::XOR_Rw_MwA16(r, m),
                    |m, r| IrOp::XOR_Rw_MwA32(r, m),
                    |m, r| IrOp::XOR_Rd_Rd(r, m),
                    |m, r| IrOp::XOR_Rd_MdA16(r, m),
                    |m, r| IrOp::XOR_Rd_MdA32(r, m),
                )
            }
            0x34 => {
                // XOR AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::XOR_Rb_Ib(GprIndex8::AL, imm))
            }
            0x35 => {
                if context.is_opsize32 {
                    // XOR EAX, imm32
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::XOR_Rd_Id(EAX, imm))
                } else {
                    // XOR AX, imm16/32
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::XOR_Rw_Iw(AX, imm))
                }
            }
            // 0x36 ss:
            0x37 => {
                // AAA
                Ok(IrOp::AAA)
            }

            0x38 => {
                // CMP r/m8, reg8
                context.modrm_rb(is, IrOp::CMP_Rb_Rb, IrOp::CMP_MbA16_Rb, IrOp::CMP_MbA32_Rb)
            }
            0x39 => {
                // CMP r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::CMP_Rw_Rw,
                    IrOp::CMP_MwA16_Rw,
                    IrOp::CMP_MwA32_Rw,
                    IrOp::CMP_Rd_Rd,
                    IrOp::CMP_MdA16_Rd,
                    IrOp::CMP_MdA32_Rd,
                )
            }
            0x3a => {
                // CMP reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::CMP_Rb_Rb(r, m),
                    |m, r| IrOp::CMP_Rb_MbA16(r, m),
                    |m, r| IrOp::CMP_Rb_MbA32(r, m),
                )
            }
            0x3b => {
                // CMP reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::CMP_Rw_Rw(r, m),
                    |m, r| IrOp::CMP_Rw_MwA16(r, m),
                    |m, r| IrOp::CMP_Rw_MwA32(r, m),
                    |m, r| IrOp::CMP_Rd_Rd(r, m),
                    |m, r| IrOp::CMP_Rd_MdA16(r, m),
                    |m, r| IrOp::CMP_Rd_MdA32(r, m),
                )
            }
            0x3c => {
                // CMP AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::CMP_Rb_Ib(GprIndex8::AL, imm))
            }
            0x3d => {
                if context.is_opsize32 {
                    // CMP EAX, imm32
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::CMP_Rd_Id(EAX, imm))
                } else {
                    // CMP AX, imm16/32
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::CMP_Rw_Iw(AX, imm))
                }
            }
            // 0x3e ds:
            0x3f => {
                // AAS
                Ok(IrOp::AAS)
            }

            0x40..=0x47 => {
                // INC reg16/32
                let reg_index = GprIndex32::from_u8(opcode);
                if context.is_opsize32 {
                    // INC reg32
                    Ok(IrOp::INC_Rd(reg_index))
                } else {
                    // INC reg16
                    Ok(IrOp::INC_Rw(reg_index.downgrade()))
                }
            }
            0x48..=0x4f => {
                // DEC reg16/32
                let reg_index = GprIndex32::from_u8(opcode);
                if context.is_opsize32 {
                    // DEC reg32
                    Ok(IrOp::DEC_Rd(reg_index))
                } else {
                    // DEC reg16
                    Ok(IrOp::DEC_Rw(reg_index.downgrade()))
                }
            }
            0x50..=0x57 => {
                // PUSH reg16/32
                let reg_index = GprIndex32::from_u8(opcode);
                if context.is_opsize32 {
                    // PUSH reg32
                    Ok(IrOp::PUSH_Rd(reg_index))
                } else {
                    // PUSH reg16
                    Ok(IrOp::PUSH_Rw(reg_index.downgrade()))
                }
            }
            0x58..=0x5f => {
                // POP reg16/32
                let reg_index = GprIndex32::from_u8(opcode);
                if context.is_opsize32 {
                    // POP reg32
                    Ok(IrOp::POP_Rd(reg_index))
                } else {
                    // POP reg16
                    Ok(IrOp::POP_Rw(reg_index.downgrade()))
                }
            }

            0x60 => {
                // PUSHA/PUSHAD
                if context.is_opsize32 {
                    Ok(IrOp::PUSHAD)
                } else {
                    Ok(IrOp::PUSHA)
                }
            }
            0x61 => {
                // POPA/POPAD
                if context.is_opsize32 {
                    Ok(IrOp::POPAD)
                } else {
                    Ok(IrOp::POPA)
                }
            }
            0x62 => {
                // BOUND r16/32, m16/32
                context.modrm_mv(
                    is,
                    |_| IrOp::UD,
                    |m, r| IrOp::BOUND_Rw_MwA16(r, m),
                    |m, r| IrOp::BOUND_Rw_MwA32(r, m),
                    |m, r| IrOp::BOUND_Rd_MdA16(r, m),
                    |m, r| IrOp::BOUND_Rd_MdA32(r, m),
                )
            }
            0x63 => {
                // ARPL r16/32, r/m16/32
                context.modrm_rw(
                    is,
                    IrOp::ARPL_Rw_Rw,
                    IrOp::ARPL_MwA16_Rw,
                    IrOp::ARPL_MwA32_Rw,
                )
            }
            // 0x64 fs:
            // 0x65 gs:
            // 0x66 operand-size override prefix
            // 0x67 address-size override prefix
            0x68 => {
                // PUSH imm16/32
                if context.is_opsize32 {
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::PUSH_Id(imm))
                } else {
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::PUSH_Iw(imm))
                }
            }
            0x69 => {
                // IMUL r16/32, r/m16/32, imm16/32
                context.modrm_rv_sub3(
                    is,
                    |is, s, c| {
                        let imm = is.fetch_u16()?;
                        Ok(IrOp::IMUL_Rw_Rw_Iw(GprIndex16::from_idx3(c), s, imm))
                    },
                    |is, s, c| {
                        let imm = is.fetch_u16()?;
                        Ok(IrOp::IMUL_Rw_MwA16_Iw(GprIndex16::from_idx3(c), s, imm))
                    },
                    |is, s, c| {
                        let imm = is.fetch_u16()?;
                        Ok(IrOp::IMUL_Rw_MwA32_Iw(GprIndex16::from_idx3(c), s, imm))
                    },
                    |is, s, c| {
                        let imm = is.fetch_u32()?;
                        Ok(IrOp::IMUL_Rd_Rd_Id(GprIndex32::from_idx3(c), s, imm))
                    },
                    |is, s, c| {
                        let imm = is.fetch_u32()?;
                        Ok(IrOp::IMUL_Rd_MdA16_Id(GprIndex32::from_idx3(c), s, imm))
                    },
                    |is, s, c| {
                        let imm = is.fetch_u32()?;
                        Ok(IrOp::IMUL_Rd_MdA32_Id(GprIndex32::from_idx3(c), s, imm))
                    },
                )
            }
            0x6a => {
                // PUSH imm8
                let imm = is.fetch_i8()? as i32;
                if context.is_opsize32 {
                    Ok(IrOp::PUSH_Id(imm as u32))
                } else {
                    Ok(IrOp::PUSH_Iw(imm as u16))
                }
            }
            0x6b => {
                // IMUL r16/32, r/m16/32, imm8
                context.modrm_rv_ib(
                    is,
                    |m, r, imm| IrOp::IMUL_Rw_Rw_Iw(r, m, imm as u16),
                    |m, r, imm| IrOp::IMUL_Rw_MwA16_Iw(r, m, imm as u16),
                    |m, r, imm| IrOp::IMUL_Rw_MwA32_Iw(r, m, imm as u16),
                    |m, r, imm| IrOp::IMUL_Rd_Rd_Id(r, m, imm as u32),
                    |m, r, imm| IrOp::IMUL_Rd_MdA16_Id(r, m, imm as u32),
                    |m, r, imm| IrOp::IMUL_Rd_MdA32_Id(r, m, imm as u32),
                )
            }
            0x6c => {
                // INS m8, DX
                match context.prefix_fx {
                    PrefixFx::NP => Ok(IrOp::INSB(ES, context.is_addr32)),
                    PrefixFx::F2_REPNZ | PrefixFx::F3_REPZ => {
                        Ok(IrOp::REP_INSB(ES, context.is_addr32))
                    }
                    PrefixFx::F0_LOCK => Ok(IrOp::UD),
                }
            }
            0x6d => {
                // INS m16/32, DX
                if context.is_opsize32 {
                    match context.prefix_fx {
                        PrefixFx::NP => Ok(IrOp::INSD(ES, context.is_addr32)),
                        PrefixFx::F2_REPNZ | PrefixFx::F3_REPZ => {
                            Ok(IrOp::REP_INSD(ES, context.is_addr32))
                        }
                        PrefixFx::F0_LOCK => Ok(IrOp::UD),
                    }
                } else {
                    match context.prefix_fx {
                        PrefixFx::NP => Ok(IrOp::INSW(ES, context.is_addr32)),
                        PrefixFx::F2_REPNZ | PrefixFx::F3_REPZ => {
                            Ok(IrOp::REP_INSW(ES, context.is_addr32))
                        }
                        PrefixFx::F0_LOCK => Ok(IrOp::UD),
                    }
                }
            }
            0x6e => {
                // OUTS DX, m8
                match context.prefix_fx {
                    PrefixFx::NP => Ok(IrOp::OUTSB(
                        context.segment_override.unwrap_or(DS),
                        context.is_addr32,
                    )),
                    PrefixFx::F2_REPNZ | PrefixFx::F3_REPZ => Ok(IrOp::REP_OUTSB(
                        context.segment_override.unwrap_or(DS),
                        context.is_addr32,
                    )),
                    PrefixFx::F0_LOCK => Ok(IrOp::UD),
                }
            }
            0x6f => {
                // OUTS DX, m16/32
                if context.is_opsize32 {
                    match context.prefix_fx {
                        PrefixFx::NP => Ok(IrOp::OUTSD(
                            context.segment_override.unwrap_or(DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F2_REPNZ | PrefixFx::F3_REPZ => Ok(IrOp::REP_OUTSD(
                            context.segment_override.unwrap_or(DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F0_LOCK => Ok(IrOp::UD),
                    }
                } else {
                    match context.prefix_fx {
                        PrefixFx::NP => Ok(IrOp::OUTSW(
                            context.segment_override.unwrap_or(DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F2_REPNZ | PrefixFx::F3_REPZ => Ok(IrOp::REP_OUTSW(
                            context.segment_override.unwrap_or(DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F0_LOCK => Ok(IrOp::UD),
                    }
                }
            }

            0x70..=0x7f => {
                // Jcc rel8
                let rel = is.fetch_i8()? as i32;
                let cc = CC::from_u8(opcode);
                let target = if context.is_opsize32 {
                    Offset32(is.current_eip().0.wrapping_add(rel as u32))
                } else {
                    Offset32(is.current_eip().0.wrapping_add(rel as u32) & 0x0000_ffff)
                };
                Ok(IrOp::JCC_Jv(cc, target))
            }

            0x80 | 0x82 => {
                // Grp1 r/m8, imm8
                // NOTE: 0x82 is strictly undefined, but treated the same as 0x80 by most CPUs.
                context.modrm_rb_sub3(
                    is,
                    move |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_u8()?;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_Rb_Ib(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_Rb_Ib(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_Rb_Ib(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_Rb_Ib(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_Rb_Ib(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_Rb_Ib(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_Rb_Ib(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_Rb_Ib(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_u8()?;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_MbA16_Ib(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_MbA16_Ib(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_MbA16_Ib(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_MbA16_Ib(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_MbA16_Ib(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_MbA16_Ib(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_MbA16_Ib(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_MbA16_Ib(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_u8()?;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_MbA32_Ib(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_MbA32_Ib(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_MbA32_Ib(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_MbA32_Ib(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_MbA32_Ib(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_MbA32_Ib(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_MbA32_Ib(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_MbA32_Ib(s, imm)),
                        }
                    },
                )
            }
            0x81 => {
                // Grp1 r/m16/32, imm16/32
                context.modrm_rv_sub3(
                    is,
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_u16()?;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_Rw_Iw(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_Rw_Iw(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_Rw_Iw(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_Rw_Iw(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_Rw_Iw(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_Rw_Iw(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_Rw_Iw(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_Rw_Iw(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_u16()?;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_MwA16_Iw(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_MwA16_Iw(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_MwA16_Iw(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_MwA16_Iw(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_MwA16_Iw(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_MwA16_Iw(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_MwA16_Iw(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_MwA16_Iw(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_u16()?;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_MwA32_Iw(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_MwA32_Iw(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_MwA32_Iw(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_MwA32_Iw(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_MwA32_Iw(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_MwA32_Iw(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_MwA32_Iw(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_MwA32_Iw(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_u32()?;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_Rd_Id(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_Rd_Id(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_Rd_Id(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_Rd_Id(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_Rd_Id(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_Rd_Id(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_Rd_Id(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_Rd_Id(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_u32()?;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_MdA16_Id(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_MdA16_Id(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_MdA16_Id(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_MdA16_Id(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_MdA16_Id(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_MdA16_Id(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_MdA16_Id(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_MdA16_Id(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_u32()?;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_MdA32_Id(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_MdA32_Id(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_MdA32_Id(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_MdA32_Id(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_MdA32_Id(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_MdA32_Id(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_MdA32_Id(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_MdA32_Id(s, imm)),
                        }
                    },
                )
            }
            0x83 => {
                // Grp1 r/m16/32, imm8
                context.modrm_rv_sub3(
                    is,
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_i8()? as i16 as u16;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_Rw_Iw(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_Rw_Iw(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_Rw_Iw(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_Rw_Iw(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_Rw_Iw(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_Rw_Iw(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_Rw_Iw(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_Rw_Iw(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_i8()? as i16 as u16;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_MwA16_Iw(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_MwA16_Iw(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_MwA16_Iw(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_MwA16_Iw(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_MwA16_Iw(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_MwA16_Iw(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_MwA16_Iw(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_MwA16_Iw(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_i8()? as i16 as u16;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_MwA32_Iw(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_MwA32_Iw(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_MwA32_Iw(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_MwA32_Iw(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_MwA32_Iw(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_MwA32_Iw(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_MwA32_Iw(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_MwA32_Iw(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_i8()? as i32 as u32;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_Rd_Id(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_Rd_Id(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_Rd_Id(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_Rd_Id(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_Rd_Id(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_Rd_Id(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_Rd_Id(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_Rd_Id(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_i8()? as i32 as u32;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_MdA16_Id(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_MdA16_Id(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_MdA16_Id(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_MdA16_Id(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_MdA16_Id(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_MdA16_Id(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_MdA16_Id(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_MdA16_Id(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_i8()? as i32 as u32;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_MdA32_Id(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_MdA32_Id(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_MdA32_Id(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_MdA32_Id(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_MdA32_Id(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_MdA32_Id(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_MdA32_Id(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_MdA32_Id(s, imm)),
                        }
                    },
                )
            }
            0x84 => {
                // TEST r/m8, reg8
                context.modrm_rb(
                    is,
                    IrOp::TEST_Rb_Rb,
                    IrOp::TEST_MbA16_Rb,
                    IrOp::TEST_MbA32_Rb,
                )
            }
            0x85 => {
                // TEST r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::TEST_Rw_Rw,
                    IrOp::TEST_MwA16_Rw,
                    IrOp::TEST_MwA32_Rw,
                    IrOp::TEST_Rd_Rd,
                    IrOp::TEST_MdA16_Rd,
                    IrOp::TEST_MdA32_Rd,
                )
            }
            0x86 => {
                // XCHG r/m8, reg8
                context.modrm_rb(
                    is,
                    IrOp::XCHG_Rb_Rb,
                    IrOp::XCHG_MbA16_Rb,
                    IrOp::XCHG_MbA32_Rb,
                )
            }
            0x87 => {
                // XCHG r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::XCHG_Rw_Rw,
                    IrOp::XCHG_MwA16_Rw,
                    IrOp::XCHG_MwA32_Rw,
                    IrOp::XCHG_Rd_Rd,
                    IrOp::XCHG_MdA16_Rd,
                    IrOp::XCHG_MdA32_Rd,
                )
            }
            0x88 => {
                // MOV r/m8, reg8
                context.modrm_rb(is, IrOp::MOV_Rb_Rb, IrOp::MOV_MbA16_Rb, IrOp::MOV_MbA32_Rb)
            }
            0x89 => {
                // MOV r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::MOV_Rw_Rw,
                    IrOp::MOV_MwA16_Rw,
                    IrOp::MOV_MwA32_Rw,
                    IrOp::MOV_Rd_Rd,
                    IrOp::MOV_MdA16_Rd,
                    IrOp::MOV_MdA32_Rd,
                )
            }
            0x8a => {
                // MOV reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::MOV_Rb_Rb(r, m),
                    |m, r| IrOp::MOV_Rb_MbA16(r, m),
                    |m, r| IrOp::MOV_Rb_MbA32(r, m),
                )
            }
            0x8b => {
                // MOV reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::MOV_Rw_Rw(r, m),
                    |m, r| IrOp::MOV_Rw_MwA16(r, m),
                    |m, r| IrOp::MOV_Rw_MwA32(r, m),
                    |m, r| IrOp::MOV_Rd_Rd(r, m),
                    |m, r| IrOp::MOV_Rd_MdA16(r, m),
                    |m, r| IrOp::MOV_Rd_MdA32(r, m),
                )
            }
            0x8c => {
                // MOV r/m16/32, Sreg
                context.modrm_rv(
                    is,
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_Rw_Sr(m, sr)
                        } else {
                            IrOp::UD
                        }
                    },
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_MwA16_Sr(m, sr)
                        } else {
                            IrOp::UD
                        }
                    },
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_MwA32_Sr(m, sr)
                        } else {
                            IrOp::UD
                        }
                    },
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_Rd_Sr(m, sr)
                        } else {
                            IrOp::UD
                        }
                    },
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_MdA16_Sr(m, sr)
                        } else {
                            IrOp::UD
                        }
                    },
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_MdA32_Sr(m, sr)
                        } else {
                            IrOp::UD
                        }
                    },
                )
            }
            0x8d => {
                // LEA reg16/32, mem
                context.modrm_mv(
                    is,
                    |_| IrOp::UD,
                    |m, r| IrOp::LEA_Rw_MwA16(r, m),
                    |m, r| IrOp::LEA_Rw_MwA32(r, m),
                    |m, r| IrOp::LEA_Rd_MdA16(r, m),
                    |m, r| IrOp::LEA_Rd_MdA32(r, m),
                )
            }
            0x8e => {
                // MOV Sreg, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_Sr_Rw(sr, m)
                        } else {
                            IrOp::UD
                        }
                    },
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_Sr_MwA16(sr, m)
                        } else {
                            IrOp::UD
                        }
                    },
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_Sr_MwA32(sr, m)
                        } else {
                            IrOp::UD
                        }
                    },
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_Sr_Rd(sr, m)
                        } else {
                            IrOp::UD
                        }
                    },
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_Sr_MdA16(sr, m)
                        } else {
                            IrOp::UD
                        }
                    },
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_Sr_MdA32(sr, m)
                        } else {
                            IrOp::UD
                        }
                    },
                )
            }
            0x8f => {
                // POP r/m16/32
                context.modrm_rv_sub3(
                    is,
                    |_, s, c| match c {
                        Index3::I000 => Ok(IrOp::POP_Rw(s)),
                        _ => Ok(IrOp::UD),
                    },
                    |_, s, c| match c {
                        Index3::I000 => Ok(IrOp::POP_MwA16(s)),
                        _ => Ok(IrOp::UD),
                    },
                    |_, s, c| match c {
                        Index3::I000 => Ok(IrOp::POP_MwA32(s)),
                        _ => Ok(IrOp::UD),
                    },
                    |_, s, c| match c {
                        Index3::I000 => Ok(IrOp::POP_Rd(s)),
                        _ => Ok(IrOp::UD),
                    },
                    |_, s, c| match c {
                        Index3::I000 => Ok(IrOp::POP_MdA16(s)),
                        _ => Ok(IrOp::UD),
                    },
                    |_, s, c| match c {
                        Index3::I000 => Ok(IrOp::POP_MdA32(s)),
                        _ => Ok(IrOp::UD),
                    },
                )
            }

            0x90 => {
                // NOP/PAUSE
                if matches!(context.prefix_fx, PrefixFx::F3_REPZ) {
                    Ok(IrOp::PAUSE)
                } else {
                    Ok(IrOp::NOP)
                }
            }
            0x91..=0x97 => {
                // XCHG reg16/32, AX/EAX
                let reg_index = GprIndex32::from_u8(opcode);
                if context.is_opsize32 {
                    Ok(IrOp::XCHG_Rd_Rd(EAX, reg_index))
                } else {
                    Ok(IrOp::XCHG_Rw_Rw(AX, reg_index.downgrade()))
                }
            }
            0x98 => {
                // CBW/CWDE
                if context.is_opsize32 {
                    Ok(IrOp::MOVSX_Rd_Rw(EAX, AX))
                } else {
                    Ok(IrOp::MOVSX_Rw_Rb(AX, AL))
                }
            }
            0x99 => {
                // CWD/CDQ
                if context.is_opsize32 {
                    Ok(IrOp::CDQ)
                } else {
                    Ok(IrOp::CWD)
                }
            }
            0x9a => {
                // CALL ptr16:16/32
                let offset = if context.is_opsize32 {
                    is.fetch_u32()?
                } else {
                    is.fetch_u16()? as u32
                };
                let segment = is.fetch_u16()?;
                Ok(IrOp::CALLF_Ap(SegmentSelector(segment), Offset32(offset)))
            }
            0x9b => {
                // FWAIT/WAIT
                Ok(IrOp::WAIT)
            }
            0x9c => {
                // PUSHF/PUSHFD
                if context.is_opsize32 {
                    Ok(IrOp::PUSHFD)
                } else {
                    Ok(IrOp::PUSHF)
                }
            }
            0x9d => {
                // POPF/POPFD
                if context.is_opsize32 {
                    Ok(IrOp::POPFD)
                } else {
                    Ok(IrOp::POPF)
                }
            }
            0x9e => {
                // SAHF
                Ok(IrOp::SAHF)
            }
            0x9f => {
                // LAHF
                Ok(IrOp::LAHF)
            }

            0xa0 => {
                // MOV AL, moffs8
                if context.is_addr32 {
                    let offset = Offset32(is.fetch_u32()?);
                    Ok(IrOp::MOV_Rb_MbA32(
                        AL,
                        MemOpr32::from_sel_off(context.segment_override.unwrap_or(DS), offset),
                    ))
                } else {
                    let offset = Offset16(is.fetch_u16()?);
                    Ok(IrOp::MOV_Rb_MbA16(
                        AL,
                        MemOpr16::from_sel_off(context.segment_override.unwrap_or(DS), offset),
                    ))
                }
            }
            0xa1 => {
                // MOV AX/EAX, moffs8
                if context.is_addr32 {
                    let offset = Offset32(is.fetch_u32()?);
                    let memopr =
                        MemOpr32::from_sel_off(context.segment_override.unwrap_or(DS), offset);
                    if context.is_opsize32 {
                        Ok(IrOp::MOV_Rd_MdA32(EAX, memopr))
                    } else {
                        Ok(IrOp::MOV_Rw_MwA32(AX, memopr))
                    }
                } else {
                    let offset = Offset16(is.fetch_u16()?);
                    let memopr =
                        MemOpr16::from_sel_off(context.segment_override.unwrap_or(DS), offset);
                    if context.is_opsize32 {
                        Ok(IrOp::MOV_Rd_MdA16(EAX, memopr))
                    } else {
                        Ok(IrOp::MOV_Rw_MwA16(AX, memopr))
                    }
                }
            }
            0xa2 => {
                // MOV moffs8, AL
                if context.is_addr32 {
                    let offset = Offset32(is.fetch_u32()?);
                    Ok(IrOp::MOV_MbA32_Rb(
                        MemOpr32::from_sel_off(context.segment_override.unwrap_or(DS), offset),
                        AL,
                    ))
                } else {
                    let offset = Offset16(is.fetch_u16()?);
                    Ok(IrOp::MOV_MbA16_Rb(
                        MemOpr16::from_sel_off(context.segment_override.unwrap_or(DS), offset),
                        AL,
                    ))
                }
            }
            0xa3 => {
                // MOV moffs8, AX/EAX
                if context.is_addr32 {
                    let offset = Offset32(is.fetch_u32()?);
                    let memopr =
                        MemOpr32::from_sel_off(context.segment_override.unwrap_or(DS), offset);
                    if context.is_opsize32 {
                        Ok(IrOp::MOV_MdA32_Rd(memopr, EAX))
                    } else {
                        Ok(IrOp::MOV_MwA32_Rw(memopr, AX))
                    }
                } else {
                    let offset = Offset16(is.fetch_u16()?);
                    let memopr =
                        MemOpr16::from_sel_off(context.segment_override.unwrap_or(DS), offset);
                    if context.is_opsize32 {
                        Ok(IrOp::MOV_MdA16_Rd(memopr, EAX))
                    } else {
                        Ok(IrOp::MOV_MwA16_Rw(memopr, AX))
                    }
                }
            }
            0xa4 => {
                // MOVSB
                match context.prefix_fx {
                    PrefixFx::NP => Ok(IrOp::MOVSB(
                        context.segment_override.unwrap_or(SrIndex::DS),
                        context.is_addr32,
                    )),
                    PrefixFx::F2_REPNZ | PrefixFx::F3_REPZ => Ok(IrOp::REP_MOVSB(
                        context.segment_override.unwrap_or(SrIndex::DS),
                        context.is_addr32,
                    )),
                    PrefixFx::F0_LOCK => Ok(IrOp::UD),
                }
            }
            0xa5 => {
                // MOVSW/MOVSD
                if context.is_opsize32 {
                    match context.prefix_fx {
                        PrefixFx::NP => Ok(IrOp::MOVSD(
                            context.segment_override.unwrap_or(SrIndex::DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F2_REPNZ | PrefixFx::F3_REPZ => Ok(IrOp::REP_MOVSD(
                            context.segment_override.unwrap_or(SrIndex::DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F0_LOCK => Ok(IrOp::UD),
                    }
                } else {
                    match context.prefix_fx {
                        PrefixFx::NP => Ok(IrOp::MOVSW(
                            context.segment_override.unwrap_or(SrIndex::DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F2_REPNZ | PrefixFx::F3_REPZ => Ok(IrOp::REP_MOVSW(
                            context.segment_override.unwrap_or(SrIndex::DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F0_LOCK => Ok(IrOp::UD),
                    }
                }
            }
            0xa6 => {
                // CMPSB
                match context.prefix_fx {
                    PrefixFx::NP => Ok(IrOp::CMPSB(
                        context.segment_override.unwrap_or(SrIndex::DS),
                        context.is_addr32,
                    )),
                    PrefixFx::F2_REPNZ => Ok(IrOp::REPNZ_CMPSB(
                        context.segment_override.unwrap_or(SrIndex::DS),
                        context.is_addr32,
                    )),
                    PrefixFx::F3_REPZ => Ok(IrOp::REPZ_CMPSB(
                        context.segment_override.unwrap_or(SrIndex::DS),
                        context.is_addr32,
                    )),
                    PrefixFx::F0_LOCK => Ok(IrOp::UD),
                }
            }
            0xa7 => {
                // CMPSW/CMPSD
                if context.is_opsize32 {
                    match context.prefix_fx {
                        PrefixFx::NP => Ok(IrOp::CMPSD(
                            context.segment_override.unwrap_or(SrIndex::DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F2_REPNZ => Ok(IrOp::REPNZ_CMPSD(
                            context.segment_override.unwrap_or(SrIndex::DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F3_REPZ => Ok(IrOp::REPZ_CMPSD(
                            context.segment_override.unwrap_or(SrIndex::DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F0_LOCK => Ok(IrOp::UD),
                    }
                } else {
                    match context.prefix_fx {
                        PrefixFx::NP => Ok(IrOp::CMPSW(
                            context.segment_override.unwrap_or(SrIndex::DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F2_REPNZ => Ok(IrOp::REPNZ_CMPSW(
                            context.segment_override.unwrap_or(SrIndex::DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F3_REPZ => Ok(IrOp::REPZ_CMPSW(
                            context.segment_override.unwrap_or(SrIndex::DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F0_LOCK => Ok(IrOp::UD),
                    }
                }
            }
            0xa8 => {
                // TEST AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::TEST_Rb_Ib(AL, imm))
            }
            0xa9 => {
                // TEST AX/EAX, imm16/32
                if context.is_opsize32 {
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::TEST_Rd_Id(EAX, imm))
                } else {
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::TEST_Rw_Iw(AX, imm))
                }
            }
            0xaa => {
                // STOSB
                match context.prefix_fx {
                    PrefixFx::NP => Ok(IrOp::STOSB(SrIndex::ES, context.is_addr32)),
                    PrefixFx::F2_REPNZ | PrefixFx::F3_REPZ => {
                        Ok(IrOp::REP_STOSB(SrIndex::ES, context.is_addr32))
                    }
                    PrefixFx::F0_LOCK => Ok(IrOp::UD),
                }
            }
            0xab => {
                // STOSW/STOSD
                if context.is_opsize32 {
                    match context.prefix_fx {
                        PrefixFx::NP => Ok(IrOp::STOSD(SrIndex::ES, context.is_addr32)),
                        PrefixFx::F2_REPNZ | PrefixFx::F3_REPZ => {
                            Ok(IrOp::REP_STOSD(SrIndex::ES, context.is_addr32))
                        }
                        PrefixFx::F0_LOCK => Ok(IrOp::UD),
                    }
                } else {
                    match context.prefix_fx {
                        PrefixFx::NP => Ok(IrOp::STOSW(SrIndex::ES, context.is_addr32)),
                        PrefixFx::F2_REPNZ | PrefixFx::F3_REPZ => {
                            Ok(IrOp::REP_STOSW(SrIndex::ES, context.is_addr32))
                        }
                        PrefixFx::F0_LOCK => Ok(IrOp::UD),
                    }
                }
            }
            0xac => {
                // LODSB
                match context.prefix_fx {
                    PrefixFx::NP => Ok(IrOp::LODSB(
                        context.segment_override.unwrap_or(SrIndex::DS),
                        context.is_addr32,
                    )),
                    PrefixFx::F2_REPNZ | PrefixFx::F3_REPZ => Ok(IrOp::REP_LODSB(
                        context.segment_override.unwrap_or(SrIndex::DS),
                        context.is_addr32,
                    )),
                    PrefixFx::F0_LOCK => Ok(IrOp::UD),
                }
            }
            0xad => {
                // LODSW/LODSD
                if context.is_opsize32 {
                    match context.prefix_fx {
                        PrefixFx::NP => Ok(IrOp::LODSD(
                            context.segment_override.unwrap_or(SrIndex::DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F2_REPNZ | PrefixFx::F3_REPZ => Ok(IrOp::REP_LODSD(
                            context.segment_override.unwrap_or(SrIndex::DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F0_LOCK => Ok(IrOp::UD),
                    }
                } else {
                    match context.prefix_fx {
                        PrefixFx::NP => Ok(IrOp::LODSW(
                            context.segment_override.unwrap_or(SrIndex::DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F2_REPNZ | PrefixFx::F3_REPZ => Ok(IrOp::REP_LODSW(
                            context.segment_override.unwrap_or(SrIndex::DS),
                            context.is_addr32,
                        )),
                        PrefixFx::F0_LOCK => Ok(IrOp::UD),
                    }
                }
            }
            0xae => {
                // SCASB
                match context.prefix_fx {
                    PrefixFx::NP => Ok(IrOp::SCASB(ES, context.is_addr32)),
                    PrefixFx::F2_REPNZ => Ok(IrOp::REPNZ_SCASB(ES, context.is_addr32)),
                    PrefixFx::F3_REPZ => Ok(IrOp::REPZ_SCASB(ES, context.is_addr32)),
                    PrefixFx::F0_LOCK => Ok(IrOp::UD),
                }
            }
            0xaf => {
                // SCASW/SCASD
                if context.is_opsize32 {
                    match context.prefix_fx {
                        PrefixFx::NP => Ok(IrOp::SCASD(ES, context.is_addr32)),
                        PrefixFx::F2_REPNZ => Ok(IrOp::REPNZ_SCASD(ES, context.is_addr32)),
                        PrefixFx::F3_REPZ => Ok(IrOp::REPZ_SCASD(ES, context.is_addr32)),
                        PrefixFx::F0_LOCK => Ok(IrOp::UD),
                    }
                } else {
                    match context.prefix_fx {
                        PrefixFx::NP => Ok(IrOp::SCASW(ES, context.is_addr32)),
                        PrefixFx::F2_REPNZ => Ok(IrOp::REPNZ_SCASW(ES, context.is_addr32)),
                        PrefixFx::F3_REPZ => Ok(IrOp::REPZ_SCASW(ES, context.is_addr32)),
                        PrefixFx::F0_LOCK => Ok(IrOp::UD),
                    }
                }
            }

            0xb0..=0xb7 => {
                // MOV reg8, imm8
                let reg_index = GprIndex8::from_u8(opcode);
                let imm = is.fetch_u8()?;
                Ok(IrOp::MOV_Rb_Ib(reg_index, imm))
            }
            0xb8..=0xbf => {
                // MOV reg16/32, imm16/32
                if context.is_opsize32 {
                    let reg_index = GprIndex32::from_u8(opcode);
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::MOV_Rd_Id(reg_index, imm))
                } else {
                    let reg_index = GprIndex16::from_u8(opcode);
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::MOV_Rw_Iw(reg_index, imm))
                }
            }

            0xc0 => {
                // Grp2 r/m8, imm8 (shift/rotate)
                context.modrm_rb_sub3(
                    is,
                    |is, s, c| {
                        let imm = is.fetch_u8()?;
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rb_Ib(s, imm)),
                            OpShift::ROR => Ok(IrOp::ROR_Rb_Ib(s, imm)),
                            OpShift::RCL => Ok(IrOp::RCL_Rb_Ib(s, imm)),
                            OpShift::RCR => Ok(IrOp::RCR_Rb_Ib(s, imm)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rb_Ib(s, imm)),
                            OpShift::SHR => Ok(IrOp::SHR_Rb_Ib(s, imm)),
                            OpShift::SAR => Ok(IrOp::SAR_Rb_Ib(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let imm = is.fetch_u8()?;
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MbA16_Ib(s, imm)),
                            OpShift::ROR => Ok(IrOp::ROR_MbA16_Ib(s, imm)),
                            OpShift::RCL => Ok(IrOp::RCL_MbA16_Ib(s, imm)),
                            OpShift::RCR => Ok(IrOp::RCR_MbA16_Ib(s, imm)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MbA16_Ib(s, imm)),
                            OpShift::SHR => Ok(IrOp::SHR_MbA16_Ib(s, imm)),
                            OpShift::SAR => Ok(IrOp::SAR_MbA16_Ib(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let imm = is.fetch_u8()?;
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MbA32_Ib(s, imm)),
                            OpShift::ROR => Ok(IrOp::ROR_MbA32_Ib(s, imm)),
                            OpShift::RCL => Ok(IrOp::RCL_MbA32_Ib(s, imm)),
                            OpShift::RCR => Ok(IrOp::RCR_MbA32_Ib(s, imm)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MbA32_Ib(s, imm)),
                            OpShift::SHR => Ok(IrOp::SHR_MbA32_Ib(s, imm)),
                            OpShift::SAR => Ok(IrOp::SAR_MbA32_Ib(s, imm)),
                        }
                    },
                )
            }
            0xc1 => {
                // Grp2 r/m16/32, imm8 (shift/rotate)
                context.modrm_rv_sub3(
                    is,
                    |is, s, c| {
                        let imm = is.fetch_u8()?;
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rw_Ib(s, imm)),
                            OpShift::ROR => Ok(IrOp::ROR_Rw_Ib(s, imm)),
                            OpShift::RCL => Ok(IrOp::RCL_Rw_Ib(s, imm)),
                            OpShift::RCR => Ok(IrOp::RCR_Rw_Ib(s, imm)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rw_Ib(s, imm)),
                            OpShift::SHR => Ok(IrOp::SHR_Rw_Ib(s, imm)),
                            OpShift::SAR => Ok(IrOp::SAR_Rw_Ib(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let imm = is.fetch_u8()?;
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MwA16_Ib(s, imm)),
                            OpShift::ROR => Ok(IrOp::ROR_MwA16_Ib(s, imm)),
                            OpShift::RCL => Ok(IrOp::RCL_MwA16_Ib(s, imm)),
                            OpShift::RCR => Ok(IrOp::RCR_MwA16_Ib(s, imm)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MwA16_Ib(s, imm)),
                            OpShift::SHR => Ok(IrOp::SHR_MwA16_Ib(s, imm)),
                            OpShift::SAR => Ok(IrOp::SAR_MwA16_Ib(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let imm = is.fetch_u8()?;
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MwA32_Ib(s, imm)),
                            OpShift::ROR => Ok(IrOp::ROR_MwA32_Ib(s, imm)),
                            OpShift::RCL => Ok(IrOp::RCL_MwA32_Ib(s, imm)),
                            OpShift::RCR => Ok(IrOp::RCR_MwA32_Ib(s, imm)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MwA32_Ib(s, imm)),
                            OpShift::SHR => Ok(IrOp::SHR_MwA32_Ib(s, imm)),
                            OpShift::SAR => Ok(IrOp::SAR_MwA32_Ib(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let imm = is.fetch_u8()?;
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rd_Ib(s, imm)),
                            OpShift::ROR => Ok(IrOp::ROR_Rd_Ib(s, imm)),
                            OpShift::RCL => Ok(IrOp::RCL_Rd_Ib(s, imm)),
                            OpShift::RCR => Ok(IrOp::RCR_Rd_Ib(s, imm)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rd_Ib(s, imm)),
                            OpShift::SHR => Ok(IrOp::SHR_Rd_Ib(s, imm)),
                            OpShift::SAR => Ok(IrOp::SAR_Rd_Ib(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let imm = is.fetch_u8()?;
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MdA16_Ib(s, imm)),
                            OpShift::ROR => Ok(IrOp::ROR_MdA16_Ib(s, imm)),
                            OpShift::RCL => Ok(IrOp::RCL_MdA16_Ib(s, imm)),
                            OpShift::RCR => Ok(IrOp::RCR_MdA16_Ib(s, imm)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MdA16_Ib(s, imm)),
                            OpShift::SHR => Ok(IrOp::SHR_MdA16_Ib(s, imm)),
                            OpShift::SAR => Ok(IrOp::SAR_MdA16_Ib(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let imm = is.fetch_u8()?;
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MdA32_Ib(s, imm)),
                            OpShift::ROR => Ok(IrOp::ROR_MdA32_Ib(s, imm)),
                            OpShift::RCL => Ok(IrOp::RCL_MdA32_Ib(s, imm)),
                            OpShift::RCR => Ok(IrOp::RCR_MdA32_Ib(s, imm)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MdA32_Ib(s, imm)),
                            OpShift::SHR => Ok(IrOp::SHR_MdA32_Ib(s, imm)),
                            OpShift::SAR => Ok(IrOp::SAR_MdA32_Ib(s, imm)),
                        }
                    },
                )
            }
            0xc2 => {
                // RET imm16
                let imm = is.fetch_u16()?;
                if context.is_opsize32 {
                    Ok(IrOp::RET_D32(imm))
                } else {
                    Ok(IrOp::RET_D16(imm))
                }
            }
            0xc3 => {
                // RET
                if context.is_opsize32 {
                    Ok(IrOp::RET_D32(0))
                } else {
                    Ok(IrOp::RET_D16(0))
                }
            }
            0xc4 => {
                // LES r16/32, m16:16/32
                context.modrm_mv(
                    is,
                    |_| IrOp::UD,
                    |m, r| IrOp::LES_Rw_MwA16(r, m),
                    |m, r| IrOp::LES_Rw_MwA32(r, m),
                    |m, r| IrOp::LES_Rd_MdA16(r, m),
                    |m, r| IrOp::LES_Rd_MdA32(r, m),
                )
            }
            0xc5 => {
                // LDS r16/32, m16:16/32
                context.modrm_mv(
                    is,
                    |_| IrOp::UD,
                    |m, r| IrOp::LDS_Rw_MwA16(r, m),
                    |m, r| IrOp::LDS_Rw_MwA32(r, m),
                    |m, r| IrOp::LDS_Rd_MdA16(r, m),
                    |m, r| IrOp::LDS_Rd_MdA32(r, m),
                )
            }
            0xc6 => {
                // MOV r/m8, imm8
                context.modrm_rb_sub3(
                    is,
                    |is, d, c| match c {
                        Index3::I000 => {
                            let imm = is.fetch_u8()?;
                            Ok(IrOp::MOV_Rb_Ib(d, imm))
                        }
                        _ => Ok(IrOp::UD),
                    },
                    |is, d, c| match c {
                        Index3::I000 => {
                            let imm = is.fetch_u8()?;
                            Ok(IrOp::MOV_MbA16_Ib(d, imm))
                        }
                        _ => Ok(IrOp::UD),
                    },
                    |is, d, c| match c {
                        Index3::I000 => {
                            let imm = is.fetch_u8()?;
                            Ok(IrOp::MOV_MbA32_Ib(d, imm))
                        }
                        _ => Ok(IrOp::UD),
                    },
                )
            }
            0xc7 => {
                // MOV r/m16/32, imm16/32
                context.modrm_rv_sub3(
                    is,
                    |is, d, c| match c {
                        Index3::I000 => {
                            let imm = is.fetch_u16()?;
                            Ok(IrOp::MOV_Rw_Iw(d, imm))
                        }
                        _ => Ok(IrOp::UD),
                    },
                    |is, d, _c| {
                        let imm = is.fetch_u16()?;
                        Ok(IrOp::MOV_MwA16_Iw(d, imm))
                    },
                    |is, d, _c| {
                        let imm = is.fetch_u16()?;
                        Ok(IrOp::MOV_MwA32_Iw(d, imm))
                    },
                    |is, d, _c| {
                        let imm = is.fetch_u32()?;
                        Ok(IrOp::MOV_Rd_Id(d, imm))
                    },
                    |is, d, _c| {
                        let imm = is.fetch_u32()?;
                        Ok(IrOp::MOV_MdA16_Id(d, imm))
                    },
                    |is, d, _c| {
                        let imm = is.fetch_u32()?;
                        Ok(IrOp::MOV_MdA32_Id(d, imm))
                    },
                )
            }
            0xc8 => {
                // ENTER imm16, imm8
                // TODO: support prefix 66
                let iw = is.fetch_u16()?;
                let ib = is.fetch_u8()?;
                Ok(IrOp::ENTER_Iw_Ib(iw, ib))
            }
            0xc9 => {
                // LEAVE
                // TODO: support prefix 66
                Ok(IrOp::LEAVE)
            }
            0xca => {
                // RETF imm16
                let imm = is.fetch_u16()?;
                if context.is_opsize32 {
                    Ok(IrOp::RETF_D32(imm))
                } else {
                    Ok(IrOp::RETF_D16(imm))
                }
            }
            0xcb => {
                // RETF
                if context.is_opsize32 {
                    Ok(IrOp::RETF_D32(0))
                } else {
                    Ok(IrOp::RETF_D16(0))
                }
            }
            0xcc => {
                // INT3
                Ok(IrOp::INT_Ib(3))
            }
            0xcd => {
                // INT imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::INT_Ib(imm))
            }
            0xce => {
                // INTO
                Ok(IrOp::INTO)
            }
            0xcf => {
                // IRET
                if context.is_opsize32 {
                    Ok(IrOp::IRETD)
                } else {
                    Ok(IrOp::IRET)
                }
            }

            0xd0 => {
                // Grp2 r/m8, 1 (shift/rotate)
                context.modrm_rb_sub3(
                    is,
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rb_Ib(s, 1)),
                            OpShift::ROR => Ok(IrOp::ROR_Rb_Ib(s, 1)),
                            OpShift::RCL => Ok(IrOp::RCL_Rb_Ib(s, 1)),
                            OpShift::RCR => Ok(IrOp::RCR_Rb_Ib(s, 1)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rb_Ib(s, 1)),
                            OpShift::SHR => Ok(IrOp::SHR_Rb_Ib(s, 1)),
                            OpShift::SAR => Ok(IrOp::SAR_Rb_Ib(s, 1)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MbA16_Ib(s, 1)),
                            OpShift::ROR => Ok(IrOp::ROR_MbA16_Ib(s, 1)),
                            OpShift::RCL => Ok(IrOp::RCL_MbA16_Ib(s, 1)),
                            OpShift::RCR => Ok(IrOp::RCR_MbA16_Ib(s, 1)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MbA16_Ib(s, 1)),
                            OpShift::SHR => Ok(IrOp::SHR_MbA16_Ib(s, 1)),
                            OpShift::SAR => Ok(IrOp::SAR_MbA16_Ib(s, 1)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MbA32_Ib(s, 1)),
                            OpShift::ROR => Ok(IrOp::ROR_MbA32_Ib(s, 1)),
                            OpShift::RCL => Ok(IrOp::RCL_MbA32_Ib(s, 1)),
                            OpShift::RCR => Ok(IrOp::RCR_MbA32_Ib(s, 1)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MbA32_Ib(s, 1)),
                            OpShift::SHR => Ok(IrOp::SHR_MbA32_Ib(s, 1)),
                            OpShift::SAR => Ok(IrOp::SAR_MbA32_Ib(s, 1)),
                        }
                    },
                )
            }
            0xd1 => {
                // Grp2 r/m16/32, 1 (shift/rotate)
                context.modrm_rv_sub3(
                    is,
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rw_Ib(s, 1)),
                            OpShift::ROR => Ok(IrOp::ROR_Rw_Ib(s, 1)),
                            OpShift::RCL => Ok(IrOp::RCL_Rw_Ib(s, 1)),
                            OpShift::RCR => Ok(IrOp::RCR_Rw_Ib(s, 1)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rw_Ib(s, 1)),
                            OpShift::SHR => Ok(IrOp::SHR_Rw_Ib(s, 1)),
                            OpShift::SAR => Ok(IrOp::SAR_Rw_Ib(s, 1)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MwA16_Ib(s, 1)),
                            OpShift::ROR => Ok(IrOp::ROR_MwA16_Ib(s, 1)),
                            OpShift::RCL => Ok(IrOp::RCL_MwA16_Ib(s, 1)),
                            OpShift::RCR => Ok(IrOp::RCR_MwA16_Ib(s, 1)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MwA16_Ib(s, 1)),
                            OpShift::SHR => Ok(IrOp::SHR_MwA16_Ib(s, 1)),
                            OpShift::SAR => Ok(IrOp::SAR_MwA16_Ib(s, 1)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MwA32_Ib(s, 1)),
                            OpShift::ROR => Ok(IrOp::ROR_MwA32_Ib(s, 1)),
                            OpShift::RCL => Ok(IrOp::RCL_MwA32_Ib(s, 1)),
                            OpShift::RCR => Ok(IrOp::RCR_MwA32_Ib(s, 1)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MwA32_Ib(s, 1)),
                            OpShift::SHR => Ok(IrOp::SHR_MwA32_Ib(s, 1)),
                            OpShift::SAR => Ok(IrOp::SAR_MwA32_Ib(s, 1)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rd_Ib(s, 1)),
                            OpShift::ROR => Ok(IrOp::ROR_Rd_Ib(s, 1)),
                            OpShift::RCL => Ok(IrOp::RCL_Rd_Ib(s, 1)),
                            OpShift::RCR => Ok(IrOp::RCR_Rd_Ib(s, 1)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rd_Ib(s, 1)),
                            OpShift::SHR => Ok(IrOp::SHR_Rd_Ib(s, 1)),
                            OpShift::SAR => Ok(IrOp::SAR_Rd_Ib(s, 1)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MdA16_Ib(s, 1)),
                            OpShift::ROR => Ok(IrOp::ROR_MdA16_Ib(s, 1)),
                            OpShift::RCL => Ok(IrOp::RCL_MdA16_Ib(s, 1)),
                            OpShift::RCR => Ok(IrOp::RCR_MdA16_Ib(s, 1)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MdA16_Ib(s, 1)),
                            OpShift::SHR => Ok(IrOp::SHR_MdA16_Ib(s, 1)),
                            OpShift::SAR => Ok(IrOp::SAR_MdA16_Ib(s, 1)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MdA32_Ib(s, 1)),
                            OpShift::ROR => Ok(IrOp::ROR_MdA32_Ib(s, 1)),
                            OpShift::RCL => Ok(IrOp::RCL_MdA32_Ib(s, 1)),
                            OpShift::RCR => Ok(IrOp::RCR_MdA32_Ib(s, 1)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MdA32_Ib(s, 1)),
                            OpShift::SHR => Ok(IrOp::SHR_MdA32_Ib(s, 1)),
                            OpShift::SAR => Ok(IrOp::SAR_MdA32_Ib(s, 1)),
                        }
                    },
                )
            }
            0xd2 => {
                // Grp2 r/m8, CL (shift/rotate)
                context.modrm_rb_sub3(
                    is,
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rb_Cl(s)),
                            OpShift::ROR => Ok(IrOp::ROR_Rb_Cl(s)),
                            OpShift::RCL => Ok(IrOp::RCL_Rb_Cl(s)),
                            OpShift::RCR => Ok(IrOp::RCR_Rb_Cl(s)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rb_Cl(s)),
                            OpShift::SHR => Ok(IrOp::SHR_Rb_Cl(s)),
                            OpShift::SAR => Ok(IrOp::SAR_Rb_Cl(s)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MbA16_Cl(s)),
                            OpShift::ROR => Ok(IrOp::ROR_MbA16_Cl(s)),
                            OpShift::RCL => Ok(IrOp::RCL_MbA16_Cl(s)),
                            OpShift::RCR => Ok(IrOp::RCR_MbA16_Cl(s)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MbA16_Cl(s)),
                            OpShift::SHR => Ok(IrOp::SHR_MbA16_Cl(s)),
                            OpShift::SAR => Ok(IrOp::SAR_MbA16_Cl(s)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MbA32_Cl(s)),
                            OpShift::ROR => Ok(IrOp::ROR_MbA32_Cl(s)),
                            OpShift::RCL => Ok(IrOp::RCL_MbA32_Cl(s)),
                            OpShift::RCR => Ok(IrOp::RCR_MbA32_Cl(s)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MbA32_Cl(s)),
                            OpShift::SHR => Ok(IrOp::SHR_MbA32_Cl(s)),
                            OpShift::SAR => Ok(IrOp::SAR_MbA32_Cl(s)),
                        }
                    },
                )
            }
            0xd3 => {
                // Grp2 r/m16/32, CL (shift/rotate)
                context.modrm_rv_sub3(
                    is,
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rw_Cl(s)),
                            OpShift::ROR => Ok(IrOp::ROR_Rw_Cl(s)),
                            OpShift::RCL => Ok(IrOp::RCL_Rw_Cl(s)),
                            OpShift::RCR => Ok(IrOp::RCR_Rw_Cl(s)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rw_Cl(s)),
                            OpShift::SHR => Ok(IrOp::SHR_Rw_Cl(s)),
                            OpShift::SAR => Ok(IrOp::SAR_Rw_Cl(s)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MwA16_Cl(s)),
                            OpShift::ROR => Ok(IrOp::ROR_MwA16_Cl(s)),
                            OpShift::RCL => Ok(IrOp::RCL_MwA16_Cl(s)),
                            OpShift::RCR => Ok(IrOp::RCR_MwA16_Cl(s)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MwA16_Cl(s)),
                            OpShift::SHR => Ok(IrOp::SHR_MwA16_Cl(s)),
                            OpShift::SAR => Ok(IrOp::SAR_MwA16_Cl(s)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MwA32_Cl(s)),
                            OpShift::ROR => Ok(IrOp::ROR_MwA32_Cl(s)),
                            OpShift::RCL => Ok(IrOp::RCL_MwA32_Cl(s)),
                            OpShift::RCR => Ok(IrOp::RCR_MwA32_Cl(s)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MwA32_Cl(s)),
                            OpShift::SHR => Ok(IrOp::SHR_MwA32_Cl(s)),
                            OpShift::SAR => Ok(IrOp::SAR_MwA32_Cl(s)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rd_Cl(s)),
                            OpShift::ROR => Ok(IrOp::ROR_Rd_Cl(s)),
                            OpShift::RCL => Ok(IrOp::RCL_Rd_Cl(s)),
                            OpShift::RCR => Ok(IrOp::RCR_Rd_Cl(s)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rd_Cl(s)),
                            OpShift::SHR => Ok(IrOp::SHR_Rd_Cl(s)),
                            OpShift::SAR => Ok(IrOp::SAR_Rd_Cl(s)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MdA16_Cl(s)),
                            OpShift::ROR => Ok(IrOp::ROR_MdA16_Cl(s)),
                            OpShift::RCL => Ok(IrOp::RCL_MdA16_Cl(s)),
                            OpShift::RCR => Ok(IrOp::RCR_MdA16_Cl(s)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MdA16_Cl(s)),
                            OpShift::SHR => Ok(IrOp::SHR_MdA16_Cl(s)),
                            OpShift::SAR => Ok(IrOp::SAR_MdA16_Cl(s)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_MdA32_Cl(s)),
                            OpShift::ROR => Ok(IrOp::ROR_MdA32_Cl(s)),
                            OpShift::RCL => Ok(IrOp::RCL_MdA32_Cl(s)),
                            OpShift::RCR => Ok(IrOp::RCR_MdA32_Cl(s)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_MdA32_Cl(s)),
                            OpShift::SHR => Ok(IrOp::SHR_MdA32_Cl(s)),
                            OpShift::SAR => Ok(IrOp::SAR_MdA32_Cl(s)),
                        }
                    },
                )
            }
            0xd4 => {
                // AAM imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::AAM_Ib(imm))
            }
            0xd5 => {
                // AAD imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::AAD_Ib(imm))
            }
            0xd6 => {
                // SALC
                Ok(IrOp::SALC)
            }
            0xd7 => {
                // XLAT
                if context.is_addr32 {
                    Ok(IrOp::XLAT_MbA32(MemOpr32::for_xlat(
                        context.segment_override.unwrap_or(DS),
                    )))
                } else {
                    Ok(IrOp::XLAT_MbA16(MemOpr16::for_xlat(
                        context.segment_override.unwrap_or(DS),
                    )))
                }
            }

            0xd8..=0xdf => {
                // FPU instructions (escape 0xd8..0xdf)
                context.modrm_rb(
                    is,
                    |m, r| IrOp::ESC_Rb(opcode, r as u8, m),
                    |m, r| IrOp::ESC_MbA16(opcode, r as u8, m),
                    |m, r| IrOp::ESC_MbA32(opcode, r as u8, m),
                )
            }

            0xe0 => {
                // LOOPNE/LOOPNZ rel8
                let rel = is.fetch_i8()? as i32;
                let target = if context.is_opsize32 {
                    Offset32(is.current_eip().0.wrapping_add(rel as u32))
                } else {
                    Offset32(is.current_eip().0.wrapping_add(rel as u32) & 0x0000_ffff)
                };
                Ok(IrOp::LOOPNZ_Jv(target))
            }
            0xe1 => {
                // LOOPE/LOOPZ rel8
                let rel = is.fetch_i8()? as i32;
                let target = if context.is_opsize32 {
                    Offset32(is.current_eip().0.wrapping_add(rel as u32))
                } else {
                    Offset32(is.current_eip().0.wrapping_add(rel as u32) & 0x0000_ffff)
                };
                Ok(IrOp::LOOPZ_Jv(target))
            }
            0xe2 => {
                // LOOP rel8
                let rel = is.fetch_i8()? as i32;
                let target = if context.is_opsize32 {
                    Offset32(is.current_eip().0.wrapping_add(rel as u32))
                } else {
                    Offset32(is.current_eip().0.wrapping_add(rel as u32) & 0x0000_ffff)
                };
                Ok(IrOp::LOOP_Jv(target))
            }
            0xe3 => {
                // JECXZ/JRCXZ rel8
                let rel = is.fetch_i8()? as i32;
                let target = if context.is_opsize32 {
                    Offset32(is.current_eip().0.wrapping_add(rel as u32))
                } else {
                    Offset32(is.current_eip().0.wrapping_add(rel as u32) & 0x0000_ffff)
                };
                Ok(IrOp::JCXZ_Jv(target))
            }
            0xe4 => {
                // IN AL, imm8
                let port = is.fetch_u8()?;
                Ok(IrOp::IN_Al_Ib(port))
            }
            0xe5 => {
                // IN AX, imm16/32
                let port = is.fetch_u8()?;
                if context.is_opsize32 {
                    Ok(IrOp::IN_Ad_Ib(port))
                } else {
                    Ok(IrOp::IN_Aw_Ib(port))
                }
            }
            0xe6 => {
                // OUT imm8, AL
                let port = is.fetch_u8()?;
                Ok(IrOp::OUT_Ib_Al(port))
            }
            0xe7 => {
                // OUT imm8, AX/EAX
                let port = is.fetch_u8()?;
                if context.is_opsize32 {
                    Ok(IrOp::OUT_Ib_Ad(port))
                } else {
                    Ok(IrOp::OUT_Ib_Aw(port))
                }
            }
            0xe8 => {
                // CALL rel16/32
                let target = if context.is_opsize32 {
                    let rel = is.fetch_i32()?;
                    Offset32(is.current_eip().0.wrapping_add(rel as u32))
                } else {
                    let rel = is.fetch_i16()?;
                    Offset32(is.current_eip().0.wrapping_add(rel as u32) & 0x0000_ffff)
                };
                Ok(IrOp::CALL_Jv(target))
            }
            0xe9 => {
                // JMP rel16/32
                let target = if context.is_opsize32 {
                    let rel = is.fetch_i32()?;
                    Offset32(is.current_eip().0.wrapping_add(rel as u32))
                } else {
                    let rel = is.fetch_i16()?;
                    Offset32(is.current_eip().0.wrapping_add(rel as u32) & 0x0000_ffff)
                };
                Ok(IrOp::JMP_Jv(target))
            }
            0xea => {
                // JMP ptr16:16/32
                let offset = if context.is_opsize32 {
                    is.fetch_u32()?
                } else {
                    is.fetch_u16()? as u32
                };
                let segment = is.fetch_u16()?;
                Ok(IrOp::JMPF_Ap(SegmentSelector(segment), Offset32(offset)))
            }
            0xeb => {
                // JMP rel8
                let rel = is.fetch_i8()? as i32;
                let target = if context.is_opsize32 {
                    Offset32(is.current_eip().0.wrapping_add(rel as u32))
                } else {
                    Offset32(is.current_eip().0.wrapping_add(rel as u32) & 0x0000_ffff)
                };
                Ok(IrOp::JMP_Jv(target))
            }
            0xec => {
                // IN AL, DX
                Ok(IrOp::IN_Al_Dx)
            }
            0xed => {
                // IN AX/EAX, DX
                if context.is_opsize32 {
                    Ok(IrOp::IN_Ad_Dx)
                } else {
                    Ok(IrOp::IN_Aw_Dx)
                }
            }
            0xee => {
                // OUT DX, AL
                Ok(IrOp::OUT_Dx_Al)
            }
            0xef => {
                // OUT DX, AX/EAX
                if context.is_opsize32 {
                    Ok(IrOp::OUT_Dx_Ad)
                } else {
                    Ok(IrOp::OUT_Dx_Aw)
                }
            }

            // 0xf0: LOCK prefix
            0xf1 => {
                // INT1 (ICEBP)
                Ok(IrOp::ICEBP)
            }
            // 0xf2: REPNE/REPNZ prefix
            // 0xf3: REP/REPE/REPZ prefix
            0xf4 => {
                // HLT
                Ok(IrOp::HLT)
            }
            0xf5 => {
                // CMC
                Ok(IrOp::CMC)
            }
            0xf6 => {
                // grp3 r/m8
                context.modrm_rb_sub3(
                    is,
                    |is, s, c| {
                        let sub_op = OpGrp3::from_idx3(c);
                        match sub_op {
                            OpGrp3::TEST | OpGrp3::_TEST1 => {
                                let imm = is.fetch_u8()?;
                                Ok(IrOp::TEST_Rb_Ib(s, imm))
                            }
                            OpGrp3::NOT => Ok(IrOp::NOT_Rb(s)),
                            OpGrp3::NEG => Ok(IrOp::NEG_Rb(s)),
                            OpGrp3::MUL => Ok(IrOp::MUL_Rb(s)),
                            OpGrp3::IMUL => Ok(IrOp::IMUL_Rb(s)),
                            OpGrp3::DIV => Ok(IrOp::DIV_Rb(s)),
                            OpGrp3::IDIV => Ok(IrOp::IDIV_Rb(s)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp3::from_idx3(c);
                        match sub_op {
                            OpGrp3::TEST | OpGrp3::_TEST1 => {
                                let imm = is.fetch_u8()?;
                                Ok(IrOp::TEST_MbA16_Ib(s, imm))
                            }
                            OpGrp3::NOT => Ok(IrOp::NOT_MbA16(s)),
                            OpGrp3::NEG => Ok(IrOp::NEG_MbA16(s)),
                            OpGrp3::MUL => Ok(IrOp::MUL_MbA16(s)),
                            OpGrp3::IMUL => Ok(IrOp::IMUL_MbA16(s)),
                            OpGrp3::DIV => Ok(IrOp::DIV_MbA16(s)),
                            OpGrp3::IDIV => Ok(IrOp::IDIV_MbA16(s)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp3::from_idx3(c);
                        match sub_op {
                            OpGrp3::TEST | OpGrp3::_TEST1 => {
                                let imm = is.fetch_u8()?;
                                Ok(IrOp::TEST_MbA32_Ib(s, imm))
                            }
                            OpGrp3::NOT => Ok(IrOp::NOT_MbA32(s)),
                            OpGrp3::NEG => Ok(IrOp::NEG_MbA32(s)),
                            OpGrp3::MUL => Ok(IrOp::MUL_MbA32(s)),
                            OpGrp3::IMUL => Ok(IrOp::IMUL_MbA32(s)),
                            OpGrp3::DIV => Ok(IrOp::DIV_MbA32(s)),
                            OpGrp3::IDIV => Ok(IrOp::IDIV_MbA32(s)),
                        }
                    },
                )
            }
            0xf7 => {
                // grp3 r/m16/32
                context.modrm_rv_sub3(
                    is,
                    |is, s, c| {
                        let sub_op = OpGrp3::from_idx3(c);
                        match sub_op {
                            OpGrp3::TEST | OpGrp3::_TEST1 => {
                                let imm = is.fetch_u16()?;
                                Ok(IrOp::TEST_Rw_Iw(s, imm))
                            }
                            OpGrp3::NOT => Ok(IrOp::NOT_Rw(s)),
                            OpGrp3::NEG => Ok(IrOp::NEG_Rw(s)),
                            OpGrp3::MUL => Ok(IrOp::MUL_Rw(s)),
                            OpGrp3::IMUL => Ok(IrOp::IMUL_Rw(s)),
                            OpGrp3::DIV => Ok(IrOp::DIV_Rw(s)),
                            OpGrp3::IDIV => Ok(IrOp::IDIV_Rw(s)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp3::from_idx3(c);
                        match sub_op {
                            OpGrp3::TEST | OpGrp3::_TEST1 => {
                                let imm = is.fetch_u16()?;
                                Ok(IrOp::TEST_MwA16_Iw(s, imm))
                            }
                            OpGrp3::NOT => Ok(IrOp::NOT_MwA16(s)),
                            OpGrp3::NEG => Ok(IrOp::NEG_MwA16(s)),
                            OpGrp3::MUL => Ok(IrOp::MUL_MwA16(s)),
                            OpGrp3::IMUL => Ok(IrOp::IMUL_MwA16(s)),
                            OpGrp3::DIV => Ok(IrOp::DIV_MwA16(s)),
                            OpGrp3::IDIV => Ok(IrOp::IDIV_MwA16(s)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp3::from_idx3(c);
                        match sub_op {
                            OpGrp3::TEST | OpGrp3::_TEST1 => {
                                let imm = is.fetch_u16()?;
                                Ok(IrOp::TEST_MwA32_Iw(s, imm))
                            }
                            OpGrp3::NOT => Ok(IrOp::NOT_MwA32(s)),
                            OpGrp3::NEG => Ok(IrOp::NEG_MwA32(s)),
                            OpGrp3::MUL => Ok(IrOp::MUL_MwA32(s)),
                            OpGrp3::IMUL => Ok(IrOp::IMUL_MwA32(s)),
                            OpGrp3::DIV => Ok(IrOp::DIV_MwA32(s)),
                            OpGrp3::IDIV => Ok(IrOp::IDIV_MwA32(s)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp3::from_idx3(c);
                        match sub_op {
                            OpGrp3::TEST | OpGrp3::_TEST1 => {
                                let imm = is.fetch_u32()?;
                                Ok(IrOp::TEST_Rd_Id(s, imm))
                            }
                            OpGrp3::NOT => Ok(IrOp::NOT_Rd(s)),
                            OpGrp3::NEG => Ok(IrOp::NEG_Rd(s)),
                            OpGrp3::MUL => Ok(IrOp::MUL_Rd(s)),
                            OpGrp3::IMUL => Ok(IrOp::IMUL_Rd(s)),
                            OpGrp3::DIV => Ok(IrOp::DIV_Rd(s)),
                            OpGrp3::IDIV => Ok(IrOp::IDIV_Rd(s)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp3::from_idx3(c);
                        match sub_op {
                            OpGrp3::TEST | OpGrp3::_TEST1 => {
                                let imm = is.fetch_u32()?;
                                Ok(IrOp::TEST_MdA16_Id(s, imm))
                            }
                            OpGrp3::NOT => Ok(IrOp::NOT_MdA16(s)),
                            OpGrp3::NEG => Ok(IrOp::NEG_MdA16(s)),
                            OpGrp3::MUL => Ok(IrOp::MUL_MdA16(s)),
                            OpGrp3::IMUL => Ok(IrOp::IMUL_MdA16(s)),
                            OpGrp3::DIV => Ok(IrOp::DIV_MdA16(s)),
                            OpGrp3::IDIV => Ok(IrOp::IDIV_MdA16(s)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp3::from_idx3(c);
                        match sub_op {
                            OpGrp3::TEST | OpGrp3::_TEST1 => {
                                let imm = is.fetch_u32()?;
                                Ok(IrOp::TEST_MdA32_Id(s, imm))
                            }
                            OpGrp3::NOT => Ok(IrOp::NOT_MdA32(s)),
                            OpGrp3::NEG => Ok(IrOp::NEG_MdA32(s)),
                            OpGrp3::MUL => Ok(IrOp::MUL_MdA32(s)),
                            OpGrp3::IMUL => Ok(IrOp::IMUL_MdA32(s)),
                            OpGrp3::DIV => Ok(IrOp::DIV_MdA32(s)),
                            OpGrp3::IDIV => Ok(IrOp::IDIV_MdA32(s)),
                        }
                    },
                )
            }
            0xf8 => {
                // CLC
                Ok(IrOp::CLC)
            }
            0xf9 => {
                // STC
                Ok(IrOp::STC)
            }
            0xfa => {
                // CLI
                Ok(IrOp::CLI)
            }
            0xfb => {
                // STI
                Ok(IrOp::STI)
            }
            0xfc => {
                // CLD
                Ok(IrOp::CLD)
            }
            0xfd => {
                // STD
                Ok(IrOp::STD)
            }
            0xfe => {
                // Grp4 r/m8
                context.modrm_rb_sub3(
                    is,
                    |_, s, c| {
                        let sub_op = OpGrp4::from_idx3(c);
                        match sub_op {
                            OpGrp4::INC => Ok(IrOp::INC_Rb(s)),
                            OpGrp4::DEC => Ok(IrOp::DEC_Rb(s)),
                            _ => Ok(IrOp::UD),
                        }
                    },
                    |_, s, c| {
                        let sub_op = OpGrp4::from_idx3(c);
                        match sub_op {
                            OpGrp4::INC => Ok(IrOp::INC_MbA16(s)),
                            OpGrp4::DEC => Ok(IrOp::DEC_MbA16(s)),
                            _ => Ok(IrOp::UD),
                        }
                    },
                    |_, s, c| {
                        let sub_op = OpGrp4::from_idx3(c);
                        match sub_op {
                            OpGrp4::INC => Ok(IrOp::INC_MbA32(s)),
                            OpGrp4::DEC => Ok(IrOp::DEC_MbA32(s)),
                            _ => Ok(IrOp::UD),
                        }
                    },
                )
            }
            0xff => {
                // Grp5 r/m16/32
                context.modrm_rv_sub3(
                    is,
                    |_, s, c| {
                        let sub_op = OpGrp5::from_idx3(c);
                        match sub_op {
                            OpGrp5::INC => Ok(IrOp::INC_Rw(s)),
                            OpGrp5::DEC => Ok(IrOp::DEC_Rw(s)),
                            OpGrp5::CALL => Ok(IrOp::CALL_Rw(s)),
                            OpGrp5::JMP => Ok(IrOp::JMP_Rw(s)),
                            OpGrp5::PUSH => Ok(IrOp::PUSH_Rw(s)),
                            _ => Ok(IrOp::UD),
                        }
                    },
                    |_, s, c| {
                        let sub_op = OpGrp5::from_idx3(c);
                        match sub_op {
                            OpGrp5::INC => Ok(IrOp::INC_MwA16(s)),
                            OpGrp5::DEC => Ok(IrOp::DEC_MwA16(s)),
                            OpGrp5::CALL => Ok(IrOp::CALL_MwA16(s)),
                            OpGrp5::CALLF => Ok(IrOp::CALLF_MpA16O16(s)),
                            OpGrp5::JMP => Ok(IrOp::JMP_MwA16(s)),
                            OpGrp5::JMPF => Ok(IrOp::JMPF_MpA16O16(s)),
                            OpGrp5::PUSH => Ok(IrOp::PUSH_MwA16(s)),
                            OpGrp5::_UD7 => Ok(IrOp::UD),
                        }
                    },
                    |_, s, c| {
                        let sub_op = OpGrp5::from_idx3(c);
                        match sub_op {
                            OpGrp5::INC => Ok(IrOp::INC_MwA32(s)),
                            OpGrp5::DEC => Ok(IrOp::DEC_MwA32(s)),
                            OpGrp5::CALL => Ok(IrOp::CALL_MwA32(s)),
                            OpGrp5::CALLF => Ok(IrOp::CALLF_MpA32O16(s)),
                            OpGrp5::JMP => Ok(IrOp::JMP_MwA32(s)),
                            OpGrp5::JMPF => Ok(IrOp::JMPF_MpA32O16(s)),
                            OpGrp5::PUSH => Ok(IrOp::PUSH_MwA32(s)),
                            OpGrp5::_UD7 => Ok(IrOp::UD),
                        }
                    },
                    |_, s, c| {
                        let sub_op = OpGrp5::from_idx3(c);
                        match sub_op {
                            OpGrp5::INC => Ok(IrOp::INC_Rd(s)),
                            OpGrp5::DEC => Ok(IrOp::DEC_Rd(s)),
                            OpGrp5::CALL => Ok(IrOp::CALL_Rd(s)),
                            OpGrp5::JMP => Ok(IrOp::JMP_Rd(s)),
                            OpGrp5::PUSH => Ok(IrOp::PUSH_Rd(s)),
                            _ => Ok(IrOp::UD),
                        }
                    },
                    |_, s, c| {
                        let sub_op = OpGrp5::from_idx3(c);
                        match sub_op {
                            OpGrp5::INC => Ok(IrOp::INC_MdA16(s)),
                            OpGrp5::DEC => Ok(IrOp::DEC_MdA16(s)),
                            OpGrp5::CALL => Ok(IrOp::CALL_MdA16(s)),
                            OpGrp5::CALLF => Ok(IrOp::CALLF_MpA16O32(s)),
                            OpGrp5::JMP => Ok(IrOp::JMP_MdA16(s)),
                            OpGrp5::JMPF => Ok(IrOp::JMPF_MpA16O32(s)),
                            OpGrp5::PUSH => Ok(IrOp::PUSH_MdA16(s)),
                            OpGrp5::_UD7 => Ok(IrOp::UD),
                        }
                    },
                    |_, s, c| {
                        let sub_op = OpGrp5::from_idx3(c);
                        match sub_op {
                            OpGrp5::INC => Ok(IrOp::INC_MdA32(s)),
                            OpGrp5::DEC => Ok(IrOp::DEC_MdA32(s)),
                            OpGrp5::CALL => Ok(IrOp::CALL_MdA32(s)),
                            OpGrp5::CALLF => Ok(IrOp::CALLF_MpA32O32(s)),
                            OpGrp5::JMP => Ok(IrOp::JMP_MdA32(s)),
                            OpGrp5::JMPF => Ok(IrOp::JMPF_MpA32O32(s)),
                            OpGrp5::PUSH => Ok(IrOp::PUSH_MdA32(s)),
                            OpGrp5::_UD7 => Ok(IrOp::UD),
                        }
                    },
                )
            }

            0x0f => {
                // Two-byte opcode escape
                let second_opcode = is.fetch_u8()?;
                match second_opcode {
                    0x00 => {
                        // grp6
                        context.modrm_rw_sub3(
                            is,
                            |_is, s, c| {
                                let sub_op = OpGrp6::from_idx3(c);
                                match sub_op {
                                    OpGrp6::SLDT => Ok(IrOp::SLDT_Rw(s)),
                                    OpGrp6::STR => Ok(IrOp::STR_Rw(s)),
                                    OpGrp6::LLDT => Ok(IrOp::LLDT_Rw(s)),
                                    OpGrp6::LTR => Ok(IrOp::LTR_Rw(s)),
                                    OpGrp6::VERR => Ok(IrOp::VERR_Rw(s)),
                                    OpGrp6::VERW => Ok(IrOp::VERW_Rw(s)),
                                    OpGrp6::_UD => Ok(IrOp::UD),
                                }
                            },
                            |_is, s, c| {
                                let sub_op = OpGrp6::from_idx3(c);
                                match sub_op {
                                    OpGrp6::SLDT => Ok(IrOp::SLDT_MwA16(s)),
                                    OpGrp6::STR => Ok(IrOp::STR_MwA16(s)),
                                    OpGrp6::LLDT => Ok(IrOp::LLDT_MwA16(s)),
                                    OpGrp6::LTR => Ok(IrOp::LTR_MwA16(s)),
                                    OpGrp6::VERR => Ok(IrOp::VERR_MwA16(s)),
                                    OpGrp6::VERW => Ok(IrOp::VERW_MwA16(s)),
                                    OpGrp6::_UD => Ok(IrOp::UD),
                                }
                            },
                            |_is, s, c| {
                                let sub_op = OpGrp6::from_idx3(c);
                                match sub_op {
                                    OpGrp6::SLDT => Ok(IrOp::SLDT_MwA32(s)),
                                    OpGrp6::STR => Ok(IrOp::STR_MwA32(s)),
                                    OpGrp6::LLDT => Ok(IrOp::LLDT_MwA32(s)),
                                    OpGrp6::LTR => Ok(IrOp::LTR_MwA32(s)),
                                    OpGrp6::VERR => Ok(IrOp::VERR_MwA32(s)),
                                    OpGrp6::VERW => Ok(IrOp::VERW_MwA32(s)),
                                    OpGrp6::_UD => Ok(IrOp::UD),
                                }
                            },
                        )
                    }
                    0x01 => {
                        // grp7
                        context.modrm_mw_sub3(
                            is,
                            |_is, modrm| {
                                let sub_op = OpGrp7::from_idx3(modrm.reg_index());
                                let s = GprIndex16::from_idx3(modrm.rm_index());
                                match sub_op {
                                    OpGrp7::SMSW => Ok(IrOp::SMSW_Rw(s)),
                                    OpGrp7::LMSW => Ok(IrOp::LMSW_Rw(s)),
                                    _ => match modrm.raw() {
                                        // 0xc1 => Ok(IrOp::VMCALL),
                                        // 0xc2 => Ok(IrOp::VMLAUNCH),
                                        // 0xc3 => Ok(IrOp::VMRESUME),
                                        // 0xc4 => Ok(IrOp::VMXOFF),
                                        // 0xc8 => Ok(IrOp::MONITOR),
                                        // 0xc9 => Ok(IrOp::MWAIT),
                                        0xd0 => Ok(IrOp::XGETBV),
                                        0xd1 => Ok(IrOp::XSETBV),
                                        0xf8 => Ok(IrOp::SWAPGS),
                                        0xf9 => Ok(IrOp::RDTSCP),
                                        _ => Ok(IrOp::UD),
                                    },
                                }
                            },
                            |_is, s, c| {
                                let sub_op = OpGrp7::from_idx3(c);
                                match sub_op {
                                    OpGrp7::SGDT => Ok(IrOp::SGDT_MpA16(s)),
                                    OpGrp7::SIDT => Ok(IrOp::SIDT_MpA16(s)),
                                    OpGrp7::LGDT => Ok(IrOp::LGDT_MpA16(s)),
                                    OpGrp7::LIDT => Ok(IrOp::LIDT_MpA16(s)),
                                    OpGrp7::SMSW => Ok(IrOp::SMSW_MwA16(s)),
                                    OpGrp7::LMSW => Ok(IrOp::LMSW_MwA16(s)),
                                    OpGrp7::INVLPG => Ok(IrOp::INVLPG_MwA16(s)),
                                    _ => Ok(IrOp::UD),
                                }
                            },
                            |_is, s, c| {
                                let sub_op = OpGrp7::from_idx3(c);
                                match sub_op {
                                    OpGrp7::SGDT => Ok(IrOp::SGDT_MpA32(s)),
                                    OpGrp7::SIDT => Ok(IrOp::SIDT_MpA32(s)),
                                    OpGrp7::LGDT => Ok(IrOp::LGDT_MpA32(s)),
                                    OpGrp7::LIDT => Ok(IrOp::LIDT_MpA32(s)),
                                    OpGrp7::SMSW => Ok(IrOp::SMSW_MwA32(s)),
                                    OpGrp7::LMSW => Ok(IrOp::LMSW_MwA32(s)),
                                    OpGrp7::INVLPG => Ok(IrOp::INVLPG_MwA32(s)),
                                    _ => Ok(IrOp::UD),
                                }
                            },
                        )
                    }

                    0x05 => {
                        // SYSCALL
                        Ok(IrOp::SYSCALL)
                    }
                    0x06 => {
                        // CLTS
                        Ok(IrOp::CLTS)
                    }
                    0x07 => {
                        // SYSRET
                        Ok(IrOp::SYSRET)
                    }
                    0x08 => {
                        // INVD
                        Ok(IrOp::INVD)
                    }
                    0x09 => {
                        // WBINVD
                        Ok(IrOp::WBINVD)
                    }

                    0x0b => {
                        // UD2
                        Ok(IrOp::UD)
                    }

                    0x0d => {
                        // NOP
                        context.modrm_mv(
                            is,
                            |_| IrOp::NOP,
                            |_, _| IrOp::NOP,
                            |_, _| IrOp::NOP,
                            |_, _| IrOp::NOP,
                            |_, _| IrOp::NOP,
                        )
                    }

                    0x18..=0x1f => {
                        // HINT_NOP Ev /?
                        context.modrm_rw_sub3(
                            is,
                            |_, _, _| Ok(IrOp::NOP),
                            |_, _, _| Ok(IrOp::NOP),
                            |_, _, _| Ok(IrOp::NOP),
                        )
                    }

                    0x20 => {
                        // MOV r/m16/32, CR0..CR7
                        let modrm = ModRM::new(is.fetch_u8()?);
                        if let Some(cr) = CrIndex::from_idx3(modrm.reg_index()) {
                            let reg = GprIndex32::from_idx3(modrm.rm_index());
                            Ok(IrOp::MOV_Rd_Cr(reg, cr))
                        } else {
                            Ok(IrOp::UD)
                        }
                    }
                    0x21 => {
                        // MOV CR0..CR7, r/m16/32
                        let modrm = ModRM::new(is.fetch_u8()?);
                        if let Some(cr) = CrIndex::from_idx3(modrm.reg_index()) {
                            let reg = GprIndex32::from_idx3(modrm.rm_index());
                            Ok(IrOp::MOV_Cr_Rd(cr, reg))
                        } else {
                            Ok(IrOp::UD)
                        }
                    }
                    0x22 => {
                        // MOV r/m16/32, DR0..DR7
                        context.modrm_rd_sub3(
                            is,
                            |_, s, c| Ok(IrOp::MOV_Rd_Dr(s, DrIndex::from_idx3(c))),
                            |_, _, _| Ok(IrOp::UD),
                            |_, _, _| Ok(IrOp::UD),
                        )
                    }
                    0x23 => {
                        // MOV DR0..DR7, r/m16/32
                        context.modrm_rd_sub3(
                            is,
                            |_, s, c| Ok(IrOp::MOV_Dr_Rd(DrIndex::from_idx3(c), s)),
                            |_, _, _| Ok(IrOp::UD),
                            |_, _, _| Ok(IrOp::UD),
                        )
                    }

                    0x30 => {
                        // WRMSR
                        Ok(IrOp::WRMSR)
                    }
                    0x31 => {
                        // RDTSC
                        Ok(IrOp::RDTSC)
                    }
                    0x32 => {
                        // RDMSR
                        Ok(IrOp::RDMSR)
                    }

                    0x34 => {
                        // SYSENTER
                        Ok(IrOp::SYSENTER)
                    }
                    0x35 => {
                        // SYSEXIT
                        Ok(IrOp::SYSEXIT)
                    }

                    0x38 => {
                        // TODO: 3-byte opcode escape (0x0f 0x38)
                        Ok(IrOp::UD)
                    }

                    0x3a => {
                        // TODO: 3-byte opcode escape (0x0f 0x3a)
                        Ok(IrOp::UD)
                    }

                    0x40..=0x4f => {
                        // CMOVcc r16/32, r/m16/32
                        let cc = CC::from_u8(second_opcode);
                        context.modrm_rv(
                            is,
                            |m, r| IrOp::CMOV_Rw_Rw(cc, r, m),
                            |m, r| IrOp::CMOV_Rw_MwA16(cc, r, m),
                            |m, r| IrOp::CMOV_Rw_MwA32(cc, r, m),
                            |m, r| IrOp::CMOV_Rd_Rd(cc, r, m),
                            |m, r| IrOp::CMOV_Rd_MdA16(cc, r, m),
                            |m, r| IrOp::CMOV_Rd_MdA32(cc, r, m),
                        )
                    }

                    0x80..=0x8f => {
                        // Jcc rel16/32
                        let cc = CC::from_u8(second_opcode);
                        let target = if context.is_opsize32 {
                            let rel = is.fetch_i32()?;
                            Offset32(is.current_eip().0.wrapping_add(rel as u32))
                        } else {
                            let rel = is.fetch_i16()?;
                            Offset32(is.current_eip().0.wrapping_add(rel as u32) & 0x0000_ffff)
                        };
                        Ok(IrOp::JCC_Jv(cc, target))
                    }

                    0x90..=0x9f => {
                        // SETcc r/m8
                        let cc = CC::from_u8(second_opcode);
                        context.modrm_rb_sub3(
                            is,
                            |_, s, c| match c {
                                Index3::I000 => Ok(IrOp::SETCC_Rb(cc, s)),
                                _ => Ok(IrOp::UD),
                            },
                            |_, s, c| match c {
                                Index3::I000 => Ok(IrOp::SETCC_MbA16(cc, s)),
                                _ => Ok(IrOp::UD),
                            },
                            |_, s, c| match c {
                                Index3::I000 => Ok(IrOp::SETCC_MbA32(cc, s)),
                                _ => Ok(IrOp::UD),
                            },
                        )
                    }

                    0xa0 => {
                        // PUSH FS
                        Ok(IrOp::PUSH_Sr(FS))
                    }
                    0xa1 => {
                        // POP FS
                        Ok(IrOp::POP_Sr(FS))
                    }
                    0xa2 => {
                        // CPUID
                        Ok(IrOp::CPUID)
                    }
                    0xa3 => {
                        // BT r/m16/32, r16/32
                        context.modrm_rv(
                            is,
                            IrOp::BT_Rw_Rw,
                            IrOp::BT_MwA16_Rw,
                            IrOp::BT_MwA32_Rw,
                            IrOp::BT_Rd_Rd,
                            IrOp::BT_MdA16_Rd,
                            IrOp::BT_MdA32_Rd,
                        )
                    }
                    0xa4 => {
                        // SHLD r/m16/32, r16/32, imm8
                        context.modrm_rv_ib(
                            is,
                            IrOp::SHLD_Rw_Rw_Ib,
                            IrOp::SHLD_MwA16_Rw_Ib,
                            IrOp::SHLD_MwA32_Rw_Ib,
                            IrOp::SHLD_Rd_Rd_Ib,
                            IrOp::SHLD_MdA16_Rd_Ib,
                            IrOp::SHLD_MdA32_Rd_Ib,
                        )
                    }
                    0xa5 => {
                        // SHLD r/m16/32, r16/32, cl
                        context.modrm_rv(
                            is,
                            IrOp::SHLD_Rw_Rw_Cl,
                            IrOp::SHLD_MwA16_Rw_Cl,
                            IrOp::SHLD_MwA32_Rw_Cl,
                            IrOp::SHLD_Rd_Rd_Cl,
                            IrOp::SHLD_MdA16_Rd_Cl,
                            IrOp::SHLD_MdA32_Rd_Cl,
                        )
                    }
                    0xa8 => {
                        // PUSH GS
                        Ok(IrOp::PUSH_Sr(GS))
                    }
                    0xa9 => {
                        // POP GS
                        Ok(IrOp::POP_Sr(GS))
                    }
                    0xaa => {
                        // RSM
                        Ok(IrOp::RSM)
                    }
                    0xab => {
                        // BTS r/m16/32, r16/32
                        context.modrm_rv(
                            is,
                            IrOp::BTS_Rw_Rw,
                            IrOp::BTS_MwA16_Rw,
                            IrOp::BTS_MwA32_Rw,
                            IrOp::BTS_Rd_Rd,
                            IrOp::BTS_MdA16_Rd,
                            IrOp::BTS_MdA32_Rd,
                        )
                    }
                    0xac => {
                        // SHRD r/m16/32, r16/32, imm8
                        context.modrm_rv_ib(
                            is,
                            IrOp::SHRD_Rw_Rw_Ib,
                            IrOp::SHRD_MwA16_Rw_Ib,
                            IrOp::SHRD_MwA32_Rw_Ib,
                            IrOp::SHRD_Rd_Rd_Ib,
                            IrOp::SHRD_MdA16_Rd_Ib,
                            IrOp::SHRD_MdA32_Rd_Ib,
                        )
                    }
                    0xad => {
                        // SHRD r/m16/32, r16/32, cl
                        context.modrm_rv(
                            is,
                            IrOp::SHRD_Rw_Rw_Cl,
                            IrOp::SHRD_MwA16_Rw_Cl,
                            IrOp::SHRD_MwA32_Rw_Cl,
                            IrOp::SHRD_Rd_Rd_Cl,
                            IrOp::SHRD_MdA16_Rd_Cl,
                            IrOp::SHRD_MdA32_Rd_Cl,
                        )
                    }
                    // 0xae grp15
                    0xaf => {
                        // IMUL r16/32, r/m16/32
                        context.modrm_rv(
                            is,
                            |m, r| IrOp::IMUL_Rw_Rw(r, m),
                            |m, r| IrOp::IMUL_Rw_MwA16(r, m),
                            |m, r| IrOp::IMUL_Rw_MwA32(r, m),
                            |m, r| IrOp::IMUL_Rd_Rd(r, m),
                            |m, r| IrOp::IMUL_Rd_MdA16(r, m),
                            |m, r| IrOp::IMUL_Rd_MdA32(r, m),
                        )
                    }

                    0xb0 => {
                        // CMPXCHG r/m8, reg8
                        context.modrm_rb(
                            is,
                            IrOp::CMPXCHG_Rb_Rb,
                            IrOp::CMPXCHG_MbA16_Rb,
                            IrOp::CMPXCHG_MbA32_Rb,
                        )
                    }
                    0xb1 => {
                        // CMPXCHG r/m16/32, reg16/32
                        context.modrm_rv(
                            is,
                            IrOp::CMPXCHG_Rw_Rw,
                            IrOp::CMPXCHG_MwA16_Rw,
                            IrOp::CMPXCHG_MwA32_Rw,
                            IrOp::CMPXCHG_Rd_Rd,
                            IrOp::CMPXCHG_MdA16_Rd,
                            IrOp::CMPXCHG_MdA32_Rd,
                        )
                    }
                    0xb2 => {
                        // LSS r16/32, m16:16/32
                        context.modrm_mv(
                            is,
                            |_| IrOp::UD,
                            |m, r| IrOp::LSS_Rw_MwA16(r, m),
                            |m, r| IrOp::LSS_Rw_MwA32(r, m),
                            |m, r| IrOp::LSS_Rd_MdA16(r, m),
                            |m, r| IrOp::LSS_Rd_MdA32(r, m),
                        )
                    }
                    0xb3 => {
                        // BTR r/m16/32, r16/32
                        context.modrm_rv(
                            is,
                            IrOp::BTR_Rw_Rw,
                            IrOp::BTR_MwA16_Rw,
                            IrOp::BTR_MwA32_Rw,
                            IrOp::BTR_Rd_Rd,
                            IrOp::BTR_MdA16_Rd,
                            IrOp::BTR_MdA32_Rd,
                        )
                    }
                    0xb4 => {
                        // LFS r16/32, m16:16/32
                        context.modrm_mv(
                            is,
                            |_| IrOp::UD,
                            |m, r| IrOp::LFS_Rw_MwA16(r, m),
                            |m, r| IrOp::LFS_Rw_MwA32(r, m),
                            |m, r| IrOp::LFS_Rd_MdA16(r, m),
                            |m, r| IrOp::LFS_Rd_MdA32(r, m),
                        )
                    }
                    0xb5 => {
                        // LGS r16/32, m16:16/32
                        context.modrm_mv(
                            is,
                            |_| IrOp::UD,
                            |m, r| IrOp::LGS_Rw_MwA16(r, m),
                            |m, r| IrOp::LGS_Rw_MwA32(r, m),
                            |m, r| IrOp::LGS_Rd_MdA16(r, m),
                            |m, r| IrOp::LGS_Rd_MdA32(r, m),
                        )
                    }
                    0xb6 => {
                        // MOVZX r16/32, r/m8
                        context.modrm_rv(
                            is,
                            |m, r| IrOp::MOVZX_Rw_Rb(r, GprIndex8::from_u8(m as u8)),
                            |m, r| IrOp::MOVZX_Rw_MbA16(r, m),
                            |m, r| IrOp::MOVZX_Rw_MbA32(r, m),
                            |m, r| IrOp::MOVZX_Rd_Rb(r, GprIndex8::from_u8(m as u8)),
                            |m, r| IrOp::MOVZX_Rd_MbA16(r, m),
                            |m, r| IrOp::MOVZX_Rd_MbA32(r, m),
                        )
                    }
                    0xb7 => {
                        // MOVZX r16/32, r/m16
                        context.modrm_rv(
                            is,
                            |m, r| IrOp::MOV_Rw_Rw(r, m),
                            |m, r| IrOp::MOV_Rw_MwA16(r, m),
                            |m, r| IrOp::MOV_Rw_MwA32(r, m),
                            |m, r| IrOp::MOVZX_Rd_Rw(r, m.downgrade()),
                            |m, r| IrOp::MOVZX_Rd_MwA16(r, m),
                            |m, r| IrOp::MOVZX_Rd_MwA32(r, m),
                        )
                    }
                    0xb8 => {
                        if matches!(context.prefix_fx, PrefixFx::F3_REPZ) {
                            // POPCNT r16/32, r/m16/32
                            context.modrm_rv(
                                is,
                                |m, r| IrOp::POPCNT_Rw_Rw(r, m),
                                |m, r| IrOp::POPCNT_Rw_MwA16(r, m),
                                |m, r| IrOp::POPCNT_Rw_MwA32(r, m),
                                |m, r| IrOp::POPCNT_Rd_Rd(r, m),
                                |m, r| IrOp::POPCNT_Rd_MdA16(r, m),
                                |m, r| IrOp::POPCNT_Rd_MdA32(r, m),
                            )
                        } else {
                            // JMPE?
                            Ok(IrOp::UD)
                        }
                    }
                    0xb9 => {
                        // UD r/m/ reg
                        context.modrm_rw_sub3(
                            is,
                            |_, _, _| Ok(IrOp::UD),
                            |_, _, _| Ok(IrOp::UD),
                            |_, _, _| Ok(IrOp::UD),
                        )
                    }
                    0xba => {
                        // grp8 r/m16/32, imm8
                        context.modrm_rv_sub3(
                            is,
                            |is, s, c| {
                                let sub_op = OpGrp8::from_idx3(c);
                                let imm = is.fetch_u8()?;
                                match sub_op {
                                    OpGrp8::BT => Ok(IrOp::BT_Rw_Ib(s, imm)),
                                    OpGrp8::BTS => Ok(IrOp::BTS_Rw_Ib(s, imm)),
                                    OpGrp8::BTR => Ok(IrOp::BTR_Rw_Ib(s, imm)),
                                    OpGrp8::BTC => Ok(IrOp::BTC_Rw_Ib(s, imm)),
                                    _ => Ok(IrOp::UD),
                                }
                            },
                            |is, s, c| {
                                let sub_op = OpGrp8::from_idx3(c);
                                let imm = is.fetch_u8()?;
                                match sub_op {
                                    OpGrp8::BT => Ok(IrOp::BT_MwA16_Ib(s, imm)),
                                    OpGrp8::BTS => Ok(IrOp::BTS_MwA16_Ib(s, imm)),
                                    OpGrp8::BTR => Ok(IrOp::BTR_MwA16_Ib(s, imm)),
                                    OpGrp8::BTC => Ok(IrOp::BTC_MwA16_Ib(s, imm)),
                                    _ => Ok(IrOp::UD),
                                }
                            },
                            |is, s, c| {
                                let sub_op = OpGrp8::from_idx3(c);
                                let imm = is.fetch_u8()?;
                                match sub_op {
                                    OpGrp8::BT => Ok(IrOp::BT_MwA32_Ib(s, imm)),
                                    OpGrp8::BTS => Ok(IrOp::BTS_MwA32_Ib(s, imm)),
                                    OpGrp8::BTR => Ok(IrOp::BTR_MwA32_Ib(s, imm)),
                                    OpGrp8::BTC => Ok(IrOp::BTC_MwA32_Ib(s, imm)),
                                    _ => Ok(IrOp::UD),
                                }
                            },
                            |is, s, c| {
                                let sub_op = OpGrp8::from_idx3(c);
                                let imm = is.fetch_u8()?;
                                match sub_op {
                                    OpGrp8::BT => Ok(IrOp::BT_Rd_Ib(s, imm)),
                                    OpGrp8::BTS => Ok(IrOp::BTS_Rd_Ib(s, imm)),
                                    OpGrp8::BTR => Ok(IrOp::BTR_Rd_Ib(s, imm)),
                                    OpGrp8::BTC => Ok(IrOp::BTC_Rd_Ib(s, imm)),
                                    _ => Ok(IrOp::UD),
                                }
                            },
                            |is, s, c| {
                                let sub_op = OpGrp8::from_idx3(c);
                                let imm = is.fetch_u8()?;
                                match sub_op {
                                    OpGrp8::BT => Ok(IrOp::BT_MdA16_Ib(s, imm)),
                                    OpGrp8::BTS => Ok(IrOp::BTS_MdA16_Ib(s, imm)),
                                    OpGrp8::BTR => Ok(IrOp::BTR_MdA16_Ib(s, imm)),
                                    OpGrp8::BTC => Ok(IrOp::BTC_MdA16_Ib(s, imm)),
                                    _ => Ok(IrOp::UD),
                                }
                            },
                            |is, s, c| {
                                let sub_op = OpGrp8::from_idx3(c);
                                let imm = is.fetch_u8()?;
                                match sub_op {
                                    OpGrp8::BT => Ok(IrOp::BT_MdA32_Ib(s, imm)),
                                    OpGrp8::BTS => Ok(IrOp::BTS_MdA32_Ib(s, imm)),
                                    OpGrp8::BTR => Ok(IrOp::BTR_MdA32_Ib(s, imm)),
                                    OpGrp8::BTC => Ok(IrOp::BTC_MdA32_Ib(s, imm)),
                                    _ => Ok(IrOp::UD),
                                }
                            },
                        )
                    }
                    0xbb => {
                        // BTC r/m16/32, r16/32
                        context.modrm_rv(
                            is,
                            IrOp::BTC_Rw_Rw,
                            IrOp::BTC_MwA16_Rw,
                            IrOp::BTC_MwA32_Rw,
                            IrOp::BTC_Rd_Rd,
                            IrOp::BTC_MdA16_Rd,
                            IrOp::BTC_MdA32_Rd,
                        )
                    }
                    0xbc => {
                        // BSF r16/32, r/m16/32
                        context.modrm_rv(
                            is,
                            |m, r| IrOp::BSF_Rw_Rw(r, m),
                            |m, r| IrOp::BSF_Rw_MwA16(r, m),
                            |m, r| IrOp::BSF_Rw_MwA32(r, m),
                            |m, r| IrOp::BSF_Rd_Rd(r, m),
                            |m, r| IrOp::BSF_Rd_MdA16(r, m),
                            |m, r| IrOp::BSF_Rd_MdA32(r, m),
                        )
                    }
                    0xbd => {
                        // BSR r16/32, r/m16/32
                        context.modrm_rv(
                            is,
                            |m, r| IrOp::BSR_Rw_Rw(r, m),
                            |m, r| IrOp::BSR_Rw_MwA16(r, m),
                            |m, r| IrOp::BSR_Rw_MwA32(r, m),
                            |m, r| IrOp::BSR_Rd_Rd(r, m),
                            |m, r| IrOp::BSR_Rd_MdA16(r, m),
                            |m, r| IrOp::BSR_Rd_MdA32(r, m),
                        )
                    }
                    0xbe => {
                        // MOVSX r16/32, r/m8
                        context.modrm_rv(
                            is,
                            |m, r| IrOp::MOVSX_Rw_Rb(r, GprIndex8::from_u8(m as u8)),
                            |m, r| IrOp::MOVSX_Rw_MbA16(r, m),
                            |m, r| IrOp::MOVSX_Rw_MbA32(r, m),
                            |m, r| IrOp::MOVSX_Rd_Rb(r, GprIndex8::from_u8(m as u8)),
                            |m, r| IrOp::MOVSX_Rd_MbA16(r, m),
                            |m, r| IrOp::MOVSX_Rd_MbA32(r, m),
                        )
                    }
                    0xbf => {
                        // MOVSX r16/32, r/m16
                        context.modrm_rv(
                            is,
                            |m, r| IrOp::MOV_Rw_Rw(r, m),
                            |m, r| IrOp::MOV_Rw_MwA16(r, m),
                            |m, r| IrOp::MOV_Rw_MwA32(r, m),
                            |m, r| IrOp::MOVSX_Rd_Rw(r, m.downgrade()),
                            |m, r| IrOp::MOVSX_Rd_MwA16(r, m),
                            |m, r| IrOp::MOVSX_Rd_MwA32(r, m),
                        )
                    }

                    0xc0 => {
                        // XADD r/m8, r8
                        context.modrm_rb(
                            is,
                            IrOp::XADD_Rb_Rb,
                            IrOp::XADD_MbA16_Rb,
                            IrOp::XADD_MbA32_Rb,
                        )
                    }
                    0xc1 => {
                        // XADD r/m16/32, r16/32
                        context.modrm_rv(
                            is,
                            IrOp::XADD_Rw_Rw,
                            IrOp::XADD_MwA16_Rw,
                            IrOp::XADD_MwA32_Rw,
                            IrOp::XADD_Rd_Rd,
                            IrOp::XADD_MdA16_Rd,
                            IrOp::XADD_MdA32_Rd,
                        )
                    }

                    0xc8..=0xcf => {
                        // BSWAP r32
                        let reg = GprIndex32::from_u8(second_opcode);
                        Ok(IrOp::BSWAP_Rd(reg))
                    }

                    _ => {
                        // unimplemented!(
                        //     "Two-byte opcode {:#x} {:#x} is not implemented yet",
                        //     opcode,
                        //     second_opcode
                        // );
                        Ok(IrOp::UD)
                    }
                }
            }

            _ => {
                // unimplemented!("Opcode {:#x} is not implemented yet", opcode);
                Ok(IrOp::UD)
            }
        }
    }
}

impl DecoderContext {
    #[inline]
    pub const fn new() -> Self {
        Self {
            segment_override: None,
            is_opsize32: false,
            is_addr32: false,
            prefix_fx: PrefixFx::NP,
        }
    }

    /// Decodes instructions with ModR/M byte and 8-bit register operand.
    #[inline]
    pub fn modrm_rb<E>(
        &self,
        is: &mut impl Fetch<E = E>,
        ir_rb: impl Fn(GprIndex8, GprIndex8) -> IrOp,
        ir_m16_rb: impl Fn(MemOpr16, GprIndex8) -> IrOp,
        ir_m32_rb: impl Fn(MemOpr32, GprIndex8) -> IrOp,
    ) -> Result<IrOp, E> {
        self.modrm_rb_sub3(
            is,
            |_, s, c| Ok(ir_rb(s, GprIndex8::from_idx3(c))),
            |_, s, c| Ok(ir_m16_rb(s, GprIndex8::from_idx3(c))),
            |_, s, c| Ok(ir_m32_rb(s, GprIndex8::from_idx3(c))),
        )
    }

    /// Decodes instructions with ModR/M byte and reg field indicates a subcode
    #[inline]
    pub fn modrm_rb_sub3<IS, E>(
        &self,
        is: &mut IS,
        ir_rb: impl Fn(&mut IS, GprIndex8, Index3) -> Result<IrOp, E>,
        ir_m16_rb: impl Fn(&mut IS, MemOpr16, Index3) -> Result<IrOp, E>,
        ir_m32_rb: impl Fn(&mut IS, MemOpr32, Index3) -> Result<IrOp, E>,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        if self.is_addr32 {
            let modrm = ModRm32::fetch(is, self.segment_override)?;
            match modrm.reg_or_mem() {
                RegOrMem::Reg(index) => ir_rb(is, GprIndex8::from_idx3(index), modrm.reg_index()),
                RegOrMem::Mem(mem) => ir_m32_rb(is, mem, modrm.reg_index()),
            }
        } else {
            let modrm = ModRm16::fetch(is, self.segment_override)?;
            match modrm.reg_or_mem() {
                RegOrMem::Reg(index) => ir_rb(is, GprIndex8::from_idx3(index), modrm.reg_index()),
                RegOrMem::Mem(mem) => ir_m16_rb(is, mem, modrm.reg_index()),
            }
        }
    }

    /// Decodes instructions with ModR/M byte and register operand whose size depends on the operand-size override prefix.
    #[inline]
    pub fn modrm_rv<E>(
        &self,
        is: &mut impl Fetch<E = E>,
        ir_rw: impl Fn(GprIndex16, GprIndex16) -> IrOp,
        ir_m16_rw: impl Fn(MemOpr16, GprIndex16) -> IrOp,
        ir_m32_rw: impl Fn(MemOpr32, GprIndex16) -> IrOp,
        ir_rd: impl Fn(GprIndex32, GprIndex32) -> IrOp,
        ir_m16_rd: impl Fn(MemOpr16, GprIndex32) -> IrOp,
        ir_m32_rd: impl Fn(MemOpr32, GprIndex32) -> IrOp,
    ) -> Result<IrOp, E> {
        self.modrm_rv_sub3(
            is,
            |_, s, c| Ok(ir_rw(s, GprIndex16::from_idx3(c))),
            |_, s, c| Ok(ir_m16_rw(s, GprIndex16::from_idx3(c))),
            |_, s, c| Ok(ir_m32_rw(s, GprIndex16::from_idx3(c))),
            |_, s, c| Ok(ir_rd(s, GprIndex32::from_idx3(c))),
            |_, s, c| Ok(ir_m16_rd(s, GprIndex32::from_idx3(c))),
            |_, s, c| Ok(ir_m32_rd(s, GprIndex32::from_idx3(c))),
        )
    }

    /// Decodes instructions with ModR/M byte and register operand whose size depends on the operand-size override prefix, and an immediate 8-bit value.
    #[inline]
    pub fn modrm_rv_ib<IS, E>(
        &self,
        is: &mut IS,
        ir_rw: impl Fn(GprIndex16, GprIndex16, u8) -> IrOp,
        ir_m16_rw: impl Fn(MemOpr16, GprIndex16, u8) -> IrOp,
        ir_m32_rw: impl Fn(MemOpr32, GprIndex16, u8) -> IrOp,
        ir_rd: impl Fn(GprIndex32, GprIndex32, u8) -> IrOp,
        ir_m16_rd: impl Fn(MemOpr16, GprIndex32, u8) -> IrOp,
        ir_m32_rd: impl Fn(MemOpr32, GprIndex32, u8) -> IrOp,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        self.modrm_rv_sub3(
            is,
            |is, s, c| {
                let imm = is.fetch_u8()?;
                Ok(ir_rw(s, GprIndex16::from_idx3(c), imm))
            },
            |is, s, c| {
                let imm = is.fetch_u8()?;
                Ok(ir_m16_rw(s, GprIndex16::from_idx3(c), imm))
            },
            |is, s, c| {
                let imm = is.fetch_u8()?;
                Ok(ir_m32_rw(s, GprIndex16::from_idx3(c), imm))
            },
            |is, s, c| {
                let imm = is.fetch_u8()?;
                Ok(ir_rd(s, GprIndex32::from_idx3(c), imm))
            },
            |is, s, c| {
                let imm = is.fetch_u8()?;
                Ok(ir_m16_rd(s, GprIndex32::from_idx3(c), imm))
            },
            |is, s, c| {
                let imm = is.fetch_u8()?;
                Ok(ir_m32_rd(s, GprIndex32::from_idx3(c), imm))
            },
        )
    }

    /// Decodes instructions with ModR/M byte and register operand whose size depends on the operand-size override prefix, and reg field indicates a subcode
    #[inline]
    pub fn modrm_rv_sub3<IS, E>(
        &self,
        is: &mut IS,
        ir_rw: impl Fn(&mut IS, GprIndex16, Index3) -> Result<IrOp, E>,
        ir_m16_rw: impl Fn(&mut IS, MemOpr16, Index3) -> Result<IrOp, E>,
        ir_m32_rw: impl Fn(&mut IS, MemOpr32, Index3) -> Result<IrOp, E>,
        ir_rd: impl Fn(&mut IS, GprIndex32, Index3) -> Result<IrOp, E>,
        ir_m16_rd: impl Fn(&mut IS, MemOpr16, Index3) -> Result<IrOp, E>,
        ir_m32_rd: impl Fn(&mut IS, MemOpr32, Index3) -> Result<IrOp, E>,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        self.modrm_mv_sub3(
            is,
            |s, modrm| {
                let reg_index = modrm.reg_index();
                let rm = modrm.rm_index();
                if self.is_opsize32 {
                    ir_rd(s, GprIndex32::from_idx3(rm), reg_index)
                } else {
                    ir_rw(s, GprIndex16::from_idx3(rm), reg_index)
                }
            },
            ir_m16_rw,
            ir_m32_rw,
            ir_m16_rd,
            ir_m32_rd,
        )
    }

    /// Decodes instructions with ModR/M byte and 16-bit register operand.
    #[inline]
    pub fn modrm_rw<IS, E>(
        &self,
        is: &mut IS,
        ir_rw: impl Fn(GprIndex16, GprIndex16) -> IrOp,
        ir_m16_rw: impl Fn(MemOpr16, GprIndex16) -> IrOp,
        ir_m32_rw: impl Fn(MemOpr32, GprIndex16) -> IrOp,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        self.modrm_rw_sub3(
            is,
            |_, s, c| Ok(ir_rw(s, GprIndex16::from_idx3(c))),
            |_, s, c| Ok(ir_m16_rw(s, GprIndex16::from_idx3(c))),
            |_, s, c| Ok(ir_m32_rw(s, GprIndex16::from_idx3(c))),
        )
    }

    /// Decodes instructions with ModR/M byte and 16-bit register operand, and reg field indicates a subcode
    #[inline]
    pub fn modrm_rw_sub3<IS, E>(
        &self,
        is: &mut IS,
        ir_rw: impl Fn(&mut IS, GprIndex16, Index3) -> Result<IrOp, E>,
        ir_m16_rw: impl Fn(&mut IS, MemOpr16, Index3) -> Result<IrOp, E>,
        ir_m32_rw: impl Fn(&mut IS, MemOpr32, Index3) -> Result<IrOp, E>,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        self.modrm_mw_sub3(
            is,
            |is, modrm| {
                let reg_index = modrm.reg_index();
                let rm = modrm.rm_index();
                ir_rw(is, GprIndex16::from_idx3(rm), reg_index)
            },
            ir_m16_rw,
            ir_m32_rw,
        )
    }

    /// Decodes instructions with ModR/M byte and 32-bit register operand, and reg field indicates a subcode
    #[inline]
    pub fn modrm_rd_sub3<IS, E>(
        &self,
        is: &mut IS,
        ir_rd: impl Fn(&mut IS, GprIndex32, Index3) -> Result<IrOp, E>,
        ir_m16_rd: impl Fn(&mut IS, MemOpr16, Index3) -> Result<IrOp, E>,
        ir_m32_rd: impl Fn(&mut IS, MemOpr32, Index3) -> Result<IrOp, E>,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        self.modrm_mw_sub3(
            is,
            |is, modrm| {
                let reg_index = modrm.reg_index();
                let rm = modrm.rm_index();
                ir_rd(is, GprIndex32::from_idx3(rm), reg_index)
            },
            ir_m16_rd,
            ir_m32_rd,
        )
    }

    /// Decodes instructions with ModR/M byte and memory operand whose size depends on the operand-size override prefix
    #[inline]
    pub fn modrm_mv<IS, E>(
        &self,
        is: &mut IS,
        ir_rv: impl Fn(ModRM) -> IrOp,
        ir_m16_rw: impl Fn(MemOpr16, GprIndex16) -> IrOp,
        ir_m32_rw: impl Fn(MemOpr32, GprIndex16) -> IrOp,
        ir_m16_rd: impl Fn(MemOpr16, GprIndex32) -> IrOp,
        ir_m32_rd: impl Fn(MemOpr32, GprIndex32) -> IrOp,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        self.modrm_mv_sub3(
            is,
            |_, modrm| Ok(ir_rv(modrm)),
            |_, s, c| Ok(ir_m16_rw(s, GprIndex16::from_idx3(c))),
            |_, s, c| Ok(ir_m32_rw(s, GprIndex16::from_idx3(c))),
            |_, s, c| Ok(ir_m16_rd(s, GprIndex32::from_idx3(c))),
            |_, s, c| Ok(ir_m32_rd(s, GprIndex32::from_idx3(c))),
        )
    }

    /// Decodes instructions with ModR/M byte and memory operand whose size depends on the operand-size override prefix, and reg field indicates a subcode
    #[inline]
    pub fn modrm_mv_sub3<IS, E>(
        &self,
        is: &mut IS,
        ir_rv: impl Fn(&mut IS, ModRM) -> Result<IrOp, E>,
        ir_m16_rw: impl Fn(&mut IS, MemOpr16, Index3) -> Result<IrOp, E>,
        ir_m32_rw: impl Fn(&mut IS, MemOpr32, Index3) -> Result<IrOp, E>,
        ir_m16_rd: impl Fn(&mut IS, MemOpr16, Index3) -> Result<IrOp, E>,
        ir_m32_rd: impl Fn(&mut IS, MemOpr32, Index3) -> Result<IrOp, E>,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        self.modrm_mw_sub3(
            is,
            ir_rv,
            |is, mem, index| {
                if self.is_opsize32 {
                    ir_m16_rd(is, mem, index)
                } else {
                    ir_m16_rw(is, mem, index)
                }
            },
            |is, mem, index| {
                if self.is_opsize32 {
                    ir_m32_rd(is, mem, index)
                } else {
                    ir_m32_rw(is, mem, index)
                }
            },
        )
    }

    /// Decodes instructions with ModR/M byte and 16bit memory operand.
    #[inline]
    pub fn modrm_mw_sub3<IS, E>(
        &self,
        is: &mut IS,
        ir_rw: impl Fn(&mut IS, ModRM) -> Result<IrOp, E>,
        ir_m16_rw: impl Fn(&mut IS, MemOpr16, Index3) -> Result<IrOp, E>,
        ir_m32_rw: impl Fn(&mut IS, MemOpr32, Index3) -> Result<IrOp, E>,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        if self.is_addr32 {
            let modrm = ModRm32::fetch(is, self.segment_override)?;
            match modrm.reg_or_mem() {
                RegOrMem::Reg(_) => ir_rw(is, modrm.raw_modrm()),
                RegOrMem::Mem(mem) => ir_m32_rw(is, mem, modrm.reg_index()),
            }
        } else {
            let modrm = ModRm16::fetch(is, self.segment_override)?;
            match modrm.reg_or_mem() {
                RegOrMem::Reg(_) => ir_rw(is, modrm.raw_modrm()),
                RegOrMem::Mem(mem) => ir_m16_rw(is, mem, modrm.reg_index()),
            }
        }
    }
}
