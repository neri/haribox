//! Japanese Language support for Haribote OS

#[allow(dead_code, non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LangMode {
    Ascii = 0,
    ShiftJIS,
    EUC,
}

/// A String encoded in Japanese Industrial Standards
pub struct JisString<'a>(&'a [u8]);

impl<'a> JisString<'a> {
    #[inline]
    pub const fn from_bytes(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }

    #[inline]
    pub fn to_str(&self) -> Option<&'a str> {
        core::str::from_utf8(self.0).ok()
    }

    #[inline]
    pub fn chars(&'a self, lang_mode: LangMode) -> impl Iterator<Item = JisChar> + 'a {
        JisChars::new(self, lang_mode)
    }
}

impl core::fmt::Display for JisString<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.to_str().map(|v| f.write_str(v));
        Ok(())
    }
}

pub struct JisChars<'a> {
    data: &'a JisString<'a>,
    lang_mode: LangMode,
    index: usize,
}

impl<'a> JisChars<'a> {
    #[inline]
    pub const fn new(data: &'a JisString<'a>, lang_mode: LangMode) -> Self {
        Self {
            data,
            lang_mode,
            index: 0,
        }
    }
}

impl Iterator for JisChars<'_> {
    type Item = JisChar;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(lead) = self.data.0.get(self.index) {
            let lead = *lead;
            self.index += 1;
            match self.lang_mode {
                LangMode::Ascii => Some(JisChar::ANK(lead)),
                LangMode::ShiftJIS => {
                    if (lead >= 0x81 && lead <= 0x9f) || (lead >= 0xe0 && lead <= 0xef) {
                        if let Some(trail) = self.data.0.get(self.index) {
                            let trail = *trail;
                            self.index += 1;
                            let mut k = if lead >= 0x81 && lead <= 0x9f {
                                (lead - 0x81) * 2
                            } else {
                                (lead - 0xe0) * 2 + 62
                            };
                            let t;
                            if trail >= 0x40 && trail <= 0x7e {
                                t = trail - 0x40;
                            } else if trail >= 0x80 && trail <= 0x9e {
                                t = trail - 0x80 + 63;
                            } else {
                                t = trail - 0x9f;
                                k += 1;
                            }
                            Some(JisChar::Kanji((k as u16) * 94 + (t as u16)))
                        } else {
                            // invalid character sequence
                            None
                        }
                    } else {
                        Some(JisChar::ANK(lead))
                    }
                }
                LangMode::EUC => {
                    if lead >= 0x81 && lead <= 0xfe {
                        if let Some(trail) = self.data.0.get(self.index) {
                            let trail = *trail;
                            self.index += 1;
                            Some(JisChar::Kanji(
                                (lead as u16 - 0xa1) * 94 + (trail as u16 - 0xa1),
                            ))
                        } else {
                            None
                        }
                    } else {
                        Some(JisChar::ANK(lead))
                    }
                }
            }
        } else {
            None
        }
    }
}

#[allow(non_camel_case_types)]
pub enum JisChar {
    /// Alphabet Numeric Kana
    ANK(u8),
    /// Jis Kanji Code
    Kanji(u16),
}
