//! Haribote OS executable format
#![cfg_attr(not(test), no_std)]

/// Haribote OS executable format
///
/// ## In file
///
/// ```text
/// +---------+
/// | header  |
/// + - - - - +
/// | code    |
/// +---------+
/// | data    |
/// +---------+
/// ```
///
/// ## In memory
///
/// ```text
/// cs:
/// +---------+ <- cs:0
/// | header  |
/// + - - - - +
/// | code    |
/// | ...     | <- entry_point
/// +---------+ <- start_data
///
/// ds:
/// +---------+ <- ds:0
/// | stack   |
/// +---------+ <- esp / (start_data)
/// | data    | } size_of_data
/// +---------+ <- start_malloc?
/// | bss     | } size_of_bss?
/// +---------+ <- size_of_ds
/// ```
#[repr(C)]
#[derive(Debug)]
pub struct HrbExecutable {
    /// Size of data segment
    pub size_of_ds: u32,
    /// Must be `b"Hari"`
    pub signature: [u8; 4],
    /// Size of bss?
    pub size_of_bss: u32,
    /// Initial Stack Pointer
    pub esp: u32,
    /// Size of data in file
    pub size_of_data: u32,
    /// Size of code and start data in file
    pub start_data: u32,
    /// startup machine code
    pub _start: [u8; 4],
    /// Entry point (relative)
    pub entry_m20: u32,
    /// Malloc area?
    pub start_malloc: u32,
}

impl HrbExecutable {
    pub const SIGNATURE: [u8; 4] = *b"Hari";

    pub const START: u32 = 0x1B;

    pub const MINIMAL_BIN_SIZE: usize = 0x24;

    pub fn identify(bytes: &[u8]) -> Option<&HrbExecutable> {
        if bytes.len() < Self::MINIMAL_BIN_SIZE {
            return None;
        }
        let align = core::mem::align_of::<HrbExecutable>();
        if bytes.as_ptr() as usize % align != 0 {
            return None;
        }
        let ptr = bytes.as_ptr() as *const HrbExecutable;
        // Safety: We have checked that the slice is large enough and properly aligned for `HrbExecutable`.
        let hrb = unsafe { &*ptr };
        if hrb.signature != Self::SIGNATURE {
            return None;
        }
        Some(hrb)
    }

    #[inline]
    pub const fn entry_point(&self) -> u32 {
        self.entry_m20.wrapping_add(0x20)
    }
}
