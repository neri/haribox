//! Encoding Rules for x86 Instructions

use crate::_prelude_::*;
use crate::encoding::sib::SibIndex;

pub mod sib;

/// 3-bit index used in ModR/M and SIB bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Index3 {
    I000 = 0,
    I001 = 1,
    I010 = 2,
    I011 = 3,
    I100 = 4,
    I101 = 5,
    I110 = 6,
    I111 = 7,
}

impl Index3 {
    #[inline]
    pub const fn from_u8(value: u8) -> Self {
        match value & 0b111 {
            0 => Self::I000,
            1 => Self::I001,
            2 => Self::I010,
            3 => Self::I011,
            4 => Self::I100,
            5 => Self::I101,
            6 => Self::I110,
            7 => Self::I111,
            _ => unreachable!(),
        }
    }
}

/// General Purpose Register index for 32bit registers (EAX, ECX, EDX, EBX, ESP, EBP, ESI, EDI)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GprIndex32 {
    EAX = 0,
    ECX = 1,
    EDX = 2,
    EBX = 3,
    ESP = 4,
    EBP = 5,
    ESI = 6,
    EDI = 7,
}

impl GprIndex32 {
    #[inline]
    pub const fn from_u8(value: u8) -> Self {
        Self::from_idx3(Index3::from_u8(value))
    }

    #[inline]
    pub const fn from_idx3(index: Index3) -> Self {
        match index {
            Index3::I000 => GprIndex32::EAX,
            Index3::I001 => GprIndex32::ECX,
            Index3::I010 => GprIndex32::EDX,
            Index3::I011 => GprIndex32::EBX,
            Index3::I100 => GprIndex32::ESP,
            Index3::I101 => GprIndex32::EBP,
            Index3::I110 => GprIndex32::ESI,
            Index3::I111 => GprIndex32::EDI,
        }
    }

    /// Downgrades a 32-bit GPR index to its corresponding 16-bit GPR index.
    #[inline]
    pub const fn downgrade(&self) -> GprIndex16 {
        match self {
            GprIndex32::EAX => GprIndex16::AX,
            GprIndex32::ECX => GprIndex16::CX,
            GprIndex32::EDX => GprIndex16::DX,
            GprIndex32::EBX => GprIndex16::BX,
            GprIndex32::ESP => GprIndex16::SP,
            GprIndex32::EBP => GprIndex16::BP,
            GprIndex32::ESI => GprIndex16::SI,
            GprIndex32::EDI => GprIndex16::DI,
        }
    }
}

/// General Purpose Register index for 16bit registers (AX, CX, DX, BX, SP, BP, SI, DI)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GprIndex16 {
    AX = 0,
    CX = 1,
    DX = 2,
    BX = 3,
    SP = 4,
    BP = 5,
    SI = 6,
    DI = 7,
}

impl GprIndex16 {
    #[inline]
    pub const fn from_u8(value: u8) -> Self {
        Self::from_idx3(Index3::from_u8(value))
    }

    #[inline]
    pub const fn from_idx3(index: Index3) -> Self {
        match index {
            Index3::I000 => GprIndex16::AX,
            Index3::I001 => GprIndex16::CX,
            Index3::I010 => GprIndex16::DX,
            Index3::I011 => GprIndex16::BX,
            Index3::I100 => GprIndex16::SP,
            Index3::I101 => GprIndex16::BP,
            Index3::I110 => GprIndex16::SI,
            Index3::I111 => GprIndex16::DI,
        }
    }

    /// Upgrades a 16-bit GPR index to its corresponding 32-bit GPR index.
    #[inline]
    pub const fn upgrade(&self) -> GprIndex32 {
        match self {
            GprIndex16::AX => GprIndex32::EAX,
            GprIndex16::CX => GprIndex32::ECX,
            GprIndex16::DX => GprIndex32::EDX,
            GprIndex16::BX => GprIndex32::EBX,
            GprIndex16::SP => GprIndex32::ESP,
            GprIndex16::BP => GprIndex32::EBP,
            GprIndex16::SI => GprIndex32::ESI,
            GprIndex16::DI => GprIndex32::EDI,
        }
    }
}

/// General Purpose Register index for 8-bit partial registers (AL, CL, DL, BL, AH, CH, DH, BH)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GprIndex8 {
    AL = 0,
    CL = 1,
    DL = 2,
    BL = 3,
    AH = 4,
    CH = 5,
    DH = 6,
    BH = 7,
}

impl GprIndex8 {
    #[inline]
    pub const fn from_u8(value: u8) -> Self {
        Self::from_idx3(Index3::from_u8(value))
    }

    #[inline]
    pub const fn from_idx3(index: Index3) -> Self {
        match index {
            Index3::I000 => GprIndex8::AL,
            Index3::I001 => GprIndex8::CL,
            Index3::I010 => GprIndex8::DL,
            Index3::I011 => GprIndex8::BL,
            Index3::I100 => GprIndex8::AH,
            Index3::I101 => GprIndex8::CH,
            Index3::I110 => GprIndex8::DH,
            Index3::I111 => GprIndex8::BH,
        }
    }
}

/// Segment Register index (ES, CS, SS, DS, FS, GS)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SrIndex {
    ES = 0,
    CS = 1,
    SS = 2,
    DS = 3,
    FS = 4,
    GS = 5,
}

impl SrIndex {
    #[inline]
    pub const fn from_u8(value: u8) -> Option<Self> {
        Self::from_idx3(Index3::from_u8(value))
    }

    #[inline]
    pub const fn from_idx3(index: Index3) -> Option<Self> {
        match index {
            Index3::I000 => Some(SrIndex::ES),
            Index3::I001 => Some(SrIndex::CS),
            Index3::I010 => Some(SrIndex::SS),
            Index3::I011 => Some(SrIndex::DS),
            Index3::I100 => Some(SrIndex::FS),
            Index3::I101 => Some(SrIndex::GS),
            _ => None,
        }
    }
}

/// Control Register
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrIndex {
    CR0 = 0,
    //CR1 = 1,
    CR2 = 2,
    CR3 = 3,
    CR4 = 4,
    //CR5 = 5,
    //CR6 = 6,
    //CR7 = 7,
}

impl CrIndex {
    #[inline]
    pub const fn from_idx3(index: Index3) -> Option<Self> {
        match index {
            Index3::I000 => Some(CrIndex::CR0),
            Index3::I010 => Some(CrIndex::CR2),
            Index3::I011 => Some(CrIndex::CR3),
            Index3::I100 => Some(CrIndex::CR4),
            _ => None,
        }
    }
}

/// Debug Register
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrIndex {
    DR0 = 0,
    DR1 = 1,
    DR2 = 2,
    DR3 = 3,
    DR4 = 4,
    DR5 = 5,
    DR6 = 6,
    DR7 = 7,
}

impl DrIndex {
    #[inline]
    pub const fn from_idx3(index: Index3) -> Self {
        match index {
            Index3::I000 => DrIndex::DR0,
            Index3::I001 => DrIndex::DR1,
            Index3::I010 => DrIndex::DR2,
            Index3::I011 => DrIndex::DR3,
            Index3::I100 => DrIndex::DR4,
            Index3::I101 => DrIndex::DR5,
            Index3::I110 => DrIndex::DR6,
            Index3::I111 => DrIndex::DR7,
        }
    }
}

/// 3bit opcode extension for grp1 instructions (0x80, 0x81, 0x83)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpGrp1 {
    ADD = 0,
    OR = 1,
    ADC = 2,
    SBB = 3,
    AND = 4,
    SUB = 5,
    XOR = 6,
    CMP = 7,
}

impl OpGrp1 {
    #[inline]
    pub const fn from_idx3(index: Index3) -> Self {
        match index {
            Index3::I000 => OpGrp1::ADD,
            Index3::I001 => OpGrp1::OR,
            Index3::I010 => OpGrp1::ADC,
            Index3::I011 => OpGrp1::SBB,
            Index3::I100 => OpGrp1::AND,
            Index3::I101 => OpGrp1::SUB,
            Index3::I110 => OpGrp1::XOR,
            Index3::I111 => OpGrp1::CMP,
        }
    }
}

/// 3bit opcode extension for shift/rotate instructions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpShift {
    ROL = 0,
    ROR = 1,
    RCL = 2,
    RCR = 3,
    SHL = 4,
    SHR = 5,
    /// Officially undefined, but commonly alias of SHL
    _SAL = 6,
    SAR = 7,
}

impl OpShift {
    #[inline]
    pub const fn from_idx3(index: Index3) -> Self {
        match index {
            Index3::I000 => OpShift::ROL,
            Index3::I001 => OpShift::ROR,
            Index3::I010 => OpShift::RCL,
            Index3::I011 => OpShift::RCR,
            Index3::I100 => OpShift::SHL,
            Index3::I101 => OpShift::SHR,
            Index3::I110 => OpShift::_SAL,
            Index3::I111 => OpShift::SAR,
        }
    }
}

/// 3bit opcode extension for grp3 instructions (0xf6, 0xf7)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpGrp3 {
    TEST = 0,
    /// Officially undefined, but commonly alias of TEST
    _TEST1 = 1,
    NOT = 2,
    NEG = 3,
    MUL = 4,
    IMUL = 5,
    DIV = 6,
    IDIV = 7,
}

impl OpGrp3 {
    #[inline]
    pub const fn from_idx3(index: Index3) -> Self {
        match index {
            Index3::I000 => OpGrp3::TEST,
            Index3::I001 => OpGrp3::_TEST1,
            Index3::I010 => OpGrp3::NOT,
            Index3::I011 => OpGrp3::NEG,
            Index3::I100 => OpGrp3::MUL,
            Index3::I101 => OpGrp3::IMUL,
            Index3::I110 => OpGrp3::DIV,
            Index3::I111 => OpGrp3::IDIV,
        }
    }
}

/// 3bit opcode extension for grp4 instructions (0xfe)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpGrp4 {
    INC = 0,
    DEC = 1,
    /// Undefined
    _UD,
}

impl OpGrp4 {
    #[inline]
    pub const fn from_idx3(index: Index3) -> Self {
        match index {
            Index3::I000 => OpGrp4::INC,
            Index3::I001 => OpGrp4::DEC,
            _ => OpGrp4::_UD,
        }
    }
}

/// 3bit opcode extension for grp5 instructions (0xff)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpGrp5 {
    INC = 0,
    DEC = 1,
    CALL = 2,
    CALLF = 3,
    JMP = 4,
    JMPF = 5,
    PUSH = 6,
    /// undefined
    _UD7 = 7,
}

impl OpGrp5 {
    #[inline]
    pub const fn from_idx3(index: Index3) -> Self {
        match index {
            Index3::I000 => OpGrp5::INC,
            Index3::I001 => OpGrp5::DEC,
            Index3::I010 => OpGrp5::CALL,
            Index3::I011 => OpGrp5::CALLF,
            Index3::I100 => OpGrp5::JMP,
            Index3::I101 => OpGrp5::JMPF,
            Index3::I110 => OpGrp5::PUSH,
            _ => OpGrp5::_UD7,
        }
    }
}

/// 3bit opcode extension for grp6 instructions (0x0f 0x00)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpGrp6 {
    SLDT = 0,
    STR = 1,
    LLDT = 2,
    LTR = 3,
    VERR = 4,
    VERW = 5,
    //JMPE = 6,
    /// Undefined
    _UD,
}

impl OpGrp6 {
    #[inline]
    pub const fn from_idx3(index: Index3) -> Self {
        match index {
            Index3::I000 => OpGrp6::SLDT,
            Index3::I001 => OpGrp6::STR,
            Index3::I010 => OpGrp6::LLDT,
            Index3::I011 => OpGrp6::LTR,
            Index3::I100 => OpGrp6::VERR,
            Index3::I101 => OpGrp6::VERW,
            _ => OpGrp6::_UD,
        }
    }
}

/// 3bit opcode extension for grp7 instructions (0x0f 0x01)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpGrp7 {
    SGDT = 0,
    SIDT = 1,
    LGDT = 2,
    LIDT = 3,
    SMSW = 4,
    _UD5 = 5,
    LMSW = 6,
    INVLPG = 7,
}

impl OpGrp7 {
    #[inline]
    pub const fn from_idx3(index: Index3) -> Self {
        match index {
            Index3::I000 => OpGrp7::SGDT,
            Index3::I001 => OpGrp7::SIDT,
            Index3::I010 => OpGrp7::LGDT,
            Index3::I011 => OpGrp7::LIDT,
            Index3::I100 => OpGrp7::SMSW,
            Index3::I101 => OpGrp7::_UD5,
            Index3::I110 => OpGrp7::LMSW,
            Index3::I111 => OpGrp7::INVLPG,
        }
    }
}

/// 3bit opcode extension for grp8 (0x0f 0xba)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpGrp8 {
    _UD,
    BT = 4,
    BTS = 5,
    BTR = 6,
    BTC = 7,
}

impl OpGrp8 {
    #[inline]
    pub const fn from_idx3(index: Index3) -> Self {
        match index {
            Index3::I100 => OpGrp8::BT,
            Index3::I101 => OpGrp8::BTS,
            Index3::I110 => OpGrp8::BTR,
            Index3::I111 => OpGrp8::BTC,
            _ => OpGrp8::_UD,
        }
    }
}

/// ModR/M byte
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModRM(u8);

impl ModRM {
    #[inline]
    pub const fn new(value: u8) -> Self {
        ModRM(value)
    }

    #[inline]
    pub const fn raw(&self) -> u8 {
        self.0
    }

    #[inline]
    pub const fn reg_index(&self) -> Index3 {
        Index3::from_u8(self.0 >> 3)
    }

    #[inline]
    pub const fn rm_index(&self) -> Index3 {
        Index3::from_u8(self.0)
    }

    #[inline]
    pub const fn mod_bits(&self) -> ModBits {
        match self.0 >> 6 {
            0b00 => ModBits::Zero,
            0b01 => ModBits::DispByte,
            0b10 => ModBits::DispVar,
            0b11 => ModBits::Reg,
            _ => unreachable!(),
        }
    }
}

/// Scale-Index-Base byte
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SIB(u8);

impl SIB {
    #[inline]
    pub const fn scale(&self) -> Scale {
        Scale::from_sib(*self)
    }

    #[inline]
    pub const fn index(&self) -> Index3 {
        Index3::from_u8((self.0 >> 3) & 0b111)
    }

    #[inline]
    pub const fn base(&self) -> Index3 {
        Index3::from_u8(self.0 & 0b111)
    }
}

/// Mod bits in ModR/M byte
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModBits {
    /// Mod bits 00: no displacement or displacement only if special case
    Zero = 0b00,
    /// Mod bits 01: 8-bit displacement follows
    DispByte = 0b01,
    /// Mod bits 10: 16-bit or 32-bit displacement follows depending on the address size
    DispVar = 0b10,
    /// Mod bits 11: register-direct addressing mode
    Reg = 0b11,
}

/// Parsed ModR/M for 16-bit addressing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModRm16 {
    modrm: ModRM,
    mem: MemOpr16,
}

impl ModRm16 {
    /// Fetches and parses a ModR/M byte and any associated displacement from the instruction stream.
    pub fn fetch<FETCH: Fetch<E = E>, E>(
        is: &mut FETCH,
        segment: Option<SrIndex>,
    ) -> Result<Self, E> {
        let modrm = ModRM(is.fetch_u8()?);
        let mod_bits = modrm.mod_bits();
        if mod_bits == ModBits::Reg {
            Ok(Self {
                modrm,
                mem: MemOpr16::DUMMY,
            })
        } else {
            let base_index = BaseIndex16::from_modrm(modrm);
            let segment = segment.unwrap_or(base_index.map_or(DS, |v| v.default_segment()));
            match (mod_bits, base_index) {
                (ModBits::DispByte, _) => {
                    let disp = Offset16(is.fetch_i8()? as i16 as u16);
                    Ok(Self {
                        modrm,
                        mem: MemOpr16 {
                            segment,
                            base_index,
                            disp,
                        },
                    })
                }
                (ModBits::DispVar, _) | (_, None) => {
                    let disp = Offset16(is.fetch_u16()?);
                    Ok(Self {
                        modrm,
                        mem: MemOpr16 {
                            segment,
                            base_index,
                            disp,
                        },
                    })
                }
                _ => Ok(Self {
                    modrm,
                    mem: MemOpr16 {
                        segment,
                        base_index,
                        disp: Offset16(0),
                    },
                }),
            }
        }
    }

    /// Returns the raw ModR/M byte.
    #[inline]
    pub const fn raw_modrm(&self) -> ModRM {
        self.modrm
    }

    /// Returns the index of reg field in the ModR/M byte.
    #[inline]
    pub const fn reg_index(&self) -> Index3 {
        self.modrm.reg_index()
    }

    /// Returns either the register or memory operand based on the ModR/M byte.
    #[inline]
    pub fn reg_or_mem(&self) -> RegOrMem<Index3, MemOpr16> {
        if matches!(self.modrm.mod_bits(), ModBits::Reg) {
            RegOrMem::Reg(self.modrm.rm_index())
        } else {
            RegOrMem::Mem(self.mem)
        }
    }

    /// Force override the segment register for memory operand.
    ///
    /// Note: this function always safe to call even if the ModR/M byte indicates a register operand, since it will not be used in that case.
    #[inline]
    pub fn force_override(&mut self, segment: SrIndex) {
        self.mem.segment = segment;
    }
}

/// Register or Memory operand based on ModR/M byte
pub enum RegOrMem<R, M> {
    /// The operand is a register
    Reg(R),
    /// The operand is a memory address
    Mem(M),
}

/// Parsed ModR/M for 32-bit addressing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModRm32 {
    modrm: ModRM,
    sib: Option<SIB>,
    mem: MemOpr32,
}

impl ModRm32 {
    /// Fetches and parses a ModR/M byte and any associated displacement from the instruction stream.
    pub fn fetch<FETCH: Fetch<E = E>, E>(
        is: &mut FETCH,
        segment: Option<SrIndex>,
    ) -> Result<Self, E> {
        let modrm = ModRM(is.fetch_u8()?);
        let mod_bits = modrm.mod_bits();
        if mod_bits == ModBits::Reg {
            Ok(Self {
                modrm,
                sib: None,
                mem: MemOpr32::DUMMY,
            })
        } else {
            let (base_index, scale, sib) = match BaseIndex::from_modrm32(modrm) {
                Some(base_index) => (base_index, Scale::Byte, None),
                None => {
                    let sib = SIB(is.fetch_u8()?);
                    let scale = sib.scale();
                    (BaseIndex::from_modrm_sib(modrm, sib), scale, Some(sib))
                }
            };
            let no_base_index_only = matches!(base_index, BaseIndex::Index(_));
            let base_index = BaseIndex32::from_base_index(base_index, scale);
            let segment = segment.unwrap_or(base_index.default_segment());
            match (mod_bits, base_index, no_base_index_only) {
                (ModBits::DispByte, _, _) => {
                    let disp = Offset32(is.fetch_i8()? as i32 as u32);
                    Ok(Self {
                        modrm,
                        sib,
                        mem: MemOpr32 {
                            segment,
                            base_index,
                            disp,
                        },
                    })
                }
                (ModBits::DispVar, _, _) | (_, BaseIndex32::DispOnly, _) | (_, _, true) => {
                    let disp = Offset32(is.fetch_u32()?);
                    Ok(Self {
                        modrm,
                        sib,
                        mem: MemOpr32 {
                            segment,
                            base_index,
                            disp,
                        },
                    })
                }
                _ => Ok(Self {
                    modrm,
                    sib,
                    mem: MemOpr32 {
                        segment,
                        base_index,
                        disp: Offset32(0),
                    },
                }),
            }
        }
    }

    /// Returns the raw ModR/M byte.
    #[inline]
    pub const fn raw_modrm(&self) -> ModRM {
        self.modrm
    }

    /// Returns the raw SIB byte if present.
    #[inline]
    pub const fn raw_sib(&self) -> Option<SIB> {
        self.sib
    }

    /// Returns the index of reg field in the ModR/M byte.
    #[inline]
    pub const fn reg_index(&self) -> Index3 {
        self.modrm.reg_index()
    }

    /// Returns either the register or memory operand based on the ModR/M byte.
    #[inline]
    pub fn reg_or_mem(&self) -> RegOrMem<Index3, MemOpr32> {
        if matches!(self.modrm.mod_bits(), ModBits::Reg) {
            RegOrMem::Reg(self.modrm.rm_index())
        } else {
            RegOrMem::Mem(self.mem)
        }
    }

    /// Force override the segment register for memory operand.
    ///
    /// Note: this function always safe to call even if the ModR/M byte indicates a register operand, since it will not be used in that case.
    #[inline]
    pub fn force_override(&mut self, segment: SrIndex) {
        self.mem.segment = segment;
    }
}

/// Memory operand in 16-bit addressing mode
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MemOpr16 {
    pub segment: SrIndex,
    pub base_index: Option<BaseIndex16>,
    pub disp: Offset16,
}

impl MemOpr16 {
    /// A dummy memory operand used when the ModR/M byte indicates a register operand.
    pub const DUMMY: Self = unsafe { core::mem::zeroed() };

    #[inline]
    pub const fn from_sel_off(segment: SrIndex, offset: Offset16) -> Self {
        Self {
            segment,
            base_index: None,
            disp: offset,
        }
    }

    #[inline]
    pub const fn for_xlat(segment: SrIndex) -> Self {
        Self {
            segment,
            base_index: Some(BaseIndex16::Bx),
            disp: Offset16(0),
        }
    }
}

impl core::fmt::Debug for MemOpr16 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use alloc::format;

        let triple = if let Some(base_index) = self.base_index {
            if self.disp.0 != 0 {
                format!("{:?}+0x{:04x}", base_index, self.disp.0)
            } else {
                format!("{:?}", base_index)
            }
        } else {
            format!("0x{:04x}", self.disp.0)
        };

        if self.segment != self.base_index.map_or(DS, |v| v.default_segment()) {
            write!(f, "[{:?}: {}]", self.segment, triple)
        } else {
            write!(f, "[{}]", triple)
        }
    }
}

/// Memory operand in 32-bit addressing mode
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MemOpr32 {
    pub segment: SrIndex,
    pub base_index: BaseIndex32,
    pub disp: Offset32,
}

impl MemOpr32 {
    /// A dummy memory operand used when the ModR/M byte indicates a register operand.
    pub const DUMMY: Self = unsafe { core::mem::zeroed() };

    #[inline]
    pub const fn from_sel_off(segment: SrIndex, offset: Offset32) -> Self {
        Self {
            segment,
            base_index: BaseIndex32::DispOnly,
            disp: offset,
        }
    }

    #[inline]
    pub const fn for_xlat(segment: SrIndex) -> Self {
        Self {
            segment,
            base_index: BaseIndex32::Base(EBX),
            disp: Offset32(0),
        }
    }
}

impl core::fmt::Debug for MemOpr32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use alloc::format;
        use alloc::string::String;
        use alloc::vec::Vec;

        let base = if let Some(base) = self.base_index.base() {
            format!("{:?}", base)
        } else {
            String::new()
        };
        let index = if let Some(index) = self.base_index.index() {
            format!(
                "{:?}*{}",
                index,
                self.base_index
                    .scale()
                    .map(|scale| scale.scale_factor())
                    .unwrap_or(1)
            )
        } else {
            String::new()
        };
        let disp = match self.base_index {
            BaseIndex32::DispOnly => format!("0x{:08x}", self.disp.0),
            _ => {
                if self.disp.0 != 0 {
                    format!("0x{:08x}", self.disp.0)
                } else {
                    String::new()
                }
            }
        };
        let triple = [base, index, disp]
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join("+");

        if self.base_index.default_segment() != self.segment {
            write!(f, "[{:?}: {}]", self.segment, triple)
        } else {
            write!(f, "[{}]", triple)
        }
    }
}

/// Scale factor for SIB byte
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    /// 00: No scaling (scale factor = 1)
    Byte,
    /// 01: Scale factor = 2
    Word,
    /// 10: Scale factor = 4
    DWord,
    /// 11: Scale factor = 8
    QWord,
}

impl Scale {
    /// Returns the scale factor corresponding to the SIB byte.
    #[inline]
    pub const fn scale_factor(&self) -> u32 {
        match self {
            Scale::Byte => 1,
            Scale::Word => 2,
            Scale::DWord => 4,
            Scale::QWord => 8,
        }
    }

    /// Returns the shift amount corresponding to the SIB byte.
    #[inline]
    pub const fn shift(&self) -> u32 {
        match self {
            Scale::Byte => 0,
            Scale::Word => 1,
            Scale::DWord => 2,
            Scale::QWord => 3,
        }
    }

    /// Decodes the scale from the SIB byte.
    #[inline]
    pub const fn from_sib(sib: SIB) -> Self {
        match sib.0 >> 6 {
            0b00 => Scale::Byte,
            0b01 => Scale::Word,
            0b10 => Scale::DWord,
            0b11 => Scale::QWord,
            _ => unreachable!(),
        }
    }
}

/// Combination of base and index registers for 16-bit addressing mode
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BaseIndex16 {
    /// `BX + SI`
    BxSi,
    /// `BX + DI`
    BxDi,
    /// `BP + SI`
    BpSi,
    /// `BP + DI`
    BpDi,
    /// `SI`
    Si,
    /// `DI`
    Di,
    /// `BP`
    Bp,
    /// `BX`
    Bx,
}

impl BaseIndex16 {
    /// Decodes the ModR/M
    pub fn from_modrm(modrm: ModRM) -> Option<Self> {
        let mod_bits = modrm.mod_bits();
        let rm = modrm.rm_index();

        match (mod_bits, rm) {
            (_, Index3::I000) => Some(BaseIndex16::BxSi),
            (_, Index3::I001) => Some(BaseIndex16::BxDi),
            (_, Index3::I010) => Some(BaseIndex16::BpSi),
            (_, Index3::I011) => Some(BaseIndex16::BpDi),
            (_, Index3::I100) => Some(BaseIndex16::Si),
            (_, Index3::I101) => Some(BaseIndex16::Di),
            (ModBits::Zero, Index3::I110) => None,
            (_, Index3::I110) => Some(BaseIndex16::Bp),
            (_, Index3::I111) => Some(BaseIndex16::Bx),
        }
    }

    /// Returns the default segment register for the given base/index combination.
    #[inline]
    pub const fn default_segment(&self) -> SrIndex {
        match self {
            BaseIndex16::Bp | BaseIndex16::BpSi | BaseIndex16::BpDi => SrIndex::SS,
            _ => SrIndex::DS,
        }
    }

    /// Returns the base register for the given base/index combination.
    #[inline]
    pub const fn base(&self) -> GprIndex16 {
        match self {
            BaseIndex16::BxSi | BaseIndex16::BxDi | BaseIndex16::Bx => BX,
            BaseIndex16::BpSi | BaseIndex16::BpDi | BaseIndex16::Bp => BP,
            BaseIndex16::Si => SI,
            BaseIndex16::Di => DI,
        }
    }

    /// Returns the index register for the given base/index combination, if any.
    #[inline]
    pub const fn index(&self) -> Option<GprIndex16> {
        match self {
            BaseIndex16::BxSi | BaseIndex16::BpSi => Some(SI),
            BaseIndex16::BxDi | BaseIndex16::BpDi => Some(DI),
            _ => None,
        }
    }
}

impl core::fmt::Debug for BaseIndex16 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            BaseIndex16::BxSi => "BX+SI",
            BaseIndex16::BxDi => "BX+DI",
            BaseIndex16::BpSi => "BP+SI",
            BaseIndex16::BpDi => "BP+DI",
            BaseIndex16::Si => "SI",
            BaseIndex16::Di => "DI",
            BaseIndex16::Bp => "BP",
            BaseIndex16::Bx => "BX",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BaseIndex32 {
    /// Displacement only, no base or index register
    DispOnly,
    /// Base register only
    Base(GprIndex32),
    /// Scale-Index-Base addressing with base and index registers
    Sib(SibIndex),
}

impl BaseIndex32 {
    #[inline]
    pub fn from_base_index(value: BaseIndex<GprIndex32>, scale: Scale) -> Self {
        match value {
            BaseIndex::DispOnly => BaseIndex32::DispOnly,
            BaseIndex::Base(base) => BaseIndex32::Base(base),
            BaseIndex::Index(index) => BaseIndex32::Sib(SibIndex::from_sib(None, index, scale)),
            BaseIndex::BaseIndex(base, index) => {
                BaseIndex32::Sib(SibIndex::from_sib(Some(base), index, scale))
            }
        }
    }

    #[inline]
    pub fn scale(&self) -> Option<Scale> {
        match self {
            BaseIndex32::DispOnly | BaseIndex32::Base(_) => None,
            BaseIndex32::Sib(sib) => Some(sib.to_sib().2),
        }
    }

    #[inline]
    pub fn base(&self) -> Option<GprIndex32> {
        match self {
            BaseIndex32::DispOnly => None,
            BaseIndex32::Base(base) => Some(*base),
            BaseIndex32::Sib(sib) => sib.to_sib().0,
        }
    }

    #[inline]
    pub fn index(&self) -> Option<GprIndex32> {
        match self {
            BaseIndex32::DispOnly | BaseIndex32::Base(_) => None,
            BaseIndex32::Sib(sib) => Some(sib.to_sib().1),
        }
    }

    /// Returns the default segment register for the given base/index combination.
    #[inline]
    pub fn default_segment(&self) -> SrIndex {
        match self.base() {
            Some(EBP) | Some(ESP) => SrIndex::SS,
            _ => SrIndex::DS,
        }
    }
}

/// Combination of base and index registers for memory addressing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseIndex<IDX> {
    /// Displacement only, no base or index register
    DispOnly,
    /// Base register only
    Base(IDX),
    /// Index register only
    Index(IDX),
    /// Both base and index registers
    BaseIndex(IDX, IDX),
}

impl BaseIndex<GprIndex32> {
    /// Decodes the ModR/M type for 32-bit addressing mode.
    ///
    /// Returns `None` if the ModR/M byte indicates that a SIB byte follows
    pub fn from_modrm32(modrm: ModRM) -> Option<Self> {
        let mod_bits = modrm.mod_bits();
        let rm = modrm.rm_index();
        match (mod_bits, rm) {
            (ModBits::Zero, Index3::I101) => Some(BaseIndex::DispOnly),
            (_, Index3::I100) => None, // SIB byte follows
            (_, rm) => Some(BaseIndex::Base(GprIndex32::from_idx3(rm))),
        }
    }

    /// Decodes the SIB type from the ModR/M and SIB bytes.
    pub fn from_modrm_sib(modrm: ModRM, sib: SIB) -> Self {
        let mod_bits = modrm.mod_bits();
        let base = sib.base();
        let index = sib.index();
        match (mod_bits, base, index) {
            (ModBits::Zero, Index3::I101, Index3::I100) => BaseIndex::DispOnly,
            (ModBits::Zero, Index3::I101, _) => BaseIndex::Index(GprIndex32::from_idx3(index)),
            (_, _, Index3::I100) => BaseIndex::Base(GprIndex32::from_idx3(base)),
            (_, _, _) => {
                BaseIndex::BaseIndex(GprIndex32::from_idx3(base), GprIndex32::from_idx3(index))
            }
        }
    }
}

/// Condition Codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CC {
    /// Overflow
    O = 0b0000,
    /// Not Overflow
    NO = 0b0001,
    /// Carry
    C = 0b0010,
    /// Not Carry
    NC = 0b0011,
    /// Zero
    Z = 0b0100,
    /// Not Zero
    NZ = 0b0101,
    /// Below or Equal
    BE = 0b0110,
    /// Not Below or Equal
    NBE = 0b0111,
    /// Sign
    S = 0b1000,
    /// Not Sign
    NS = 0b1001,
    /// Parity
    P = 0b1010,
    /// Not Parity
    NP = 0b1011,
    /// Less
    L = 0b1100,
    /// Not Less
    NL = 0b1101,
    /// Less or Equal
    LE = 0b1110,
    /// Not Less or Equal
    NLE = 0b1111,
}

impl CC {
    /// Below
    pub const B: Self = Self::C;
    /// Not Above or Equal
    pub const NAE: Self = Self::C;

    /// Not Below
    pub const NB: Self = Self::NC;
    /// Above or Equal
    pub const AE: Self = Self::NC;

    /// Equal
    pub const E: Self = Self::Z;

    /// Not Equal
    pub const NE: Self = Self::NZ;

    /// Not Above
    pub const NA: Self = Self::BE;

    /// Above
    pub const A: Self = Self::NBE;

    /// Parity Even
    pub const PE: Self = Self::P;

    /// Parity Odd
    pub const PO: Self = Self::NP;

    /// Not Greater or Equal
    pub const NGE: Self = Self::L;

    /// Greater or Equal
    pub const GE: Self = Self::NL;

    /// Not Greater
    pub const NG: Self = Self::LE;

    /// Greater
    pub const G: Self = Self::NLE;

    #[inline]
    pub const fn from_u8(value: u8) -> Self {
        match value & 0b1111 {
            0 => Self::O,
            1 => Self::NO,
            2 => Self::C,
            3 => Self::NC,
            4 => Self::Z,
            5 => Self::NZ,
            6 => Self::BE,
            7 => Self::NBE,
            8 => Self::S,
            9 => Self::NS,
            10 => Self::P,
            11 => Self::NP,
            12 => Self::L,
            13 => Self::NL,
            14 => Self::LE,
            15 => Self::NLE,
            _ => unreachable!(),
        }
    }
}

/// Lock and Repeat Prefixes
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrefixFx {
    #[default]
    /// No prefix
    NP,
    /// F0: LOCK prefix
    F0_LOCK,
    /// F2: REPNE/REPNZ prefix
    F2_REPNZ,
    /// F3: REP/REPE/REPZ prefix
    F3_REPZ,
}
