use memchr::memchr;
use smallvec::SmallVec;

use crate::{
    Boundary, Error, Part, Result,
    boundary_scan::{
        BoundaryLine, classify_boundary_line, find_boundary_start, find_next_boundary,
    },
    content_disposition::{DispositionKind, parse_content_disposition},
    header::Header,
};

#[derive(Debug)]
pub struct Multipart<'a> {
    body: &'a [u8],
    parts: SmallVec<[Part<'a>; 4]>,
}

impl<'a> Multipart<'a> {
    pub fn body(&self) -> &'a [u8] {
        self.body
    }
    pub fn parts(&self) -> &[Part<'a>] {
        &self.parts
    }
}

/// Optional resource limits for parsing untrusted multipart bodies.
///
/// Each `None` value is unlimited. `ParseLimits::default()` preserves the
/// historical unlimited behavior used by `parse` and `MultipartParser::new`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParseLimits {
    /// Maximum number of parts to return before parsing stops with an error.
    pub max_parts: Option<usize>,
    /// Maximum number of headers accepted in each part.
    pub max_headers_per_part: Option<usize>,
    /// Maximum aggregate header bytes accepted in each part, excluding CRLF.
    pub max_header_bytes_per_part: Option<usize>,
}

impl ParseLimits {
    /// Returns a limit set with every guardrail disabled.
    pub const fn unlimited() -> Self {
        Self {
            max_parts: None,
            max_headers_per_part: None,
            max_header_bytes_per_part: None,
        }
    }
}

pub struct MultipartParser<'a> {
    body: &'a [u8],
    boundary: Boundary<'a>,
    limits: ParseLimits,
    parts_seen: usize,
    cursor: usize,
    done: bool,
}

impl<'a> MultipartParser<'a> {
    pub fn new(body: &'a [u8], boundary: &'a [u8]) -> Result<Self> {
        Self::new_with_limits(body, boundary, ParseLimits::default())
    }

    /// Creates an iterator parser with explicit resource limits.
    pub fn new_with_limits(
        body: &'a [u8],
        boundary: &'a [u8],
        limits: ParseLimits,
    ) -> Result<Self> {
        let boundary = Boundary::new(boundary)?;
        let mut parser = Self {
            body,
            boundary,
            limits,
            parts_seen: 0,
            cursor: 0,
            done: false,
        };
        parser.consume_initial_boundary()?;
        Ok(parser)
    }

    fn consume_initial_boundary(&mut self) -> Result<()> {
        let boundary_start = if self.body.starts_with(self.boundary.opening()) {
            0
        } else {
            let Some(prefix_offset) = find_boundary_start(self.body, self.boundary.as_bytes(), 0)
            else {
                return Err(Error::InvalidStartingBoundary);
            };

            prefix_offset + 2
        };

        self.cursor = boundary_start + self.boundary.opening().len();
        let Some(line) = classify_boundary_line(self.body, self.cursor) else {
            return Err(Error::InvalidBoundaryTerminator {
                offset: self.cursor,
            });
        };

        match line {
            BoundaryLine::Encapsulation { next_cursor } => {
                self.cursor = next_cursor;
            }
            BoundaryLine::Close => {
                self.cursor = self.body.len();
                self.done = true;
            }
        }

        Ok(())
    }

    fn parse_next_part(&mut self) -> Result<Part<'a>> {
        if let Some(limit) = self.limits.max_parts
            && self.parts_seen >= limit
        {
            return Err(Error::PartLimitExceeded { limit });
        }

        let part_offset = self.cursor;
        let mut headers = SmallVec::<[Header<'a>; 4]>::new();
        let mut header_bytes = 0usize;
        let mut saw_content_disposition = false;
        let mut require_name = false;
        let mut name = None;
        let mut file_name = None;
        let mut content_type = None;

        loop {
            let line_start = self.cursor;
            let Some(line_end_relative) = memchr(b'\n', &self.body[self.cursor..]) else {
                return Err(Error::UnexpectedEnd {
                    offset: self.cursor,
                });
            };

            let line_end = self.cursor + line_end_relative;
            if line_end == self.cursor || self.body[line_end - 1] != b'\r' {
                return Err(Error::InvalidHeaderLineEnding { offset: line_end });
            }

            let line = &self.body[self.cursor..line_end - 1];
            self.cursor = line_end + 1;

            if line.is_empty() {
                break;
            }

            header_bytes += line.len();
            if let Some(limit) = self.limits.max_header_bytes_per_part
                && header_bytes > limit
            {
                return Err(Error::HeaderBytesLimitExceeded {
                    limit,
                    offset: line_start,
                });
            }

            if let Some(limit) = self.limits.max_headers_per_part
                && headers.len() >= limit
            {
                return Err(Error::HeaderCountLimitExceeded {
                    limit,
                    offset: line_start,
                });
            }

            if matches!(line[0], b' ' | b'\t') {
                return Err(Error::InvalidHeaderContinuation { offset: line_start });
            }

            let header = Header::parse(line, line_start)?;
            if header.name_eq_ignore_ascii_case(b"content-disposition") {
                saw_content_disposition = true;
                let disposition = parse_content_disposition(header.value(), line_start)?;
                if disposition.kind == DispositionKind::FormData {
                    require_name = true;
                }
                if name.is_none() {
                    name = disposition.name;
                }
                if file_name.is_none() {
                    file_name = disposition.file_name;
                }
            } else if content_type.is_none() && header.name_eq_ignore_ascii_case(b"content-type") {
                content_type = Some(header.value());
            }

            headers.push(header);
        }

        if !saw_content_disposition {
            return Err(Error::MissingContentDisposition {
                offset: part_offset,
            });
        }

        if require_name && name.is_none() {
            return Err(Error::MissingPartName {
                offset: part_offset,
            });
        }

        let body_start = self.cursor;
        let (body_end, next_cursor, is_final) =
            find_next_boundary(self.body, self.boundary.as_bytes(), body_start)?;
        self.cursor = next_cursor;
        if is_final {
            self.done = true;
        }

        self.parts_seen += 1;

        Ok(Part::new(
            headers,
            body_start,
            body_end,
            name,
            file_name,
            content_type,
        ))
    }
}

impl<'a> Iterator for MultipartParser<'a> {
    type Item = Result<Part<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let result = self.parse_next_part();
        if result.is_err() {
            self.done = true;
        }
        Some(result)
    }
}

pub fn parse<'a>(body: &'a [u8], boundary: &'a [u8]) -> Result<Multipart<'a>> {
    parse_with_limits(body, boundary, ParseLimits::default())
}

/// Parses a complete multipart body with explicit resource limits.
pub fn parse_with_limits<'a>(
    body: &'a [u8],
    boundary: &'a [u8],
    limits: ParseLimits,
) -> Result<Multipart<'a>> {
    let parser = MultipartParser::new_with_limits(body, boundary, limits)?;
    let mut parts = SmallVec::<[Part<'a>; 4]>::new();
    for part in parser {
        parts.push(part?);
    }
    Ok(Multipart { body, parts })
}
