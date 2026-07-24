//! TEK decompression library for Rust
//!
//! License: KL-01
//!
#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

const TEK_HEADER: [u8; 15] = [
    0xff, 0xff, 0xff, 0x01, 0x00, 0x00, 0x00, b'O', b'S', b'A', b'S', b'K', b'C', b'M', b'P',
];

const PB_ENTRY_LEN: usize = 56;
const PB_COUNT: usize = 16;
const LEN_LOW_LEN: usize = 8;
const LEN_MID_LEN: usize = 8;
const STATE_COUNT: usize = 12;
const P_SLOT_ROWS: usize = 4;
const P_SLOT_LEN: usize = 64;
const ALGN_LEN: usize = 64;
const SPDIS_ROWS: usize = 2;
const SPDIS_LEN: usize = 62;
const LENEXT_LEN: usize = 62;
const FCHGPRM_LEN: usize = 64;
const TBM_LEN: usize = 16;
const FIXED_PROB_LEN: usize = 2064;
const PB_OFFSET: usize = 0;
const ST_OFFSET: usize = PB_OFFSET + PB_COUNT * PB_ENTRY_LEN;
const LENSEL_OFFSET: usize = ST_OFFSET + STATE_COUNT * 4;
const LENHIGH_OFFSET: usize = LENSEL_OFFSET + 4;
const PSLOT_OFFSET: usize = LENHIGH_OFFSET + 2 * 256;
const ALGN_OFFSET: usize = PSLOT_OFFSET + P_SLOT_ROWS * P_SLOT_LEN;
const SPDIS_OFFSET: usize = ALGN_OFFSET + ALGN_LEN;
const LENEXT_OFFSET: usize = SPDIS_OFFSET + SPDIS_ROWS * SPDIS_LEN;
const REPG3_OFFSET: usize = LENEXT_OFFSET + LENEXT_LEN;
const FCHGPRM_OFFSET: usize = REPG3_OFFSET + 1;
const TBMT_OFFSET: usize = FCHGPRM_OFFSET + FCHGPRM_LEN;
const TBMM_OFFSET: usize = TBMT_OFFSET + TBM_LEN;
const FCHGLT_OFFSET: usize = TBMM_OFFSET + TBM_LEN;
const LIT_OFFSET: usize = FIXED_PROB_LEN;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TekError {
    InvalidFormat,
    UnsupportedMethod(u8),
    UnexpectedEof,
    InvalidDataSize,
    CorruptData(&'static str),
}

impl fmt::Display for TekError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => f.write_str("invalid TEK header or stream"),
            Self::UnsupportedMethod(method) => write!(f, "unsupported TEK method: 0x{method:02x}"),
            Self::UnexpectedEof => f.write_str("unexpected end of input"),
            Self::InvalidDataSize => f.write_str("invalid decompressed size"),
            Self::CorruptData(message) => f.write_str(message),
        }
    }
}

// impl std::error::Error for TekError {}

pub fn tek_getsize(input: &[u8]) -> Result<usize, TekError> {
    if input.len() < 16 {
        return Err(TekError::UnexpectedEof);
    }
    let method = input[0];
    if !matches!(method, 0x83 | 0x85 | 0x89) {
        return Err(TekError::UnsupportedMethod(method));
    }
    if input[1..16] != TEK_HEADER {
        return Err(TekError::InvalidFormat);
    }

    let mut cursor = 16;
    getnum_s7s(input, &mut cursor)
}

pub fn tek_decomp(input: &[u8]) -> Result<Vec<u8>, TekError> {
    let size = tek_getsize(input)?;
    let method = input[0];
    match method {
        0x83 => decode1(input, size),
        0x85 => decode2(input, size),
        0x89 => decode5(input, size),
        _ => Err(TekError::UnsupportedMethod(method)),
    }
}

fn ensure_header(input: &[u8], method: u8) -> Result<usize, TekError> {
    if input.len() < 16 {
        return Err(TekError::UnexpectedEof);
    }
    if input[0] != method || input[1..16] != TEK_HEADER {
        return Err(TekError::InvalidFormat);
    }
    Ok(16)
}

fn getnum_s7s(input: &[u8], cursor: &mut usize) -> Result<usize, TekError> {
    let mut value = 0usize;
    loop {
        let byte = *input.get(*cursor).ok_or(TekError::UnexpectedEof)? as usize;
        *cursor += 1;
        value = (value << 7) | byte;
        if (value & 1) != 0 {
            return Ok(value >> 1);
        }
    }
}

fn getnum_s7(input: &[u8], cursor: &mut usize) -> Result<usize, TekError> {
    let mut s = 0usize;
    let mut b = 0usize;
    let mut a = 1usize;
    loop {
        let byte = *input.get(*cursor).ok_or(TekError::UnexpectedEof)? as usize;
        *cursor += 1;
        s = (s << 7) | byte;
        if (s & 1) != 0 {
            return Ok((s >> 1) + b);
        }
        a = a.checked_shl(7).ok_or(TekError::InvalidDataSize)?;
        b = b.checked_add(a).ok_or(TekError::InvalidDataSize)?;
    }
}

fn decode1(input: &[u8], output_size: usize) -> Result<Vec<u8>, TekError> {
    let mut cursor = ensure_header(input, 0x83)?;
    let dsiz = getnum_s7s(input, &mut cursor)?;
    if dsiz != output_size {
        return Err(TekError::InvalidDataSize);
    }
    if dsiz == 0 {
        return Ok(Vec::new());
    }

    let hed = getnum_s7s(input, &mut cursor)?;
    let bsiz = 1usize
        .checked_shl((((hed >> 1) & 0x0f) + 8) as u32)
        .ok_or(TekError::InvalidDataSize)?;
    if dsiz > bsiz || (hed & 0x21) != 0x01 {
        return Err(TekError::CorruptData("invalid TEK1 header"));
    }
    if (hed & 0x40) != 0 {
        let _ = getnum_s7s(input, &mut cursor)?;
    }
    if getnum_s7s(input, &mut cursor)? != 0 {
        return Err(TekError::CorruptData("unsupported TEK1 auxiliary buffer"));
    }

    lzrestore_stk1(&input[cursor..], dsiz)
}

fn decode2(input: &[u8], output_size: usize) -> Result<Vec<u8>, TekError> {
    let mut cursor = ensure_header(input, 0x85)?;
    let dsiz = getnum_s7s(input, &mut cursor)?;
    if dsiz != output_size {
        return Err(TekError::InvalidDataSize);
    }
    if dsiz == 0 {
        return Ok(Vec::new());
    }

    let hed = getnum_s7s(input, &mut cursor)?;
    let bsiz = 1usize
        .checked_shl((((hed >> 1) & 0x0f) + 8) as u32)
        .ok_or(TekError::InvalidDataSize)?;
    if dsiz > bsiz || (hed & 0x21) != 0x01 {
        return Err(TekError::CorruptData("invalid TEK2 header"));
    }
    if (hed & 0x40) != 0 {
        let _ = getnum_s7s(input, &mut cursor)?;
    }

    lzrestore_stk2(&input[cursor..], dsiz)
}

fn decode5(input: &[u8], output_size: usize) -> Result<Vec<u8>, TekError> {
    let mut cursor = ensure_header(input, 0x89)?;
    let dsiz = getnum_s7s(input, &mut cursor)?;
    if dsiz != output_size {
        return Err(TekError::InvalidDataSize);
    }
    if dsiz == 0 {
        return Ok(Vec::new());
    }

    let hed_end_before = cursor;
    let hed = getnum_s7s(input, &mut cursor)?;
    if (hed & 1) == 0 {
        let start = cursor
            .checked_sub(1)
            .ok_or(TekError::CorruptData("invalid TEK5 header state"))?;
        return lzrestore_tek5(&input[start..], dsiz);
    }

    let bsiz = 1usize
        .checked_shl((((hed >> 1) & 0x0f) + 8) as u32)
        .ok_or(TekError::InvalidDataSize)?;
    if (hed & 0x20) != 0 {
        return Err(TekError::CorruptData("unsupported TEK5 mode"));
    }
    if bsiz == 256 {
        return lzrestore_tek5(&input[cursor..], dsiz);
    }
    if dsiz > bsiz {
        return Err(TekError::InvalidDataSize);
    }
    if (hed & 0x40) != 0 {
        let _ = getnum_s7s(input, &mut cursor)?;
    }
    if hed_end_before > input.len() {
        return Err(TekError::UnexpectedEof);
    }
    lzrestore_tek5(&input[cursor..], dsiz)
}

fn lzrestore_stk1(src: &[u8], output_size: usize) -> Result<Vec<u8>, TekError> {
    let mut cursor = 0usize;
    let mut out = Vec::with_capacity(output_size);

    while out.len() < output_size {
        let token = *src.get(cursor).ok_or(TekError::UnexpectedEof)? as usize;
        cursor += 1;
        let mut by = token & 0x0f;
        let mut lz = token >> 4;
        if by == 0 {
            by = getnum_s7s(src, &mut cursor)?;
        }
        if lz == 0 {
            lz = getnum_s7s(src, &mut cursor)?;
        }

        for _ in 0..by {
            let byte = *src.get(cursor).ok_or(TekError::UnexpectedEof)?;
            cursor += 1;
            if out.len() >= output_size {
                return Err(TekError::CorruptData("literal run exceeds output buffer"));
            }
            out.push(byte);
        }
        if out.len() >= output_size {
            break;
        }

        for _ in 0..lz {
            let token = *src.get(cursor).ok_or(TekError::UnexpectedEof)? as usize;
            cursor += 1;
            let mut cp = token >> 4;
            let mut ds = token & 0x0f;
            if (ds & 1) == 0 {
                loop {
                    let byte = *src.get(cursor).ok_or(TekError::UnexpectedEof)? as usize;
                    cursor += 1;
                    ds = (ds << 7) | byte;
                    if (ds & 1) != 0 {
                        break;
                    }
                }
            }
            let distance = !((ds >> 1) as i32);
            if cp == 0 {
                loop {
                    let byte = *src.get(cursor).ok_or(TekError::UnexpectedEof)? as usize;
                    cursor += 1;
                    cp = (cp << 7) | byte;
                    if (cp & 1) != 0 {
                        break;
                    }
                }
                cp >>= 1;
            }
            cp += 1;
            copy_match(&mut out, distance, cp, output_size)?;
        }
    }

    Ok(out)
}

fn lzrestore_stk2(src: &[u8], output_size: usize) -> Result<Vec<u8>, TekError> {
    let mut cursor = 0usize;
    let mut out = Vec::with_capacity(output_size);
    let mut repdis = [-1, -2, -3, -4];
    let mut bylz = 0u8;
    let mut cbylz = 0usize;

    if output_size == 0 {
        return Ok(out);
    }
    if getnum_s7s(src, &mut cursor)? != 0 {
        return Err(TekError::CorruptData("unsupported TEK2 prefix"));
    }

    while out.len() < output_size {
        let mut j = 0usize;
        loop {
            j += 1;
            if j >= 17 {
                j += getnum_s7s(src, &mut cursor)?;
                break;
            }
            if cbylz == 0 {
                bylz = *src.get(cursor).ok_or(TekError::UnexpectedEof)?;
                cursor += 1;
                cbylz = 8;
            }
            cbylz -= 1;
            let bit = bylz & 1;
            bylz >>= 1;
            if bit != 0 {
                break;
            }
        }
        for _ in 0..j {
            let byte = *src.get(cursor).ok_or(TekError::UnexpectedEof)?;
            cursor += 1;
            if out.len() >= output_size {
                return Err(TekError::CorruptData("literal run exceeds output buffer"));
            }
            out.push(byte);
        }
        if out.len() >= output_size {
            break;
        }

        let mut lz_count = 0usize;
        loop {
            lz_count += 1;
            if lz_count >= 17 {
                lz_count += getnum_s7s(src, &mut cursor)?;
                break;
            }
            if cbylz == 0 {
                bylz = *src.get(cursor).ok_or(TekError::UnexpectedEof)?;
                cursor += 1;
                cbylz = 8;
            }
            cbylz -= 1;
            let bit = bylz & 1;
            bylz >>= 1;
            if bit != 0 {
                break;
            }
        }

        for _ in 0..lz_count {
            let mut token = *src.get(cursor).ok_or(TekError::UnexpectedEof)? as usize;
            cursor += 1;
            let mut cp = token >> 4;
            token &= 0x0f;
            if (token & 1) == 0 {
                token |= (getnum_s7(src, &mut cursor)? + 1) << 4;
            }
            token >>= 1;

            let mut distance = !(token as i32 - 6);
            if token < 4 {
                distance = repdis[token];
            }
            if token == 4 {
                distance = repdis[0] - getnum_s7(src, &mut cursor)? as i32 - 1;
            }
            if token == 5 {
                distance = repdis[0] + getnum_s7(src, &mut cursor)? as i32 + 1;
            }
            if cp == 0 {
                cp = getnum_s7(src, &mut cursor)? + 16;
            }
            cp += 1;

            if token > 0 {
                if token > 1 {
                    if token > 2 {
                        repdis[3] = repdis[2];
                    }
                    repdis[2] = repdis[1];
                }
                repdis[1] = repdis[0];
                repdis[0] = distance;
            }

            copy_match(&mut out, distance, cp, output_size)?;
        }
    }

    Ok(out)
}

fn lzrestore_tek5(src: &[u8], output_size: usize) -> Result<Vec<u8>, TekError> {
    let first = *src.first().ok_or(TekError::UnexpectedEof)?;
    let fl = first & 0x0f;
    let mut flags = match fl {
        0x01 => -1,
        0x05 => -2,
        0x09 => 0,
        _ => return Err(TekError::CorruptData("invalid TEK5 property flags")),
    };

    let mut cursor = 1usize;
    let mut prop0 = first >> 4;
    if prop0 == 0 {
        prop0 = *src.get(cursor).ok_or(TekError::UnexpectedEof)?;
        cursor += 1;
    } else if flags == -1 {
        prop0 = match prop0 {
            1 => 0x5d,
            2 => 0x00,
            _ => return Err(TekError::CorruptData("invalid stk5 property table")),
        };
    } else {
        prop0 = match prop0 {
            1 => 0x00,
            _ => return Err(TekError::CorruptData("invalid z1/z2 property table")),
        };
    }

    let mut lp = (prop0 / (9 * 5)) as usize;
    prop0 %= 9 * 5;
    let mut pb = (prop0 / 9) as usize;
    let lc = (prop0 % 9) as usize;

    if flags == 0 {
        flags = *src.get(cursor).ok_or(TekError::UnexpectedEof)? as i32;
        cursor += 1;
    }
    if flags == -1 {
        core::mem::swap(&mut lp, &mut pb);
    }

    let mut decoder = Tek5Decoder::new(&src[cursor..], output_size, lc, pb, lp, flags)?;
    decoder.decmain()
}

fn copy_match(
    out: &mut Vec<u8>,
    distance: i32,
    mut count: usize,
    output_size: usize,
) -> Result<(), TekError> {
    if distance >= 0 {
        return Err(TekError::CorruptData("invalid match distance"));
    }
    if (out.len() as i64) + (distance as i64) < 0 {
        return Err(TekError::InvalidDataSize);
    }
    let max_copy = output_size.saturating_sub(out.len());
    if count > max_copy {
        count = max_copy;
    }
    for _ in 0..count {
        let src_index = ((out.len() as i64) + (distance as i64)) as usize;
        let value = *out.get(src_index).ok_or(TekError::InvalidDataSize)?;
        out.push(value);
    }
    Ok(())
}

#[derive(Clone, Copy, Default)]
struct BitModel {
    t: u8,
    m: u8,
    s: u8,
    prb0: u32,
    prb1: u32,
    tmsk: u32,
    ntm: u32,
    lt: u32,
    lt0: u32,
}

struct Tek5Decoder<'a> {
    src: &'a [u8],
    cursor: usize,
    range: u32,
    code: u32,
    rmsk: u32,
    probs: Vec<u32>,
    bm: [BitModel; 32],
    ptbm: [usize; 14],
    lc: usize,
    pb: usize,
    lp: usize,
    flags: i32,
    output_size: usize,
    lit0cntmsk: i32,
}

impl<'a> Tek5Decoder<'a> {
    fn new(
        src: &'a [u8],
        output_size: usize,
        lc: usize,
        pb: usize,
        lp: usize,
        flags: i32,
    ) -> Result<Self, TekError> {
        if src.len() < 4 {
            return Err(TekError::UnexpectedEof);
        }
        let ctx_shift = lc + lp;
        let ctx_count = 1usize
            .checked_shl(ctx_shift as u32)
            .ok_or(TekError::InvalidDataSize)?;
        let total_probs = FIXED_PROB_LEN
            .checked_add(
                0x300usize
                    .checked_mul(ctx_count)
                    .ok_or(TekError::InvalidDataSize)?,
            )
            .ok_or(TekError::InvalidDataSize)?;
        let mut decoder = Self {
            src,
            cursor: 4,
            range: u32::MAX,
            code: u32::from(src[0]) << 24
                | u32::from(src[1]) << 16
                | u32::from(src[2]) << 8
                | u32::from(src[3]),
            rmsk: u32::MAX,
            probs: vec![1 << 15; total_probs],
            bm: [BitModel::default(); 32],
            ptbm: [0; 14],
            lc,
            pb,
            lp,
            flags,
            output_size,
            lit0cntmsk: 0x78,
        };
        decoder.init_models()?;
        Ok(decoder)
    }

    fn init_models(&mut self) -> Result<(), TekError> {
        for i in 0..32 {
            self.bm[i].lt = if i >= 4 { 1 } else { 0 };
            self.bm[i].lt0 = if i < 24 { 16 * 1024 } else { 8 * 1024 };
            self.bm[i].s = 0;
            self.bm[i].t = 5;
            self.bm[i].m = 5;
        }

        let stk = self.flags == -1;
        if stk {
            self.rmsk = u32::MAX << 11;
            for model in &mut self.bm {
                model.lt = 0;
            }
            for i in 0..14 {
                self.ptbm[i] = 0;
            }
        } else {
            let mut pt = [0u8; 14];
            const PT1: [u8; 14] = [8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 18, 18, 18, 8];
            const PT2: [u8; 14] = [8, 8, 10, 11, 12, 12, 14, 15, 16, 16, 18, 18, 20, 21];

            self.bm[1].t = 5;
            self.bm[1].m = 3;
            self.bm[2].t = 9;
            self.bm[2].m = 2;
            if (self.flags & 0x40) != 0 {
                self.bm[3].t = 0;
                self.bm[3].m = 1;
                self.probs[FCHGLT_OFFSET] = 0xffff;
            }
            self.bm[22].t = 0;
            self.bm[22].m = 1;
            self.probs[REPG3_OFFSET] = 0xffff;

            if self.flags == -2 {
                self.bm[22].lt = 0;
                pt.copy_from_slice(&PT1);
            } else {
                pt.copy_from_slice(&PT2);
                self.lit0cntmsk = (((7 >> (self.flags & 3)) << 4) | 8) as i32;
                pt[1] = 8 + u8::from((self.flags & 0x04) != 0);
                pt[5] = 12 + u8::from((self.flags & 0x08) != 0);
                pt[9] = 16 + u8::from((self.flags & 0x10) != 0);
                pt[11] = 18 + u8::from((self.flags & 0x20) != 0);
            }

            for (i, value) in pt.into_iter().enumerate() {
                self.ptbm[i] = value as usize;
            }
        }

        for i in 0..32 {
            self.set_bm(i, self.bm[i].t, self.bm[i].m);
        }
        Ok(())
    }

    fn set_bm(&mut self, index: usize, t: u8, m: u8) {
        let model = &mut self.bm[index];
        model.t = t;
        model.m = m;
        model.prb1 = (!0u32) << (u32::from(m) + u32::from(t));
        model.prb0 = !model.prb1;
        model.prb1 |= 1u32 << u32::from(t);
        model.tmsk = ((!0u32) << u32::from(t)) & 0xffff;
        model.prb0 &= model.tmsk;
        model.prb1 &= model.tmsk;
        model.ntm = !model.tmsk;
    }

    fn decmain(&mut self) -> Result<Vec<u8>, TekError> {
        const STATE_TABLE: [usize; 12] = [0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 4, 5];
        let stk = i32::from(self.flags == -1);
        let lcr = 8usize
            .checked_sub(self.lc)
            .ok_or(TekError::CorruptData("invalid literal context"))?;
        let m_pos = (1usize
            .checked_shl(self.pb as u32)
            .ok_or(TekError::InvalidDataSize)?)
            - 1;
        let m_lp = (1usize
            .checked_shl(self.lp as u32)
            .ok_or(TekError::InvalidDataSize)?)
            - 1;
        let ctx_shift = self.lc + self.lp;
        let lit1_base = LIT_OFFSET
            + ((256usize
                .checked_shl(ctx_shift as u32)
                .ok_or(TekError::InvalidDataSize)?)
                - 2);

        if (self.rdget1(self.pb_mch_index(0, 0), 0x71, 0, self.ptbm[0])? ^ stk) == 0 {
            return Err(TekError::CorruptData("invalid initial literal flag"));
        }

        let mut out = Vec::with_capacity(self.output_size);
        out.push((self.rdget1(self.lit_index(0), self.lit0cntmsk, 1, 24)? & 0xff) as u8);

        let mut rep = [!0i32, !1i32, !2i32, !3i32];
        let mut pmch = 0i32;
        let mut state = 0usize;
        let mut pos = 1usize;

        while pos < self.output_size {
            let s_pos = pos & m_pos;
            if (self.rdget1(
                self.pb_mch_index(s_pos, state),
                0x71,
                0,
                self.ptbm[usize::from(state > 0)],
            )? ^ stk)
                != 0
            {
                let literal_context =
                    (((out[out.len() - 1] as usize) >> lcr) | ((pos & m_lp) << self.lc)) << 8;
                state = STATE_TABLE[state];

                let byte = if pmch == 0 {
                    (self.rdget1(self.lit_index(literal_context), self.lit0cntmsk, 1, 24)? & 0xff)
                        as u8
                } else {
                    let mut bm_index = 24usize;
                    let mut tree = 1i32;
                    let mut remaining = 8i32;
                    pmch = self.get_relative_byte(&out, rep[0])? as i32;
                    while remaining > 0 {
                        let lit1_index = lit1_base
                            .checked_add(
                                (((literal_context as i32 + tree) << 1) | (pmch >> 7)) as usize,
                            )
                            .ok_or(TekError::InvalidDataSize)?;
                        tree = tree + tree + self.rdget1(lit1_index, 0x71, 0, self.ptbm[2])?;
                        remaining -= 1;
                        if (remaining & (self.lit0cntmsk >> 4)) == 0 {
                            bm_index += 1;
                        }
                        if ((((pmch >> 7) ^ tree) & 1) != 0) && remaining != 0 {
                            tree = self.rdget1(
                                self.lit_index(literal_context + tree as usize - 1),
                                remaining | (self.lit0cntmsk & 0x70),
                                tree,
                                bm_index,
                            )?;
                            break;
                        }
                        pmch <<= 1;
                    }
                    pmch = 0;
                    (tree & 0xff) as u8
                };

                out.push(byte);
                pos += 1;
                continue;
            }

            pmch |= 1;
            let mut match_len: i32;

            if (self.rdget1(self.st_index(state, 0), 0x71, 0, self.ptbm[13])? ^ stk) != 0 {
                rep[3] = rep[2];
                rep[2] = rep[1];
                rep[1] = rep[0];
                match_len = self.getlen5(0, s_pos, stk)?;
                state = if state < 7 { 7 } else { 10 };

                let mut slot_row = match_len;
                if slot_row >= 4 {
                    slot_row = 3;
                }
                let slot = self.rdget1(
                    self.pslot_index(slot_row as usize, 0),
                    0x76,
                    1,
                    self.ptbm[8 + usize::from(slot_row == 3)],
                )? & 0x3f;
                rep[0] = slot;

                if slot >= 4 {
                    let mut k = (slot >> 1) - 1;
                    rep[0] = (2 | (slot & 1)) << k;
                    if slot < 14 {
                        let bits = self.rdget1(
                            self.spdis_index((slot & 1) as usize, (1usize << (k as usize)) - 2),
                            k | 0x70,
                            1,
                            self.ptbm[10 + usize::from(k >= 4)],
                        )?;
                        rep[0] |= revbit(bits as u32, k as usize) as i32;
                    } else if stk == 0 {
                        k -= 6;
                        if k != 0 {
                            rep[0] |= self.rdget0(k, !0)? << 6;
                        }
                        rep[0] |= revbit(
                            self.rdget1(self.algn_index(0), 0x76, 1, self.ptbm[12])? as u32,
                            6,
                        ) as i32;
                    } else {
                        rep[0] |= self.rdget0(k - 4, !0)? << 4;
                        rep[0] |= revbit(
                            self.rdget1(self.algn_index(0), 0x74, 1, self.ptbm[12])? as u32,
                            4,
                        ) as i32;
                    }
                }
                rep[0] = !rep[0];
            } else {
                if (self.rdget1(self.st_index(state, 1), 0x71, 0, self.ptbm[13])? ^ stk) != 0 {
                    match_len = -1;
                    if self.rdget1(self.pb_rep0l1_index(s_pos, state), 0x71, 0, self.ptbm[13])? == 0
                    {
                        state = if state < 7 { 9 } else { 11 };
                        match_len += 2;
                        let copy_count = (self.output_size - pos).min(match_len as usize);
                        copy_match(&mut out, rep[0], copy_count, self.output_size)?;
                        pos += copy_count;
                        continue;
                    }
                } else {
                    let distance =
                        if (self.rdget1(self.st_index(state, 2), 0x71, 0, self.ptbm[13])? ^ stk)
                            != 0
                        {
                            rep[1]
                        } else {
                            let distance =
                                if (self.rdget1(self.st_index(state, 3), 0x71, 0, self.ptbm[13])?
                                    ^ stk)
                                    != 0
                                {
                                    rep[2]
                                } else {
                                    if stk == 0 && self.rdget1(REPG3_OFFSET, 0x71, 0, 22)? == 0 {
                                        return Err(TekError::CorruptData("invalid repg3 state"));
                                    }
                                    let distance = rep[3];
                                    rep[3] = rep[2];
                                    distance
                                };
                            rep[2] = rep[1];
                            distance
                        };
                    rep[1] = rep[0];
                    rep[0] = distance;
                }
                match_len = self.getlen5(1, s_pos, stk)?;
                state = if state < 7 { 8 } else { 11 };
            }

            match_len += 2;
            if (pos as i64) + (rep[0] as i64) < 0 {
                return Err(TekError::InvalidDataSize);
            }
            let copy_count = (self.output_size - pos).min(match_len as usize);
            copy_match(&mut out, rep[0], copy_count, self.output_size)?;
            pos += copy_count;
        }

        Ok(out)
    }

    fn rdget0(&mut self, mut n: i32, mut i: i32) -> Result<i32, TekError> {
        while n > 0 {
            while self.range < (1 << 24) {
                self.range <<= 8;
                self.code = (self.code << 8) | u32::from(self.read_byte()?);
            }
            self.range >>= 1;
            i += i;
            if self.code >= self.range {
                self.code -= self.range;
                i |= 1;
            }
            n -= 1;
        }
        Ok(!i)
    }

    fn rdget1(
        &mut self,
        prob0: usize,
        mut n: i32,
        mut j: i32,
        mut bm_index: usize,
    ) -> Result<i32, TekError> {
        let nm = n >> 4;
        n &= 0x0f;
        let base = prob0 as i32 - j;

        while n > 0 {
            let prob_index = (base + j) as usize;
            let mut p = *self
                .probs
                .get(prob_index)
                .ok_or(TekError::InvalidDataSize)?;

            if self.bm[bm_index].lt > 0 {
                self.bm[bm_index].lt -= 1;
                if self.bm[bm_index].lt == 0 {
                    if self.rdget1(FCHGLT_OFFSET, 0x71, 0, 3)? == 0 {
                        return Err(TekError::CorruptData(
                            "dynamic probability change is unsupported",
                        ));
                    }
                    let current_s = self.bm[bm_index].s as usize;
                    let next_s =
                        self.rdget1(FCHGPRM_OFFSET + bm_index * 2 + current_s, 0x71, 0, 1)?;
                    self.bm[bm_index].s = next_s as u8;
                    if next_s == 0 {
                        let next_t = self.rdget1(TBMT_OFFSET, 0x74, 1, 2)? & 15;
                        if next_t == 15 {
                            return Err(TekError::CorruptData("invalid probability table update"));
                        }
                        let next_m = ((self.rdget1(TBMM_OFFSET, 0x74, 1, 2)? - 1) & 15) + 1;
                        self.set_bm(bm_index, next_t as u8, next_m as u8);
                    }
                    self.bm[bm_index].lt = self.bm[bm_index].lt0;
                }

                let model = self.bm[bm_index];
                if p < model.prb0 {
                    p = model.prb0;
                    self.probs[prob_index] = p;
                }
                if p > model.prb1 {
                    p = model.prb1;
                    self.probs[prob_index] = p;
                }
                if (p & model.ntm) != 0 {
                    p &= model.tmsk;
                    self.probs[prob_index] = p;
                }
            }

            while self.range < (1 << 24) {
                self.range <<= 8;
                self.code = (self.code << 8) | u32::from(self.read_byte()?);
            }
            j += j;
            let split = (((self.range & self.rmsk) as u64) * (p as u64) >> 16) as u32;
            if self.code < split {
                j |= 1;
                self.range = split;
                let delta = ((0x10000 - p) >> self.bm[bm_index].m) & self.bm[bm_index].tmsk;
                self.probs[prob_index] = self.probs[prob_index].wrapping_add(delta);
            } else {
                self.range -= split;
                self.code -= split;
                let delta = (p >> self.bm[bm_index].m) & self.bm[bm_index].tmsk;
                self.probs[prob_index] = self.probs[prob_index].wrapping_sub(delta);
            }

            n -= 1;
            if (n & nm) == 0 {
                bm_index += 1;
            }
        }

        Ok(j)
    }

    fn getlen5(&mut self, m: usize, s_pos: usize, stk: i32) -> Result<i32, TekError> {
        if (self.rdget1(self.lensel_index(m, 0), 0x71, 0, self.ptbm[3])? ^ stk) != 0 {
            Ok((self.rdget1(self.pb_lenlow_index(s_pos, m, 0), 0x73, 1, self.ptbm[4])? & 7) as i32)
        } else if (self.rdget1(self.lensel_index(m, 1), 0x71, 0, self.ptbm[3])? ^ stk) != 0 {
            Ok(self.rdget1(self.pb_lenmid_index(s_pos, m, 0), 0x73, 1, self.ptbm[5])?)
        } else {
            let mut value =
                self.rdget1(self.lenhigh_index(m, 0), 0x78, 1, self.ptbm[6])? - (256 + 256 - 8);
            if value > 0 {
                if value < 6 && stk == 0 {
                    value = self.rdget1(
                        self.lenext_index((1usize << (value as usize)) - 2),
                        value | 0x70,
                        1,
                        self.ptbm[7],
                    )? - 1;
                } else {
                    value = self.rdget0(value, !1)? - 1;
                }
                value = self.rdget0(value, !1)? - 1;
            }
            Ok(value + 256 - 8 + 16)
        }
    }

    fn read_byte(&mut self) -> Result<u8, TekError> {
        let byte = *self.src.get(self.cursor).ok_or(TekError::UnexpectedEof)?;
        self.cursor += 1;
        Ok(byte)
    }

    fn get_relative_byte(&self, out: &[u8], distance: i32) -> Result<u8, TekError> {
        if distance >= 0 {
            return Err(TekError::CorruptData("invalid relative distance"));
        }
        let index = (out.len() as i64) + (distance as i64);
        if index < 0 {
            return Err(TekError::InvalidDataSize);
        }
        out.get(index as usize)
            .copied()
            .ok_or(TekError::InvalidDataSize)
    }

    fn pb_mch_index(&self, pb: usize, state: usize) -> usize {
        PB_OFFSET + pb * PB_ENTRY_LEN + state * 2
    }

    fn pb_rep0l1_index(&self, pb: usize, state: usize) -> usize {
        self.pb_mch_index(pb, state) + 1
    }

    fn pb_lenlow_index(&self, pb: usize, m: usize, index: usize) -> usize {
        PB_OFFSET + pb * PB_ENTRY_LEN + 24 + m * LEN_LOW_LEN + index
    }

    fn pb_lenmid_index(&self, pb: usize, m: usize, index: usize) -> usize {
        PB_OFFSET + pb * PB_ENTRY_LEN + 40 + m * LEN_MID_LEN + index
    }

    fn st_index(&self, state: usize, field: usize) -> usize {
        ST_OFFSET + state * 4 + field
    }

    fn lensel_index(&self, m: usize, which: usize) -> usize {
        LENSEL_OFFSET + m * 2 + which
    }

    fn lenhigh_index(&self, m: usize, index: usize) -> usize {
        LENHIGH_OFFSET + m * 256 + index
    }

    fn pslot_index(&self, row: usize, index: usize) -> usize {
        PSLOT_OFFSET + row * P_SLOT_LEN + index
    }

    fn algn_index(&self, index: usize) -> usize {
        ALGN_OFFSET + index
    }

    fn spdis_index(&self, row: usize, index: usize) -> usize {
        SPDIS_OFFSET + row * SPDIS_LEN + index
    }

    fn lenext_index(&self, index: usize) -> usize {
        LENEXT_OFFSET + index
    }

    fn lit_index(&self, index: usize) -> usize {
        LIT_OFFSET + index
    }
}

fn revbit(mut data: u32, len: usize) -> u32 {
    let mut rev = 0u32;
    for _ in 0..len {
        rev = (rev << 1) | (data & 1);
        data >>= 1;
    }
    rev
}

#[cfg(test)]
mod tests;
