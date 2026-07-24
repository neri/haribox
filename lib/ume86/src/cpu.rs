//! CPU definitions

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Generation {
    I8086 = 0,
    I186 = 1,
    I286 = 2,
    I386 = 3,
    I486 = 4,
    Pentium = 5,
    P6 = 6,
    P7 = 7,
}

impl Generation {
    /// The latest CPU generation supported by this emulator.
    pub const LATEST: Generation = Generation::P7;

    #[inline]
    pub const fn cpuid_base(&self) -> u32 {
        (*self as u32) << 8
    }
}
