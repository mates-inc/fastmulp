use fastmulp_core::{Boundary, Error, boundary_from_content_type};

#[test]
fn extracts_boundary_case_insensitively_and_skips_other_params() {
    let content_type = "Multipart/Form-Data; charset=UTF-8; boundary=abc123; foo=bar";
    assert_eq!(
        boundary_from_content_type(content_type).as_deref(),
        Some("abc123")
    );
}

#[test]
fn extracts_quoted_boundary_with_optional_spacing() {
    let content_type = "multipart/form-data; foo=bar; boundary = \"quoted-boundary\"";
    assert_eq!(
        boundary_from_content_type(content_type).as_deref(),
        Some("quoted-boundary")
    );
}

#[test]
fn extracts_quoted_boundary_with_whitespace_before_next_parameter() {
    let content_type = "multipart/form-data; boundary=\"quoted-boundary\" \t ; charset=UTF-8";
    assert_eq!(
        boundary_from_content_type(content_type).as_deref(),
        Some("quoted-boundary")
    );
}

#[test]
fn accepts_quoted_pair_escapes_while_scanning_parameters() {
    let content_type = "multipart/form-data; title=\"needs\\\"escape\"; boundary=abc123";
    assert_eq!(
        boundary_from_content_type(content_type).as_deref(),
        Some("abc123")
    );
}

#[test]
fn extracts_escaped_quoted_boundary_values_requiring_unescape() {
    let content_type = "multipart/form-data; boundary=\"abc\\:123\"";
    assert_eq!(
        boundary_from_content_type(content_type).as_deref(),
        Some("abc:123")
    );
}

#[test]
fn rejects_invalid_quoted_values() {
    let cases = [
        "multipart/form-data; boundary=\"unterminated",
        "multipart/form-data; charset=\"unterminated\\",
        "multipart/form-data; charset=\"value\" junk; boundary=abc123",
        "multipart/form-data; boundary=\"quoted-boundary\" junk",
    ];

    for content_type in cases {
        assert_eq!(
            boundary_from_content_type(content_type),
            None,
            "{content_type}"
        );
    }
}

#[test]
fn rejects_missing_boundary_parameter() {
    let content_type = "multipart/form-data; charset=UTF-8";
    assert_eq!(boundary_from_content_type(content_type), None);
}

#[test]
fn validates_boundary_length_and_bytes() {
    assert!(Boundary::new(b"abc123").is_ok());
    assert!(Boundary::new(b"abc def").is_ok());
    assert!(matches!(
        Boundary::new(b"abc "),
        Err(Error::InvalidBoundaryByte { .. })
    ));
    assert!(matches!(
        Boundary::new(b"abc\"123"),
        Err(Error::InvalidBoundaryByte { .. })
    ));
    assert!(matches!(
        Boundary::new(&[b'a'; 71]),
        Err(Error::BoundaryTooLong { .. })
    ));
}
