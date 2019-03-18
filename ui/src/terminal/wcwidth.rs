/*
 * This is an implementation of wcwidth() and wcswidth() as defined in
 * "The Single UNIX Specification, Version 2, The Open Group, 1997"
 * <http://www.UNIX-systems.org/online.html>
 *
 * Markus Kuhn -- 2001-09-08 -- public domain
 */

// TODO: Spacing widths
// Update to Unicode 12

#[macro_export]
macro_rules! big_if_true {
    ($a:expr) => {
        if $a {
            1
        } else {
            0
        }
    };
}

type WChar = u32;
type Interval = (WChar, WChar);

pub struct CodePointsIterator<'a> {
    rest: &'a [u8],
}

/*
 * UTF-8 uses a system of binary prefixes, in which the high bits of each byte mark whether it’s a single byte, the beginning of a multi-byte sequence, or a continuation byte; the remaining bits, concatenated, give the code point index. This table shows how it works:
 *
 * UTF-8 (binary) 	                Code point (binary) 	Range
 * 0xxxxxxx                     	xxxxxxx 	        U+0000–U+007F
 * 110xxxxx 10yyyyyy 	                xxxxxyyyyyy 	        U+0080–U+07FF
 * 1110xxxx 10yyyyyy 10zzzzzz 	        xxxxyyyyyyzzzzzz 	U+0800–U+FFFF
 * 11110xxx 10yyyyyy 10zzzzzz 10wwwwww 	xxxyyyyyyzzzzzzwwwwww 	U+10000–U+10FFFF
 *
 */
impl<'a> Iterator for CodePointsIterator<'a> {
    type Item = WChar;

    fn next(&mut self) -> Option<WChar> {
        println!("rest = {:?}", self.rest);
        if self.rest.is_empty() {
            return None;
        }
        /* Input is UTF-8 valid strings, guaranteed by Rust's std */
        if self.rest[0] & 0b1000_0000 == 0x0 {
            let ret: WChar = self.rest[0] as WChar;
            self.rest = &self.rest[1..];
            return Some(ret);
        }
        if self.rest[0] & 0b1110_0000 == 0b1100_0000 {
            let ret: WChar = (self.rest[0] as WChar & 0b0001_1111).rotate_left(6)
                + (self.rest[1] as WChar & 0b0111_1111);
            self.rest = &self.rest[2..];
            return Some(ret);
        }

        if self.rest[0] & 0b1111_0000 == 0b1110_0000 {
            let ret: WChar = (self.rest[0] as WChar & 0b0000_0111).rotate_left(12)
                + (self.rest[1] as WChar & 0b0011_1111).rotate_left(6)
                + (self.rest[2] as WChar & 0b0011_1111);
            self.rest = &self.rest[3..];
            return Some(ret);
        }

        let ret: WChar = (self.rest[0] as WChar & 0b0000_0111).rotate_left(18)
            + (self.rest[1] as WChar & 0b0011_1111).rotate_left(12)
            + (self.rest[2] as WChar & 0b0011_1111).rotate_left(6)
            + (self.rest[3] as WChar & 0b0011_1111);
        self.rest = &self.rest[4..];
        Some(ret)
    }
}
pub trait CodePointsIter {
    fn code_points(&self) -> CodePointsIterator;
}

impl CodePointsIter for str {
    fn code_points(&self) -> CodePointsIterator {
        CodePointsIterator {
            rest: self.as_bytes(),
        }
    }
}
impl CodePointsIter for &str {
    fn code_points(&self) -> CodePointsIterator {
        CodePointsIterator {
            rest: self.as_bytes(),
        }
    }
}

/* auxiliary function for binary search in Interval table */
fn bisearch(ucs: WChar, table: &'static [Interval]) -> bool {
    let mut min = 0;
    let mut mid;

    let mut max = table.len() - 1;

    if ucs < table[0].0 || ucs > table[max].1 {
        return true;
    }
    while max >= min {
        mid = (min + max) / 2;
        if ucs > table[mid].1 {
            min = mid + 1;
        } else if ucs < table[mid].0 {
            max = mid - 1;
        } else {
            return true;
        }
    }

    return false;
}

/* The following functions define the column width of an ISO 10646
 * character as follows:
 *
 *    - The null character (U+0000) has a column width of 0.
 *
 *    - Other C0/C1 control characters and DEL will lead to a return
 *      value of -1.
 *
 *    - Non-spacing and enclosing combining characters (general
 *      category code Mn or Me in the Unicode database) have a
 *      column width of 0.
 *
 *    - Other format characters (general category code Cf in the Unicode
 *      database) and ZERO WIDTH SPACE (U+200B) have a column width of 0.
 *
 *    - Hangul Jamo medial vowels and final consonants (U+1160-U+11FF)
 *      have a column width of 0.
 *
 *    - Spacing characters in the East Asian Wide (W) or East Asian
 *      FullWidth (F) category as defined in Unicode Technical
 *      Report #11 have a column width of 2.
 *
 *    - All remaining characters (including all printable
 *      ISO 8859-1 and WGL4 characters, Unicode control characters,
 *      etc.) have a column width of 1.
 *
 * This implementation assumes that wchar_t characters are encoded
 * in ISO 10646.
 */

pub fn wcwidth(ucs: WChar) -> Option<usize> {
    /* sorted list of non-overlapping intervals of non-spacing characters */
    let combining: &'static [Interval] = &[
        (0x0300, 0x034E),
        (0x0360, 0x0362),
        (0x0483, 0x0486),
        (0x0488, 0x0489),
        (0x0591, 0x05A1),
        (0x05A3, 0x05B9),
        (0x05BB, 0x05BD),
        (0x05BF, 0x05BF),
        (0x05C1, 0x05C2),
        (0x05C4, 0x05C4),
        (0x064B, 0x0655),
        (0x0670, 0x0670),
        (0x06D6, 0x06E4),
        (0x06E7, 0x06E8),
        (0x06EA, 0x06ED),
        (0x070F, 0x070F),
        (0x0711, 0x0711),
        (0x0730, 0x074A),
        (0x07A6, 0x07B0),
        (0x0901, 0x0902),
        (0x093C, 0x093C),
        (0x0941, 0x0948),
        (0x094D, 0x094D),
        (0x0951, 0x0954),
        (0x0962, 0x0963),
        (0x0981, 0x0981),
        (0x09BC, 0x09BC),
        (0x09C1, 0x09C4),
        (0x09CD, 0x09CD),
        (0x09E2, 0x09E3),
        (0x0A02, 0x0A02),
        (0x0A3C, 0x0A3C),
        (0x0A41, 0x0A42),
        (0x0A47, 0x0A48),
        (0x0A4B, 0x0A4D),
        (0x0A70, 0x0A71),
        (0x0A81, 0x0A82),
        (0x0ABC, 0x0ABC),
        (0x0AC1, 0x0AC5),
        (0x0AC7, 0x0AC8),
        (0x0ACD, 0x0ACD),
        (0x0B01, 0x0B01),
        (0x0B3C, 0x0B3C),
        (0x0B3F, 0x0B3F),
        (0x0B41, 0x0B43),
        (0x0B4D, 0x0B4D),
        (0x0B56, 0x0B56),
        (0x0B82, 0x0B82),
        (0x0BC0, 0x0BC0),
        (0x0BCD, 0x0BCD),
        (0x0C3E, 0x0C40),
        (0x0C46, 0x0C48),
        (0x0C4A, 0x0C4D),
        (0x0C55, 0x0C56),
        (0x0CBF, 0x0CBF),
        (0x0CC6, 0x0CC6),
        (0x0CCC, 0x0CCD),
        (0x0D41, 0x0D43),
        (0x0D4D, 0x0D4D),
        (0x0DCA, 0x0DCA),
        (0x0DD2, 0x0DD4),
        (0x0DD6, 0x0DD6),
        (0x0E31, 0x0E31),
        (0x0E34, 0x0E3A),
        (0x0E47, 0x0E4E),
        (0x0EB1, 0x0EB1),
        (0x0EB4, 0x0EB9),
        (0x0EBB, 0x0EBC),
        (0x0EC8, 0x0ECD),
        (0x0F18, 0x0F19),
        (0x0F35, 0x0F35),
        (0x0F37, 0x0F37),
        (0x0F39, 0x0F39),
        (0x0F71, 0x0F7E),
        (0x0F80, 0x0F84),
        (0x0F86, 0x0F87),
        (0x0F90, 0x0F97),
        (0x0F99, 0x0FBC),
        (0x0FC6, 0x0FC6),
        (0x102D, 0x1030),
        (0x1032, 0x1032),
        (0x1036, 0x1037),
        (0x1039, 0x1039),
        (0x1058, 0x1059),
        (0x1160, 0x11FF),
        (0x17B7, 0x17BD),
        (0x17C6, 0x17C6),
        (0x17C9, 0x17D3),
        (0x180B, 0x180E),
        (0x18A9, 0x18A9),
        (0x200B, 0x200F),
        (0x202A, 0x202E),
        (0x206A, 0x206F),
        (0x20D0, 0x20E3),
        (0x302A, 0x302F),
        (0x3099, 0x309A),
        (0xFB1E, 0xFB1E),
        (0xFE20, 0xFE23),
        (0xFEFF, 0xFEFF),
        (0xFFF9, 0xFFFB),
    ];

    /* test for 8-bit control characters */
    if ucs == 0 {
        return Some(0);
    }
    if ucs < 32 || (ucs >= 0x7f && ucs < 0xa0) {
        return None;
    }

    /* binary search in table of non-spacing characters */
    if bisearch(ucs, combining) {
        return Some(1);
    }

    /* if we arrive here, ucs is not a combining or C0/C1 control character */

    return Some(
        1 + big_if_true!(
            ucs >= 0x1100
                && (ucs <= 0x115f ||                    /* Hangul Jamo init. consonants */
      (ucs >= 0x2e80 && ucs <= 0xa4cf && (ucs & !0x0011) != 0x300a &&
       ucs != 0x303f) ||                  /* CJK ... Yi */
      (ucs >= 0xac00 && ucs <= 0xd7a3) || /* Hangul Syllables */
      (ucs >= 0xf900 && ucs <= 0xfaff) || /* CJK Compatibility Ideographs */
      (ucs >= 0xfe30 && ucs <= 0xfe6f) || /* CJK Compatibility Forms */
      (ucs >= 0xff00 && ucs <= 0xff5f) || /* Fullwidth Forms */
      (ucs >= 0xffe0 && ucs <= 0xffe6) ||
      (ucs >= 0x20000 && ucs <= 0x2ffff))
        ),
    );
}

fn wcswidth(mut pwcs: WChar, mut n: usize) -> Option<usize> {
    let mut width = 0;

    while pwcs > 0 && n > 0 {
        if let Some(w) = wcwidth(pwcs) {
            width += w;
        } else {
            return None;
        }

        pwcs += 1;
        n -= 1;
    }

    return Some(width);
}

/*
 * The following function is the same as wcwidth(), except that
 * spacing characters in the East Asian Ambiguous (A) category as
 * defined in Unicode Technical Report #11 have a column width of 2.
 * This experimental variant might be useful for users of CJK legacy
 * encodings who want to migrate to UCS. It is not otherwise
 * recommended for general use.
 */
pub fn wcwidth_cjk(ucs: WChar) -> Option<usize> {
    /* sorted list of non-overlapping intervals of East Asian Ambiguous
     * characters */
    let ambiguous: &'static [Interval] = &[
        (0x00A1, 0x00A1),
        (0x00A4, 0x00A4),
        (0x00A7, 0x00A8),
        (0x00AA, 0x00AA),
        (0x00AD, 0x00AE),
        (0x00B0, 0x00B4),
        (0x00B6, 0x00BA),
        (0x00BC, 0x00BF),
        (0x00C6, 0x00C6),
        (0x00D0, 0x00D0),
        (0x00D7, 0x00D8),
        (0x00DE, 0x00E1),
        (0x00E6, 0x00E6),
        (0x00E8, 0x00EA),
        (0x00EC, 0x00ED),
        (0x00F0, 0x00F0),
        (0x00F2, 0x00F3),
        (0x00F7, 0x00FA),
        (0x00FC, 0x00FC),
        (0x00FE, 0x00FE),
        (0x0101, 0x0101),
        (0x0111, 0x0111),
        (0x0113, 0x0113),
        (0x011B, 0x011B),
        (0x0126, 0x0127),
        (0x012B, 0x012B),
        (0x0131, 0x0133),
        (0x0138, 0x0138),
        (0x013F, 0x0142),
        (0x0144, 0x0144),
        (0x0148, 0x014B),
        (0x014D, 0x014D),
        (0x0152, 0x0153),
        (0x0166, 0x0167),
        (0x016B, 0x016B),
        (0x01CE, 0x01CE),
        (0x01D0, 0x01D0),
        (0x01D2, 0x01D2),
        (0x01D4, 0x01D4),
        (0x01D6, 0x01D6),
        (0x01D8, 0x01D8),
        (0x01DA, 0x01DA),
        (0x01DC, 0x01DC),
        (0x0251, 0x0251),
        (0x0261, 0x0261),
        (0x02C4, 0x02C4),
        (0x02C7, 0x02C7),
        (0x02C9, 0x02CB),
        (0x02CD, 0x02CD),
        (0x02D0, 0x02D0),
        (0x02D8, 0x02DB),
        (0x02DD, 0x02DD),
        (0x02DF, 0x02DF),
        (0x0300, 0x034E),
        (0x0360, 0x0362),
        (0x0391, 0x03A1),
        (0x03A3, 0x03A9),
        (0x03B1, 0x03C1),
        (0x03C3, 0x03C9),
        (0x0401, 0x0401),
        (0x0410, 0x044F),
        (0x0451, 0x0451),
        (0x2010, 0x2010),
        (0x2013, 0x2016),
        (0x2018, 0x2019),
        (0x201C, 0x201D),
        (0x2020, 0x2022),
        (0x2024, 0x2027),
        (0x2030, 0x2030),
        (0x2032, 0x2033),
        (0x2035, 0x2035),
        (0x203B, 0x203B),
        (0x203E, 0x203E),
        (0x2074, 0x2074),
        (0x207F, 0x207F),
        (0x2081, 0x2084),
        (0x20AC, 0x20AC),
        (0x2103, 0x2103),
        (0x2105, 0x2105),
        (0x2109, 0x2109),
        (0x2113, 0x2113),
        (0x2116, 0x2116),
        (0x2121, 0x2122),
        (0x2126, 0x2126),
        (0x212B, 0x212B),
        (0x2153, 0x2155),
        (0x215B, 0x215E),
        (0x2160, 0x216B),
        (0x2170, 0x2179),
        (0x2190, 0x2199),
        (0x21B8, 0x21B9),
        (0x21D2, 0x21D2),
        (0x21D4, 0x21D4),
        (0x21E7, 0x21E7),
        (0x2200, 0x2200),
        (0x2202, 0x2203),
        (0x2207, 0x2208),
        (0x220B, 0x220B),
        (0x220F, 0x220F),
        (0x2211, 0x2211),
        (0x2215, 0x2215),
        (0x221A, 0x221A),
        (0x221D, 0x2220),
        (0x2223, 0x2223),
        (0x2225, 0x2225),
        (0x2227, 0x222C),
        (0x222E, 0x222E),
        (0x2234, 0x2237),
        (0x223C, 0x223D),
        (0x2248, 0x2248),
        (0x224C, 0x224C),
        (0x2252, 0x2252),
        (0x2260, 0x2261),
        (0x2264, 0x2267),
        (0x226A, 0x226B),
        (0x226E, 0x226F),
        (0x2282, 0x2283),
        (0x2286, 0x2287),
        (0x2295, 0x2295),
        (0x2299, 0x2299),
        (0x22A5, 0x22A5),
        (0x22BF, 0x22BF),
        (0x2312, 0x2312),
        (0x2329, 0x232A),
        (0x2460, 0x24BF),
        (0x24D0, 0x24E9),
        (0x2500, 0x254B),
        (0x2550, 0x2574),
        (0x2580, 0x258F),
        (0x2592, 0x2595),
        (0x25A0, 0x25A1),
        (0x25A3, 0x25A9),
        (0x25B2, 0x25B3),
        (0x25B6, 0x25B7),
        (0x25BC, 0x25BD),
        (0x25C0, 0x25C1),
        (0x25C6, 0x25C8),
        (0x25CB, 0x25CB),
        (0x25CE, 0x25D1),
        (0x25E2, 0x25E5),
        (0x25EF, 0x25EF),
        (0x2605, 0x2606),
        (0x2609, 0x2609),
        (0x260E, 0x260F),
        (0x261C, 0x261C),
        (0x261E, 0x261E),
        (0x2640, 0x2640),
        (0x2642, 0x2642),
        (0x2660, 0x2661),
        (0x2663, 0x2665),
        (0x2667, 0x266A),
        (0x266C, 0x266D),
        (0x266F, 0x266F),
        (0x273D, 0x273D),
        (0x3008, 0x300B),
        (0x3014, 0x3015),
        (0x3018, 0x301B),
        (0xFFFD, 0xFFFD),
    ];

    /* binary search in table of non-spacing characters */
    if bisearch(ucs, ambiguous) {
        return Some(2);
    }

    return wcwidth(ucs);
}

fn wcswidth_cjk(mut pwcs: WChar, mut n: WChar) -> Option<usize> {
    let mut width = 0;

    while (pwcs > 0) && n > 0 {
        if let Some(w) = wcwidth_cjk(pwcs) {
            width += w;
        } else {
            return None;
        }

        pwcs += 1;
        n -= 1;
    }

    return Some(width);
}
