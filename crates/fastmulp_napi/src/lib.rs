#![deny(clippy::undocumented_unsafe_blocks, unsafe_op_in_unsafe_fn)]

use fastmulp_core::{
    Part, TextValue, boundary_from_content_type as parse_boundary, parse as parse_multipart,
};
use napi::{Error, Result, Status, bindgen_prelude::Buffer};
use napi_derive::napi;

#[napi(object)]
pub struct JsHeader {
    pub name: String,
    pub value: String,
}

#[napi(object)]
pub struct JsPart {
    pub name: Option<String>,
    pub file_name: Option<String>,
    pub content_type: Option<String>,
    pub body_start: u32,
    pub body_end: u32,
    pub headers: Vec<JsHeader>,
}

#[napi]
pub fn boundary_from_content_type(content_type: String) -> Option<String> {
    parse_boundary(&content_type).map(|boundary| boundary.into_owned())
}

#[napi]
pub fn parse(body: Buffer, boundary: String) -> Result<Vec<JsPart>> {
    parse_parts(body.as_ref(), boundary.as_bytes())
}

#[napi]
pub fn parse_content_type(body: Buffer, content_type: String) -> Result<Vec<JsPart>> {
    let Some(boundary) = parse_boundary(&content_type) else {
        return Err(Error::new(
            Status::InvalidArg,
            "missing boundary parameter in Content-Type".to_owned(),
        ));
    };

    parse_parts(body.as_ref(), boundary.as_bytes())
}

fn parse_parts(body: &[u8], boundary: &[u8]) -> Result<Vec<JsPart>> {
    let multipart = parse_multipart(body, boundary).map_err(into_napi_error)?;
    let mut parts = Vec::with_capacity(multipart.parts().len());
    for part in multipart.parts() {
        parts.push(convert_part(part)?);
    }
    Ok(parts)
}

fn convert_part(part: &Part<'_>) -> Result<JsPart> {
    let body_range = part.body_range();
    let name = decode_optional_text(part.name(), "name")?;
    let file_name = decode_optional_text(part.file_name(), "file_name")?;
    let content_type = decode_optional_bytes(part.content_type(), "content_type")?;
    let mut headers = Vec::with_capacity(part.headers().len());
    for header in part.headers() {
        headers.push(JsHeader {
            name: decode_utf8(header.name(), "header name")?,
            value: decode_utf8(header.value(), "header value")?,
        });
    }

    Ok(JsPart {
        name,
        file_name,
        content_type,
        body_start: to_u32(body_range.start, "body_start")?,
        body_end: to_u32(body_range.end, "body_end")?,
        headers,
    })
}

fn decode_optional_text(value: Option<&TextValue<'_>>, label: &str) -> Result<Option<String>> {
    value
        .map(|value| decode_utf8(value.as_bytes(), label))
        .transpose()
}

fn decode_optional_bytes(value: Option<&[u8]>, label: &str) -> Result<Option<String>> {
    value.map(|value| decode_utf8(value, label)).transpose()
}

fn decode_utf8(value: &[u8], label: &str) -> Result<String> {
    core::str::from_utf8(value)
        .map(str::to_owned)
        .map_err(|_| Error::new(Status::InvalidArg, format!("{label} must be valid UTF-8")))
}

fn to_u32(value: usize, label: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| Error::new(Status::InvalidArg, format!("{label} exceeds u32")))
}

fn into_napi_error(error: fastmulp_core::Error) -> Error {
    Error::new(Status::InvalidArg, error.to_string())
}
