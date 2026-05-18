use fastmulp_core::{boundary_from_content_type, parse};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn accepts_transport_padding_around_boundary_lines() -> TestResult {
  let body = concat!(
    "--abc123 \t\r\n",
    "Content-Disposition: form-data; name=\"field\"\r\n",
    "\r\n",
    "payload\r\n",
    "--abc123-- \t\r\n",
  );

  let multipart = parse(body.as_bytes(), b"abc123")?;
  assert_eq!(multipart.parts().len(), 1);
  assert_eq!(multipart.parts()[0].body(multipart.body()), b"payload");
  Ok(())
}

#[test]
fn accepts_empty_multipart() -> TestResult {
  let multipart = parse(b"--abc123--\r\n", b"abc123")?;
  assert!(multipart.parts().is_empty());
  Ok(())
}

#[test]
fn accepts_final_boundary_without_terminal_crlf() -> TestResult {
  let body = concat!(
    "--abc123\r\n",
    "Content-Disposition: form-data; name=\"field\"\r\n",
    "\r\n",
    "payload\r\n",
    "--abc123--",
  );

  let multipart = parse(body.as_bytes(), b"abc123")?;
  assert_eq!(multipart.parts()[0].body(multipart.body()), b"payload");
  Ok(())
}

#[test]
fn try_body_returns_body_for_original_source() -> TestResult {
  let body = concat!(
    "--abc123\r\n",
    "Content-Disposition: form-data; name=\"field\"\r\n",
    "\r\n",
    "payload\r\n",
    "--abc123--\r\n",
  );

  let multipart = parse(body.as_bytes(), b"abc123")?;
  assert_eq!(multipart.parts()[0].try_body(multipart.body()), Some(&b"payload"[..]));
  Ok(())
}

#[test]
fn try_body_returns_none_for_too_short_source() -> TestResult {
  let body = concat!(
    "--abc123\r\n",
    "Content-Disposition: form-data; name=\"field\"\r\n",
    "\r\n",
    "payload\r\n",
    "--abc123--\r\n",
  );

  let multipart = parse(body.as_bytes(), b"abc123")?;
  let part = &multipart.parts()[0];
  let body_range = part.body_range();
  let too_short = &multipart.body()[..body_range.end - 1];
  assert_eq!(part.try_body(too_short), None);
  Ok(())
}

#[test]
fn accepts_preamble_and_epilogue() -> TestResult {
  let body = concat!(
    "ignored preamble\r\n",
    "--abc123\r\n",
    "Content-Disposition: form-data; name=\"field\"\r\n",
    "\r\n",
    "payload\r\n",
    "--abc123--\r\n",
    "ignored epilogue",
  );

  let multipart = parse(body.as_bytes(), b"abc123")?;
  assert_eq!(multipart.parts().len(), 1);
  assert_eq!(multipart.parts()[0].body(multipart.body()), b"payload");
  Ok(())
}

#[test]
fn preserves_duplicate_field_names_in_order() -> TestResult {
  let body = concat!(
    "--abc123\r\n",
    "Content-Disposition: form-data; name=\"field\"\r\n",
    "\r\n",
    "first\r\n",
    "--abc123\r\n",
    "Content-Disposition: form-data; name=\"field\"\r\n",
    "\r\n",
    "second\r\n",
    "--abc123--\r\n",
  );

  let multipart = parse(body.as_bytes(), b"abc123")?;
  assert_eq!(multipart.parts().len(), 2);
  assert_eq!(
    multipart.parts()[0].name().and_then(|value| value.as_str().ok()),
    Some("field")
  );
  assert_eq!(multipart.parts()[0].body(multipart.body()), b"first");
  assert_eq!(multipart.parts()[1].body(multipart.body()), b"second");
  Ok(())
}

#[test]
fn parses_empty_field_body() -> TestResult {
  let body = concat!(
    "--abc123\r\n",
    "Content-Disposition: form-data; name=\"field\"\r\n",
    "\r\n",
    "\r\n",
    "--abc123--\r\n",
  );

  let multipart = parse(body.as_bytes(), b"abc123")?;
  assert_eq!(multipart.parts()[0].body(multipart.body()), b"");
  Ok(())
}

#[test]
fn parses_binary_part_body_exactly() -> TestResult {
  let mut body = Vec::new();
  body.extend_from_slice(b"--abc123\r\n");
  body.extend_from_slice(b"Content-Disposition: form-data; name=\"file\"; filename=\"blob.bin\"\r\n");
  body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
  body.extend_from_slice(&[0x00, 0x01, 0xff, 0x7f, b'\r', b'\n', b'X']);
  body.extend_from_slice(b"\r\n--abc123--\r\n");

  let multipart = parse(body.as_slice(), b"abc123")?;
  assert_eq!(multipart.parts()[0].body(multipart.body()), [0x00, 0x01, 0xff, 0x7f, b'\r', b'\n', b'X']);
  Ok(())
}

#[test]
fn accepts_unquoted_name_parameter() -> TestResult {
  let body = concat!(
    "--abc123\r\n",
    "Content-Disposition: form-data; name=field\r\n",
    "\r\n",
    "payload\r\n",
    "--abc123--\r\n",
  );

  let multipart = parse(body.as_bytes(), b"abc123")?;
  assert_eq!(
    multipart.parts()[0].name().and_then(|value| value.as_str().ok()),
    Some("field")
  );
  Ok(())
}

#[test]
fn decodes_html_escaped_name_and_filename() -> TestResult {
  let body = concat!(
    "--abc123\r\n",
    "Content-Disposition: form-data; name=\"field%22%0A\"; filename=\"a%22b%0D%0A.txt\"\r\n",
    "\r\n",
    "payload\r\n",
    "--abc123--\r\n",
  );

  let multipart = parse(body.as_bytes(), b"abc123")?;
  let part = &multipart.parts()[0];
  assert_eq!(part.name().and_then(|value| value.as_str().ok()), Some("field\"\n"));
  assert_eq!(part.file_name().and_then(|value| value.as_str().ok()), Some("a\"b\r\n.txt"));
  Ok(())
}

#[test]
fn accepts_case_insensitive_disposition_and_header_names() -> TestResult {
  let body = concat!(
    "--abc123\r\n",
    "content-disposition: FORM-DATA; name=\"field\"\r\n",
    "content-type: text/plain\r\n",
    "\r\n",
    "payload\r\n",
    "--abc123--\r\n",
  );

  let multipart = parse(body.as_bytes(), b"abc123")?;
  let part = &multipart.parts()[0];
  assert_eq!(part.content_type(), Some(&b"text/plain"[..]));
  assert_eq!(part.body(multipart.body()), b"payload");
  Ok(())
}

#[test]
fn supports_recursive_parsing_of_nested_multipart_mixed() -> TestResult {
  let body = concat!(
    "--outer\r\n",
    "Content-Disposition: form-data; name=\"files\"\r\n",
    "Content-Type: multipart/mixed; boundary=inner\r\n",
    "\r\n",
    "--inner\r\n",
    "Content-Disposition: file; name=\"files\"; filename=\"a.txt\"\r\n",
    "\r\n",
    "A\r\n",
    "--inner\r\n",
    "Content-Disposition: file; name=\"files\"; filename=\"b.txt\"\r\n",
    "\r\n",
    "B\r\n",
    "--inner--\r\n",
    "--outer--\r\n",
  );

  let outer = parse(body.as_bytes(), b"outer")?;
  let outer_part = &outer.parts()[0];
  let content_type = core::str::from_utf8(
    outer_part
      .content_type()
      .ok_or_else(|| std::io::Error::other("content type must exist"))?,
  )?;
  let inner_boundary =
    boundary_from_content_type(content_type).ok_or_else(|| std::io::Error::other("nested boundary should exist"))?;
  let inner = parse(outer_part.body(outer.body()), inner_boundary.as_bytes())?;

  assert_eq!(inner.parts().len(), 2);
  assert_eq!(inner.parts()[0].body(inner.body()), b"A");
  assert_eq!(inner.parts()[1].body(inner.body()), b"B");
  Ok(())
}
