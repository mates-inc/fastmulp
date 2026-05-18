#![deny(clippy::undocumented_unsafe_blocks, unsafe_op_in_unsafe_fn)]

use fastmulp_core::{
    Part, TextValue, boundary_from_content_type as parse_boundary, parse as parse_multipart,
};
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

#[wasm_bindgen]
pub fn boundary_from_content_type(content_type: &str) -> Option<String> {
    parse_boundary(content_type).map(|boundary| boundary.into_owned())
}

#[wasm_bindgen]
pub fn parse(body: &[u8], boundary: &str) -> Result<Array, JsValue> {
    parse_parts(body, boundary.as_bytes())
}

#[wasm_bindgen(js_name = parseContentType)]
pub fn parse_content_type(body: &[u8], content_type: &str) -> Result<Array, JsValue> {
    let Some(boundary) = parse_boundary(content_type) else {
        return Err(JsValue::from_str(
            "missing boundary parameter in Content-Type",
        ));
    };

    parse_parts(body, boundary.as_bytes())
}

fn parse_parts(body: &[u8], boundary: &[u8]) -> Result<Array, JsValue> {
    let multipart =
        parse_multipart(body, boundary).map_err(|error| JsValue::from_str(&error.to_string()))?;
    let parts = Array::new();

    for part in multipart.parts() {
        parts.push(&part_to_js(part)?);
    }

    Ok(parts)
}

fn part_to_js(part: &Part<'_>) -> Result<JsValue, JsValue> {
    let object = Object::new();
    let headers = Array::new();
    for header in part.headers() {
        let header_object = Object::new();
        set_property(
            &header_object,
            "name",
            &JsValue::from_str(&decode_utf8(header.name(), "header name")?),
        )?;
        set_property(
            &header_object,
            "value",
            &JsValue::from_str(&decode_utf8(header.value(), "header value")?),
        )?;
        headers.push(&header_object);
    }

    let body_range = part.body_range();
    set_optional_text_property(&object, "name", part.name(), "name")?;
    set_optional_text_property(&object, "file_name", part.file_name(), "file_name")?;
    set_optional_bytes_property(&object, "content_type", part.content_type(), "content_type")?;
    set_property(
        &object,
        "body_start",
        &JsValue::from_f64(f64::from(to_u32(body_range.start, "body_start")?)),
    )?;
    set_property(
        &object,
        "body_end",
        &JsValue::from_f64(f64::from(to_u32(body_range.end, "body_end")?)),
    )?;
    set_property(&object, "headers", &headers)?;

    Ok(object.into())
}

fn set_optional_text_property(
    object: &Object,
    key: &str,
    value: Option<&TextValue<'_>>,
    label: &str,
) -> Result<(), JsValue> {
    match value {
        Some(value) => set_property(
            object,
            key,
            &JsValue::from_str(&decode_utf8(value.as_bytes(), label)?),
        ),
        None => set_property(object, key, &JsValue::UNDEFINED),
    }
}

fn set_optional_bytes_property(
    object: &Object,
    key: &str,
    value: Option<&[u8]>,
    label: &str,
) -> Result<(), JsValue> {
    match value {
        Some(value) => set_property(object, key, &JsValue::from_str(&decode_utf8(value, label)?)),
        None => set_property(object, key, &JsValue::UNDEFINED),
    }
}

fn set_property(object: &Object, key: &str, value: &JsValue) -> Result<(), JsValue> {
    Reflect::set(object.as_ref(), &JsValue::from_str(key), value)?;
    Ok(())
}

fn decode_utf8(value: &[u8], label: &str) -> Result<String, JsValue> {
    core::str::from_utf8(value)
        .map(str::to_owned)
        .map_err(|_| JsValue::from_str(&format!("{label} must be valid UTF-8")))
}

fn to_u32(value: usize, label: &str) -> Result<u32, JsValue> {
    u32::try_from(value).map_err(|_| JsValue::from_str(&format!("{label} exceeds u32")))
}
