/*
 * meli
 *
 * Copyright 2017-2019 Manos Pitsidianakis
 *
 * This file is part of meli.
 *
 * meli is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * meli is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with meli. If not, see <http://www.gnu.org/licenses/>.
 */
use crate::email::attachments::{Attachment, AttachmentBuilder};
use crate::email::parser::BytesExt;

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Charset {
    Ascii,
    UTF8,
    UTF16,
    ISO8859_1,
    ISO8859_2,
    ISO8859_7,
    ISO8859_15,
    Windows1250,
    Windows1251,
    Windows1252,
    Windows1253,
    GBK,
    GB2312,
    BIG5,
    ISO2022JP,
}

impl Default for Charset {
    fn default() -> Self {
        Charset::UTF8
    }
}

impl<'a> From<&'a [u8]> for Charset {
    fn from(b: &'a [u8]) -> Self {
        match b.trim() {
            b if b.eq_ignore_ascii_case(b"us-ascii") || b.eq_ignore_ascii_case(b"ascii") => {
                Charset::Ascii
            }
            b if b.eq_ignore_ascii_case(b"utf-8") || b.eq_ignore_ascii_case(b"utf8") => {
                Charset::UTF8
            }
            b if b.eq_ignore_ascii_case(b"utf-16") || b.eq_ignore_ascii_case(b"utf16") => {
                Charset::UTF16
            }
            b if b.eq_ignore_ascii_case(b"iso-8859-1") || b.eq_ignore_ascii_case(b"iso8859-1") => {
                Charset::ISO8859_1
            }
            b if b.eq_ignore_ascii_case(b"iso-8859-2") || b.eq_ignore_ascii_case(b"iso8859-2") => {
                Charset::ISO8859_2
            }
            b if b.eq_ignore_ascii_case(b"iso-8859-7") || b.eq_ignore_ascii_case(b"iso8859-7") => {
                Charset::ISO8859_7
            }
            b if b.eq_ignore_ascii_case(b"iso-8859-15")
                || b.eq_ignore_ascii_case(b"iso8859-15") =>
            {
                Charset::ISO8859_15
            }
            b if b.eq_ignore_ascii_case(b"windows-1250")
                || b.eq_ignore_ascii_case(b"windows1250") =>
            {
                Charset::Windows1250
            }
            b if b.eq_ignore_ascii_case(b"windows-1251")
                || b.eq_ignore_ascii_case(b"windows1251") =>
            {
                Charset::Windows1251
            }
            b if b.eq_ignore_ascii_case(b"windows-1252")
                || b.eq_ignore_ascii_case(b"windows1252") =>
            {
                Charset::Windows1252
            }
            b if b.eq_ignore_ascii_case(b"windows-1253")
                || b.eq_ignore_ascii_case(b"windows1253") =>
            {
                Charset::Windows1253
            }
            b if b.eq_ignore_ascii_case(b"gbk") => Charset::GBK,
            b if b.eq_ignore_ascii_case(b"gb2312") || b.eq_ignore_ascii_case(b"gb-2312") => {
                Charset::GB2312
            }
            b if b.eq_ignore_ascii_case(b"big5") => Charset::BIG5,
            b if b.eq_ignore_ascii_case(b"iso-2022-jp") => Charset::ISO2022JP,
            _ => {
                debug!("unknown tag is {:?}", str::from_utf8(b));
                Charset::Ascii
            }
        }
    }
}

impl Display for Charset {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Charset::Ascii => write!(f, "us-ascii"),
            Charset::UTF8 => write!(f, "utf-8"),
            Charset::UTF16 => write!(f, "utf-16"),
            Charset::ISO8859_1 => write!(f, "iso-8859-1"),
            Charset::ISO8859_2 => write!(f, "iso-8859-2"),
            Charset::ISO8859_7 => write!(f, "iso-8859-7"),
            Charset::ISO8859_15 => write!(f, "iso-8859-15"),
            Charset::Windows1250 => write!(f, "windows-1250"),
            Charset::Windows1251 => write!(f, "windows-1251"),
            Charset::Windows1252 => write!(f, "windows-1252"),
            Charset::Windows1253 => write!(f, "windows-1253"),
            Charset::GBK => write!(f, "GBK"),
            Charset::GB2312 => write!(f, "gb2312"),
            Charset::BIG5 => write!(f, "BIG5"),
            Charset::ISO2022JP => write!(f, "ISO-2022-JP"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MultipartType {
    Alternative,
    Digest,
    Mixed,
    Related,
    Signed,
}

impl Default for MultipartType {
    fn default() -> Self {
        MultipartType::Mixed
    }
}

impl Display for MultipartType {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            MultipartType::Alternative => write!(f, "multipart/alternative"),
            MultipartType::Digest => write!(f, "multipart/digest"),
            MultipartType::Mixed => write!(f, "multipart/mixed"),
            MultipartType::Related => write!(f, "multipart/related"),
            MultipartType::Signed => write!(f, "multipart/signed"),
        }
    }
}

impl From<&[u8]> for MultipartType {
    fn from(val: &[u8]) -> MultipartType {
        if val.eq_ignore_ascii_case(b"mixed") {
            MultipartType::Mixed
        } else if val.eq_ignore_ascii_case(b"alternative") {
            MultipartType::Alternative
        } else if val.eq_ignore_ascii_case(b"digest") {
            MultipartType::Digest
        } else if val.eq_ignore_ascii_case(b"signed") {
            MultipartType::Signed
        } else if val.eq_ignore_ascii_case(b"related") {
            MultipartType::Related
        } else {
            Default::default()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentType {
    Text {
        kind: Text,
        parameters: Vec<(Vec<u8>, Vec<u8>)>,
        charset: Charset,
    },
    Multipart {
        boundary: Vec<u8>,
        kind: MultipartType,
        parts: Vec<Attachment>,
    },
    MessageRfc822,
    PGPSignature,
    Other {
        tag: Vec<u8>,
        name: Option<String>,
    },
    OctetStream {
        name: Option<String>,
    },
}

impl Default for ContentType {
    fn default() -> Self {
        ContentType::Text {
            kind: Text::Plain,
            parameters: Vec::new(),
            charset: Charset::UTF8,
        }
    }
}

impl Display for ContentType {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ContentType::Text { kind: t, .. } => t.fmt(f),
            ContentType::Multipart { kind: k, .. } => k.fmt(f),
            ContentType::Other { ref tag, .. } => write!(f, "{}", String::from_utf8_lossy(tag)),
            ContentType::PGPSignature => write!(f, "application/pgp-signature"),
            ContentType::MessageRfc822 => write!(f, "message/rfc822"),
            ContentType::OctetStream { .. } => write!(f, "application/octet-stream"),
        }
    }
}

impl ContentType {
    pub fn is_text(&self) -> bool {
        if let ContentType::Text { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn is_text_html(&self) -> bool {
        if let ContentType::Text {
            kind: Text::Html, ..
        } = self
        {
            true
        } else {
            false
        }
    }

    pub fn make_boundary(parts: &[AttachmentBuilder]) -> String {
        use crate::email::compose::random::gen_boundary;
        let mut boundary = "bzz_bzz__bzz__".to_string();
        let mut random_boundary = gen_boundary();

        let mut loop_counter = 4096;
        'loo: loop {
            let mut flag = true;
            for sub in parts {
                'sub_loop: loop {
                    if sub.raw().find(random_boundary.as_bytes()).is_some() {
                        random_boundary = gen_boundary();
                        flag = false;
                    } else {
                        break 'sub_loop;
                    }
                }
            }
            if flag {
                break 'loo;
            }
            loop_counter -= 1;
            if loop_counter == 0 {
                panic!("Can't generate randomness. This is a BUG");
            }
        }

        boundary.push_str(&random_boundary);
        /* rfc134
         * "The only mandatory parameter for the multipart Content-Type is the boundary parameter,
         * which consists of 1 to 70 characters from a set of characters known to be very robust
         * through email gateways, and NOT ending with white space"*/
        boundary.truncate(70);
        boundary
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            ContentType::Other { ref name, .. } => name.as_ref().map(|n| n.as_ref()),
            ContentType::OctetStream { ref name } => name.as_ref().map(|n| n.as_ref()),
            _ => None,
        }
    }

    pub fn parts(&self) -> Option<&[Attachment]> {
        if let ContentType::Multipart { ref parts, .. } = self {
            Some(parts)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Text {
    Plain,
    Html,
    Rfc822,
    Other { tag: Vec<u8> },
}

impl Text {
    pub fn is_html(&self) -> bool {
        if let Text::Html = self {
            true
        } else {
            false
        }
    }
}

impl Display for Text {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match *self {
            Text::Plain => write!(f, "text/plain"),
            Text::Html => write!(f, "text/html"),
            Text::Rfc822 => write!(f, "text/rfc822"),
            Text::Other { tag: ref t } => write!(f, "text/{}", String::from_utf8_lossy(t)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentTransferEncoding {
    _8Bit,
    _7Bit,
    Base64,
    QuotedPrintable,
    Other { tag: Vec<u8> },
}

impl Default for ContentTransferEncoding {
    fn default() -> Self {
        ContentTransferEncoding::_8Bit
    }
}

impl Display for ContentTransferEncoding {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match *self {
            ContentTransferEncoding::_7Bit => write!(f, "7bit"),
            ContentTransferEncoding::_8Bit => write!(f, "8bit"),
            ContentTransferEncoding::Base64 => write!(f, "base64"),
            ContentTransferEncoding::QuotedPrintable => write!(f, "quoted-printable"),
            ContentTransferEncoding::Other { tag: ref t } => {
                panic!("unknown encoding {:?}", str::from_utf8(t))
            }
        }
    }
}
impl From<&[u8]> for ContentTransferEncoding {
    fn from(val: &[u8]) -> ContentTransferEncoding {
        if val.eq_ignore_ascii_case(b"base64") {
            ContentTransferEncoding::Base64
        } else if val.eq_ignore_ascii_case(b"7bit") {
            ContentTransferEncoding::_7Bit
        } else if val.eq_ignore_ascii_case(b"8bit") {
            ContentTransferEncoding::_8Bit
        } else if val.eq_ignore_ascii_case(b"quoted-printable") {
            ContentTransferEncoding::QuotedPrintable
        } else {
            ContentTransferEncoding::Other {
                tag: val.to_ascii_lowercase(),
            }
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentDisposition {
    pub kind: ContentDispositionKind,
    pub filename: Option<String>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
    pub read_date: Option<String>,
    pub size: Option<String>,
    pub parameter: Vec<String>,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentDispositionKind {
    Inline,
    Attachment,
}

impl ContentDispositionKind {
    pub fn is_inline(&self) -> bool {
        *self == ContentDispositionKind::Inline
    }

    pub fn is_attachment(&self) -> bool {
        *self == ContentDispositionKind::Attachment
    }
}

impl Default for ContentDispositionKind {
    fn default() -> Self {
        ContentDispositionKind::Inline
    }
}

impl Display for ContentDispositionKind {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match *self {
            ContentDispositionKind::Inline => write!(f, "inline"),
            ContentDispositionKind::Attachment => write!(f, "attachment"),
        }
    }
}
impl From<&[u8]> for ContentDisposition {
    fn from(val: &[u8]) -> ContentDisposition {
        crate::email::parser::attachments::content_disposition(val)
            .map(|(_, v)| v)
            .unwrap_or_default()
    }
}
