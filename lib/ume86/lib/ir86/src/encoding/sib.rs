//! Compact SIB

use super::SIB;
use crate::_prelude_::*;

/// Compact SIB Table
static COMPACT_SIB_TABLE: [(Option<GprIndex32>, GprIndex32, Scale); 256] = {
    let mut table: [(Option<GprIndex32>, GprIndex32, Scale); 256] =
        [(None, GprIndex32::EAX, Scale::Byte); 256];
    let mut i = 0usize;
    while i < 256 {
        let scale = Scale::from_sib(SIB(i as u8));
        let index = (i & 63) / 9;
        let base = (i & 63) % 9;

        // 9 * 7 = 63 < 8 * 8 = 64
        let base = match base {
            0b000 => Some(GprIndex32::EAX),
            0b001 => Some(GprIndex32::ECX),
            0b010 => Some(GprIndex32::EDX),
            0b011 => Some(GprIndex32::EBX),
            0b100 => Some(GprIndex32::ESP),
            0b101 => Some(GprIndex32::EBP),
            0b110 => Some(GprIndex32::ESI),
            0b111 => Some(GprIndex32::EDI),
            _ => None,
        };
        let index = match index {
            0b000 => GprIndex32::EAX,
            0b001 => GprIndex32::ECX,
            0b010 => GprIndex32::EDX,
            0b011 => GprIndex32::EBX,
            0b100 => GprIndex32::EBP, // ESP is invalid for index, so we skip it
            0b101 => GprIndex32::ESI,
            0b110 => GprIndex32::EDI,
            _ => GprIndex32::EAX, // TODO: this value is undefined
        };

        table[i] = (base, index, scale);
        i += 1;
    }
    table
};

/// Compact SIB Index
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SibIndex(pub u8);

impl SibIndex {
    /// Creates a new `SibIndex` from the given base, index, and scale.
    ///
    /// TODO: If index register is `ESP`, it is invalid.
    pub fn from_sib(base: Option<GprIndex32>, index: GprIndex32, scale: Scale) -> Self {
        let base_bits = match base {
            Some(reg) => reg as u8,
            None => 8, // No base
        };
        let index_bits = match index {
            GprIndex32::EAX => 0,
            GprIndex32::ECX => 1,
            GprIndex32::EDX => 2,
            GprIndex32::EBX => 3,
            GprIndex32::EBP => 4, // ESP is invalid for index, so we skip it
            GprIndex32::ESI => 5,
            GprIndex32::EDI => 6,
            GprIndex32::ESP => 7, // TODO: This is invalid for SIB
        };
        let scale_bits = scale as u8;
        let sib_byte = (scale_bits << 6) | (index_bits * 9 + base_bits);
        SibIndex(sib_byte)
    }

    /// Converts the `SibIndex` back to its base, index, and scale components.
    #[inline]
    pub const fn to_sib(&self) -> (Option<GprIndex32>, GprIndex32, Scale) {
        COMPACT_SIB_TABLE[self.0 as usize]
    }
}

#[test]
fn test_sib_index() {
    for base in [
        Some(GprIndex32::EAX),
        Some(GprIndex32::ECX),
        Some(GprIndex32::EDX),
        Some(GprIndex32::EBX),
        Some(GprIndex32::ESP),
        Some(GprIndex32::EBP),
        Some(GprIndex32::ESI),
        Some(GprIndex32::EDI),
        None,
    ] {
        for index in [
            GprIndex32::EAX,
            GprIndex32::ECX,
            GprIndex32::EDX,
            GprIndex32::EBX,
            GprIndex32::EBP,
            GprIndex32::ESI,
            GprIndex32::EDI,
        ] {
            for scale in [Scale::Byte, Scale::Word, Scale::DWord, Scale::QWord] {
                let sib_index = SibIndex::from_sib(base, index, scale);
                let (base2, index2, scale2) = sib_index.to_sib();
                assert_eq!(base, base2);
                assert_eq!(index, index2);
                assert_eq!(scale, scale2);
            }
        }
    }
}
