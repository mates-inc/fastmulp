use std::borrow::Cow;

use smallvec::SmallVec;

use crate::{
    Error, Result,
    util::{eq_ignore_ascii_case, skip_ascii_whitespace, trim_ascii},
};

#[derive(Debug, PartialEq, Eq)]
pub struct Boundary<'a> {
    raw: &'a [u8],
    opening: SmallVec<[u8; 72]>,
}

/// Extracts the `boundary` parameter from a multipart `Content-Type` header value.
///
/// The returned value is borrowed for ordinary parameters and owned when a
/// quoted-string contains MIME quoted-pair escapes that need to be decoded.
pub fn boundary_from_content_type(content_type: &str) -> Option<Cow<'_, str>> {
    let bytes = content_type.as_bytes();
    let mut cursor = skip_ascii_whitespace(bytes, 0);
    let media_start = cursor;

    while cursor < bytes.len() && bytes[cursor] != b';' {
        cursor += 1;
    }

    if !is_multipart_media_type(trim_ascii(&bytes[media_start..cursor])) {
        return None;
    }

    while cursor < bytes.len() {
        if bytes[cursor] != b';' {
            return None;
        }

        cursor += 1;
        cursor = skip_ascii_whitespace(bytes, cursor);
        if cursor == bytes.len() {
            return None;
        }

        let name_start = cursor;
        while cursor < bytes.len() && bytes[cursor] != b'=' && bytes[cursor] != b';' {
            cursor += 1;
        }

        let name = trim_ascii(&bytes[name_start..cursor]);
        if cursor == bytes.len() || bytes[cursor] != b'=' {
            if cursor < bytes.len() && bytes[cursor] == b';' {
                continue;
            }

            return None;
        }

        cursor += 1;
        cursor = skip_ascii_whitespace(bytes, cursor);

        let value = parse_content_type_value(content_type, cursor)?;
        cursor = skip_ascii_whitespace(bytes, value.next);
        if cursor < bytes.len() && bytes[cursor] != b';' {
            return None;
        }

        if eq_ignore_ascii_case(name, b"boundary") {
            return Some(value.raw);
        }
    }

    None
}

impl<'a> Boundary<'a> {
    pub fn new(raw: &'a [u8]) -> Result<Self> {
        if raw.is_empty() {
            return Err(Error::EmptyBoundary);
        }

        if raw.len() > 70 {
            return Err(Error::BoundaryTooLong { len: raw.len() });
        }

        for (index, byte) in raw.iter().copied().enumerate() {
            if !is_boundary_byte(byte, index + 1 == raw.len()) {
                return Err(Error::InvalidBoundaryByte {
                    offset: index,
                    byte,
                });
            }
        }

        let mut opening = SmallVec::<[u8; 72]>::with_capacity(raw.len() + 2);
        opening.extend_from_slice(b"--");
        opening.extend_from_slice(raw);

        Ok(Self { raw, opening })
    }

    pub fn as_bytes(&self) -> &'a [u8] {
        self.raw
    }

    pub(crate) fn opening(&self) -> &[u8] {
        self.opening.as_slice()
    }
}

struct ContentTypeValue<'a> {
    raw: Cow<'a, str>,
    next: usize,
}

fn parse_content_type_value(content_type: &str, start: usize) -> Option<ContentTypeValue<'_>> {
    let bytes = content_type.as_bytes();
    if start >= bytes.len() {
        return None;
    }

    if bytes[start] == b'"' {
        let inner_start = start + 1;
        let mut cursor = inner_start;
        let mut decoded = None::<Vec<u8>>;
        let mut copied_from = inner_start;
        while cursor < bytes.len() {
            match bytes[cursor] {
                b'"' => {
                    if let Some(mut decoded) = decoded {
                        decoded.extend_from_slice(&bytes[copied_from..cursor]);
                        let raw = String::from_utf8(decoded).ok()?;
                        return Some(ContentTypeValue {
                            raw: Cow::Owned(raw),
                            next: cursor + 1,
                        });
                    }

                    return content_type
                        .get(inner_start..cursor)
                        .map(|raw| ContentTypeValue {
                            raw: Cow::Borrowed(raw),
                            next: cursor + 1,
                        });
                }
                b'\\' => {
                    if cursor + 1 == bytes.len() {
                        return None;
                    }

                    let decoded = decoded.get_or_insert_with(|| Vec::with_capacity(bytes.len()));
                    decoded.extend_from_slice(&bytes[copied_from..cursor]);
                    decoded.push(bytes[cursor + 1]);
                    cursor += 2;
                    copied_from = cursor;
                }
                _ => {
                    cursor += 1;
                }
            }
        }

        return None;
    }

    let mut cursor = start;
    while cursor < bytes.len() && bytes[cursor] != b';' {
        cursor += 1;
    }

    let end = trim_ascii_end(bytes, cursor);
    content_type.get(start..end).map(|raw| ContentTypeValue {
        raw: Cow::Borrowed(raw),
        next: cursor,
    })
}

fn trim_ascii_end(bytes: &[u8], mut end: usize) -> usize {
    while end > 0 && matches!(bytes[end - 1], b' ' | b'\t') {
        end -= 1;
    }

    end
}

fn is_boundary_byte(byte: u8, is_last: bool) -> bool {
    match byte {
        b'0'..=b'9'
        | b'a'..=b'z'
        | b'A'..=b'Z'
        | b'\''
        | b'('
        | b')'
        | b'+'
        | b'_'
        | b','
        | b'-'
        | b'.'
        | b'/'
        | b':'
        | b'='
        | b'?' => true,
        b' ' => !is_last,
        _ => false,
    }
}

fn is_multipart_media_type(value: &[u8]) -> bool {
    const PREFIX: &[u8] = b"multipart/";
    value.len() > PREFIX.len() && eq_ignore_ascii_case(&value[..PREFIX.len()], PREFIX)
}
