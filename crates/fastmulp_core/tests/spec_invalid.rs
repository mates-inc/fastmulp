use fastmulp_core::{Boundary, Error, ParseLimits, parse, parse_with_limits};

fn assert_invalid_content_disposition(disposition: &str) {
    let body =
        format!("--abc123\r\nContent-Disposition: {disposition}\r\n\r\npayload\r\n--abc123--\r\n");

    assert!(matches!(
        parse(body.as_bytes(), b"abc123"),
        Err(Error::InvalidContentDisposition { .. })
    ));
}

#[test]
fn rejects_missing_content_disposition() {
    let body = concat!(
        "--abc123\r\n",
        "Content-Type: text/plain\r\n",
        "\r\n",
        "payload\r\n",
        "--abc123--\r\n",
    );

    assert!(matches!(
        parse(body.as_bytes(), b"abc123"),
        Err(Error::MissingContentDisposition { .. })
    ));
}

#[test]
fn rejects_missing_name_parameter() {
    let body = concat!(
        "--abc123\r\n",
        "Content-Disposition: form-data; filename=\"blob.bin\"\r\n",
        "\r\n",
        "payload\r\n",
        "--abc123--\r\n",
    );

    assert!(matches!(
        parse(body.as_bytes(), b"abc123"),
        Err(Error::MissingPartName { .. })
    ));
}

#[test]
fn rejects_unquoted_content_disposition_parameter_values_with_whitespace() {
    for disposition in [
        "form-data; name=field value",
        "form-data; name=field\tvalue",
        "form-data; name=field value; filename=blob.txt",
    ] {
        assert_invalid_content_disposition(disposition);
    }
}

#[test]
fn rejects_unquoted_content_disposition_parameter_values_with_separators() {
    for disposition in [
        "form-data; name=field\"value",
        "form-data; name=field,value",
        "form-data; name=field:value",
        "form-data; name=field/value",
        "form-data; name=field(value)",
        "form-data; name=field[value]",
        "form-data; name=field{value}",
        "form-data; name=field;value",
    ] {
        assert_invalid_content_disposition(disposition);
    }
}

#[test]
fn rejects_header_continuation() {
    let body = concat!(
        "--abc123\r\n",
        "Content-Disposition: form-data; name=\"field\"\r\n",
        " folded: value\r\n",
        "\r\n",
        "payload\r\n",
        "--abc123--\r\n",
    );

    assert!(matches!(
        parse(body.as_bytes(), b"abc123"),
        Err(Error::InvalidHeaderContinuation { .. })
    ));
}

#[test]
fn rejects_header_without_separator() {
    let body = concat!(
        "--abc123\r\n",
        "Content-Disposition form-data; name=\"field\"\r\n",
        "\r\n",
        "payload\r\n",
        "--abc123--\r\n",
    );

    assert!(matches!(
        parse(body.as_bytes(), b"abc123"),
        Err(Error::MissingHeaderSeparator { .. })
    ));
}

#[test]
fn rejects_missing_closing_boundary() {
    let body = concat!(
        "--abc123\r\n",
        "Content-Disposition: form-data; name=\"field\"\r\n",
        "\r\n",
        "payload",
    );

    assert!(matches!(
        parse(body.as_bytes(), b"abc123"),
        Err(Error::MissingClosingBoundary { .. })
    ));
}

#[test]
fn rejects_invalid_initial_boundary_terminator() {
    let body = concat!(
        "--abc123x\r\n",
        "Content-Disposition: form-data; name=\"field\"\r\n",
        "\r\n",
        "payload\r\n",
        "--abc123--\r\n",
    );

    assert!(matches!(
        parse(body.as_bytes(), b"abc123"),
        Err(Error::InvalidBoundaryTerminator { .. })
    ));
}

#[test]
fn rejects_bytes_immediately_after_closing_boundary() {
    assert!(matches!(
        parse(b"--abc123--extra", b"abc123"),
        Err(Error::InvalidBoundaryTerminator { .. })
    ));
}

#[test]
fn rejects_invalid_boundary_character() {
    assert!(matches!(
        Boundary::new(b"abc\"123"),
        Err(Error::InvalidBoundaryByte { .. })
    ));
}

#[test]
fn rejects_invalid_filename_star_escape() {
    let body = concat!(
        "--abc123\r\n",
        "Content-Disposition: form-data; name=\"file\"; filename*=UTF-8''bad%2Gname.txt\r\n",
        "\r\n",
        "payload\r\n",
        "--abc123--\r\n",
    );

    assert!(matches!(
        parse(body.as_bytes(), b"abc123"),
        Err(Error::InvalidContentDisposition { .. })
    ));
}

#[test]
fn rejects_non_form_data_disposition() {
    let body = concat!(
        "--abc123\r\n",
        "Content-Disposition: attachment; name=\"field\"\r\n",
        "\r\n",
        "payload\r\n",
        "--abc123--\r\n",
    );

    assert!(matches!(
        parse(body.as_bytes(), b"abc123"),
        Err(Error::InvalidContentDisposition { .. })
    ));
}

#[test]
fn rejects_part_count_over_configured_limit() {
    let body = concat!(
        "--abc123\r\n",
        "Content-Disposition: form-data; name=\"first\"\r\n",
        "\r\n",
        "first\r\n",
        "--abc123\r\n",
        "Content-Disposition: form-data; name=\"second\"\r\n",
        "\r\n",
        "second\r\n",
        "--abc123--\r\n",
    );

    assert!(matches!(
        parse_with_limits(
            body.as_bytes(),
            b"abc123",
            ParseLimits {
                max_parts: Some(1),
                ..ParseLimits::default()
            },
        ),
        Err(Error::PartLimitExceeded { limit: 1 })
    ));
}

#[test]
fn rejects_header_count_over_configured_limit() {
    let body = concat!(
        "--abc123\r\n",
        "Content-Disposition: form-data; name=\"file\"\r\n",
        "Content-Type: text/plain\r\n",
        "\r\n",
        "payload\r\n",
        "--abc123--\r\n",
    );

    assert!(matches!(
        parse_with_limits(
            body.as_bytes(),
            b"abc123",
            ParseLimits {
                max_headers_per_part: Some(1),
                ..ParseLimits::default()
            },
        ),
        Err(Error::HeaderCountLimitExceeded { limit: 1, .. })
    ));
}

#[test]
fn rejects_header_bytes_over_configured_limit() {
    let body = concat!(
        "--abc123\r\n",
        "Content-Disposition: form-data; name=\"field\"\r\n",
        "\r\n",
        "payload\r\n",
        "--abc123--\r\n",
    );

    assert!(matches!(
        parse_with_limits(
            body.as_bytes(),
            b"abc123",
            ParseLimits {
                max_header_bytes_per_part: Some(8),
                ..ParseLimits::default()
            },
        ),
        Err(Error::HeaderBytesLimitExceeded { limit: 8, .. })
    ));
}
