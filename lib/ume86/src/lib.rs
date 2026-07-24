//! User mode emulator for x86

#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod alu;
pub mod cpu;
pub mod flags;
pub mod gpr;
pub mod state;
pub mod types;
pub mod ume;

#[cfg(test)]
mod tests;

pub mod prelude {

    pub use ir86::prelude::*;

    pub use crate::cpu::*;
    pub use crate::flags::*;
    pub use crate::types::*;
}

#[allow(unused_imports)]
pub(crate) mod _prelude_ {
    pub use alloc::boxed::Box;
    pub use alloc::collections::BTreeMap;
    pub use alloc::string::String;
    pub use alloc::sync::Arc;
    pub use alloc::vec::Vec;

    pub use crate::prelude::*;
}
