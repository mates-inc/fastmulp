mod decode;

use memchr::memchr;

use crate::{
    Error, Result, TextValue,
    util::{eq_ignore_ascii_case, skip_ascii_whitespace, trim_ascii},
};

use self::decode::{decode_extended_value, decode_html_name_escapes};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispositionKind {
    FormData,
    File,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ContentDisposition<'a> {
    pub kind: DispositionKind,
    pub name: Option<TextValue<'a>>,
    pub file_name: Option<TextValue<'a>>,
}

pub fn parse_content_disposition<'a>(
    value: &'a [u8],
    offset: usize,
) -> Result<ContentDisposition<'a>> {
    let disposition_end = memchr(b';', value).unwrap_or(value.len());
    let disposition = trim_ascii(&value[..disposition_end]);
    let kind = parse_disposition_kind(disposition, offset)?;

    let mut cursor = disposition_end;
    let mut disposition_data = ContentDisposition {
        kind,
        name: None,
        file_name: None,
    };
    let mut file_name_star = None;

    while cursor < value.len() {
        if value[cursor] != b';' {
            return Err(Error::InvalidContentDisposition {
                offset: offset + cursor,
            });
        }

        cursor += 1;
        cursor = skip_ascii_whitespace(value, cursor);
        if cursor == value.len() {
            break;
        }

        let name_start = cursor;
        while cursor < value.len() && value[cursor] != b'=' && value[cursor] != b';' {
            cursor += 1;
        }

        let parameter_name = trim_ascii(&value[name_start..cursor]);
        if parameter_name.is_empty() || cursor == value.len() || value[cursor] != b'=' {
            return Err(Error::InvalidContentDisposition {
                offset: offset + name_start,
            });
        }

        cursor += 1;
        cursor = skip_ascii_whitespace(value, cursor);

        let (parameter_value, next) = parse_parameter_value(value, cursor, offset)?;
        if eq_ignore_ascii_case(parameter_name, b"name") {
            if disposition_data.name.is_none() {
                disposition_data.name = Some(decode_html_name_escapes(parameter_value));
            }
        } else if eq_ignore_ascii_case(parameter_name, b"filename") {
            if disposition_data.file_name.is_none() {
                disposition_data.file_name = Some(decode_html_name_escapes(parameter_value));
            }
        } else if eq_ignore_ascii_case(parameter_name, b"filename*") && file_name_star.is_none() {
            file_name_star = Some(decode_extended_value(parameter_value, offset + cursor)?);
        }

        cursor = next;
        cursor = skip_ascii_whitespace(value, cursor);
    }

    if let Some(file_name_star) = file_name_star {
        disposition_data.file_name = Some(file_name_star);
    }

    Ok(disposition_data)
}

fn parse_disposition_kind(value: &[u8], offset: usize) -> Result<DispositionKind> {
    if eq_ignore_ascii_case(value, b"form-data") {
        return Ok(DispositionKind::FormData);
    }

    if eq_ignore_ascii_case(value, b"file") {
        return Ok(DispositionKind::File);
    }

    Err(Error::InvalidContentDisposition { offset })
}

fn parse_parameter_value<'a>(
    value: &'a [u8],
    start: usize,
    offset: usize,
) -> Result<(TextValue<'a>, usize)> {
    if start == value.len() {
        return Err(Error::InvalidContentDisposition { offset });
    }

    if value[start] == b'"' {
        return parse_quoted_value(value, start + 1, offset);
    }

    let mut cursor = start;
    while cursor < value.len() && value[cursor] != b';' {
        cursor += 1;
    }

    let token = trim_ascii(&value[start..cursor]);
    if token.is_empty() {
        return Err(Error::InvalidContentDisposition {
            offset: offset + start,
        });
    }
    if !token.iter().copied().all(is_tchar) {
        return Err(Error::InvalidContentDisposition {
            offset: offset + start,
        });
    }

    Ok((TextValue::Borrowed(token), cursor))
}

fn is_tchar(byte: u8) -> bool {
    matches!(
        byte,
        b'!' | b'#'
            | b'$'
            | b'%'
            | b'&'
            | b'\''
            | b'*'
            | b'+'
            | b'-'
            | b'.'
            | b'^'
            | b'_'
            | b'`'
            | b'|'
            | b'~'
            | b'0'..=b'9'
            | b'A'..=b'Z'
            | b'a'..=b'z'
    )
}

fn parse_quoted_value<'a>(
    value: &'a [u8],
    start: usize,
    offset: usize,
) -> Result<(TextValue<'a>, usize)> {
    let mut cursor = start;
    let mut copied_from = start;
    let mut owned: Option<smallvec::SmallVec<[u8; 32]>> = None;

    while cursor < value.len() {
        match value[cursor] {
            b'"' => {
                let Some(mut owned) = owned else {
                    return Ok((TextValue::Borrowed(&value[start..cursor]), cursor + 1));
                };

                owned.extend_from_slice(&value[copied_from..cursor]);
                return Ok((TextValue::Owned(owned), cursor + 1));
            }
            b'\\' => {
                if cursor + 1 == value.len() {
                    return Err(Error::InvalidContentDisposition {
                        offset: offset + cursor,
                    });
                }

                let owned = owned.get_or_insert_with(smallvec::SmallVec::<[u8; 32]>::new);
                owned.extend_from_slice(&value[copied_from..cursor]);
                owned.push(value[cursor + 1]);
                cursor += 2;
                copied_from = cursor;
            }
            _ => {
                cursor += 1;
            }
        }
    }

    Err(Error::InvalidContentDisposition {
        offset: offset + start,
    })
}
