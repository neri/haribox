//! Intermediate Micro Opcode

use ir86::encoding::CC;
use ir86::types::Offset32;

use crate::state::{RtReg, RtReg8};
use crate::ume::sib::SibIndex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FuncIndex(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AddrIndex(pub u32);

/// Micro Operation Code
#[derive(Debug, Clone, Copy)]
pub enum Uop {
    /// Unresolved Fetch Next Instruction
    FetchNext(Offset32),

    /// Unconditional Jump
    Jump(AddrIndex),
    /// Unconditional Jump to Register
    JumpR(RtReg),

    /// Conditional Jump (Unresolved)
    JccU(CC, Offset32),
    /// Conditional Jump (Resolved)
    Jcc(CC, AddrIndex),

    /// Call to Function
    Call(FuncIndex, Offset32),
    /// Return from Function
    Ret(u16),

    /// Load Constant
    LoadConst(RtReg, u32),
    /// Load Constant 8-bit
    LoadConst8(RtReg8, u8),
    /// Load Constant 16-bit
    LoadConst16(RtReg, u16),

    /// Move Register
    Move(RtReg, RtReg),

    Move8(RtReg8, RtReg8),

    Move16(RtReg, RtReg),

    /// Not Register
    NotR(RtReg),

    /// Exchange Register
    XchgR(RtReg, RtReg),

    /// Move with Zero-Extend 8-bit to 32-bit
    MovZx8(RtReg, RtReg8),
    /// Move with Zero-Extend 16-bit to 32-bit
    MovZx16(RtReg, RtReg),
    /// Move with Sign-Extend 8-bit to 32-bit
    MovSx8(RtReg, RtReg8),
    /// Move with Sign-Extend 16-bit to 32-bit
    MovSx16(RtReg, RtReg),

    /// Load from Memory with Base Register
    LoadR32(RtReg, RtReg),
    /// Load from Memory with Base and Displacement
    LoadBD32(RtReg, RtReg, Offset32),
    /// Load from Memory with Base, Index, Scale and Displacement
    LoadSIB32(RtReg, SibIndex, Offset32),

    /// Load from Memory with Base Register and Code Segment
    LoadR32CS(RtReg, RtReg),
    /// Load from Memory with Base and Displacement and Code Segment
    LoadBD32CS(RtReg, RtReg, Offset32),
    /// Load from Memory with Base, Index, Scale and Displacement and Code Segment
    LoadSIB32CS(RtReg, SibIndex, Offset32),

    /// Store to Memory with Base Register
    StoreR32(RtReg, RtReg),
    /// Store to Memory with Base and Displacement
    StoreBD32(RtReg, RtReg, Offset32),
    /// Store to Memory with Base, Index, Scale and Displacement
    StoreSIB32(RtReg, SibIndex, Offset32),

    /// Load from Memory with Base Register (8-bit)
    LoadR8(RtReg8, RtReg),
    /// Store to Memory with Base Register (8-bit)
    StoreR8(RtReg8, RtReg),

    /// Load from Memory with Base Register (16-bit)
    LoadR16(RtReg, RtReg),
    /// Store to Memory with Base Register (16-bit)
    StoreR16(RtReg, RtReg),

    /// Load Effective Address (LEA) with Base and Displacement
    LeaBD(RtReg, RtReg, Offset32),
    /// Load Effective Address (LEA) with Base, Index, Scale and Displacement
    LeaSIB(RtReg, SibIndex, Offset32),

    /// Push Immediate Value
    PushI(u32),
    /// Push Register Value
    PushR(RtReg),
    /// Push All Registers
    PushAd,
    /// Pop into Register
    PopR(RtReg),
    /// Pop All Registers
    PopAd,

    /// Add Register and Immediate Value
    AddI(RtReg, u32),
    /// Add Register Values
    AddR(RtReg, RtReg),
    // Add Register and Immediate Value (8-bit)
    AddI8(RtReg8, u8),
    /// Add Register and Immediate Value (16-bit)
    AddI16(RtReg, u16),
    /// Add Register Values (8-bit)
    AddR8(RtReg8, RtReg8),
    /// Add Register Values (16-bit)
    AddR16(RtReg, RtReg),

    /// Subtract Register and Immediate Value
    SubI(RtReg, u32),
    /// Subtract Register Values
    SubR(RtReg, RtReg),
    /// Subtract Register and Immediate Value (8-bit)
    SubI8(RtReg8, u8),
    /// Subtract Register and Immediate Value (16-bit)
    SubI16(RtReg, u16),
    /// Subtract Register Values (8-bit)
    SubR8(RtReg8, RtReg8),
    /// Subtract Register Values (16-bit)
    SubR16(RtReg, RtReg),

    /// Negate Register Value
    NegR(RtReg),

    /// Compare Register and Immediate Value
    CmpI(RtReg, u32),
    /// Compare Register Values
    CmpR(RtReg, RtReg),

    /// Compare Register and Immediate Value (8-bit)
    CmpI8(RtReg8, u8),
    /// Compare Register and Immediate Value (16-bit)
    CmpI16(RtReg, u16),
    /// Compare Register Values (8-bit)
    CmpR8(RtReg8, RtReg8),
    /// Compare Register Values (16-bit)
    CmpR16(RtReg, RtReg),

    /// Increment Register
    IncR(RtReg),
    /// Decrement Register
    DecR(RtReg),

    /// And Register and Immediate Value
    AndI(RtReg, u32),
    /// And Register Values
    AndR(RtReg, RtReg),
    /// And Register and Immediate Value (8-bit)
    AndI8(RtReg8, u8),
    /// And Register and Immediate Value (16-bit)
    AndI16(RtReg, u16),
    /// And Register Values (8-bit)
    AndR8(RtReg8, RtReg8),
    /// And Register Values (16-bit)
    AndR16(RtReg, RtReg),

    /// Or Register and Immediate Value
    OrI(RtReg, u32),
    /// Or Register Values
    OrR(RtReg, RtReg),

    /// Xor Register and Immediate Value
    XorI(RtReg, u32),
    /// Xor Register Values
    XorR(RtReg, RtReg),

    /// Test Register and Immediate Value
    TestI(RtReg, u32),
    /// Test Register Values
    TestR(RtReg, RtReg),
    /// Test Register and Immediate Value (8-bit)
    TestI8(RtReg8, u8),

    /// Test Register Values (8-bit)
    TestR8(RtReg8, RtReg8),

    /// Integer Multiply by Register
    IMulR(RtReg, RtReg),
    /// Integer Multiply by Register and Immediate Value
    IMulRI(RtReg, RtReg, i32),
    /// Divide by Register
    DivR(RtReg),
    /// Integer Divide by Register
    IDivR(RtReg),

    /// Shift Left Register by Immediate Value
    ShlI(RtReg, u8),
    /// Shift Left Register by Cl
    ShlRCl(RtReg),
    /// Shift Left Register by Immediate Value (8-bit)
    ShlI8(RtReg8, u8),
    /// Shift Left Register by Cl (8-bit)
    ShlRCl8(RtReg8),
    /// Shift Left Register by Immediate Value (16-bit)
    ShlI16(RtReg, u8),
    /// Shift Left Register by Cl (16-bit)
    ShlRCl16(RtReg),

    /// Shift Right Register by Immediate Value
    ShrI(RtReg, u8),
    /// Shift Right Register by Cl
    ShrRCl(RtReg),
    /// Shift Right Register by Immediate Value (8-bit)
    ShrI8(RtReg8, u8),
    /// Shift Right Register by Cl (8-bit)
    ShrRCl8(RtReg8),
    /// Shift Right Register by Immediate Value (16-bit)
    ShrI16(RtReg, u8),
    /// Shift Right Register by Cl (16-bit)
    ShrRCl16(RtReg),

    /// Shift Arithmetic Right Register by Immediate Value
    SarI(RtReg, u8),
    /// Shift Arithmetic Right Register by Cl
    SarRCl(RtReg),
    /// Shift Arithmetic Right Register by Immediate Value (8-bit)
    SarI8(RtReg8, u8),
    /// Shift Arithmetic Right Register by Cl (8-bit)
    SarRCl8(RtReg8),
    /// Shift Arithmetic Right Register by Immediate Value (16-bit)
    SarI16(RtReg, u8),
    /// Shift Arithmetic Right Register by Cl (16-bit)
    SarRCl16(RtReg),

    /// Software Interrupt
    Swi(u8),

    /// Minor Instructions
    Minor(UopMinor),
}

/// Minor Uop for special instructions
#[derive(Debug, Clone, Copy)]
pub enum UopMinor {
    Cdq,
    Cld,

    /// Identify the Processor and its Features
    Cpuid,

    /// Multiply by Register
    MulR(RtReg),

    /// Repeat Scasb
    RepNzScasb,

    /// Read Time-Stamp Counter
    RdTsc,
}
