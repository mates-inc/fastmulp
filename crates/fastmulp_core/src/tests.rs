use insta::assert_snapshot;

use crate::{Error, MultipartParser, boundary_from_content_type, parse};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn extracts_boundary_from_content_type() {
    assert_eq!(
        boundary_from_content_type("multipart/form-data; boundary=----WebKitFormBoundaryabc123")
            .as_deref(),
        Some("----WebKitFormBoundaryabc123")
    );
    assert_eq!(
        boundary_from_content_type("multipart/form-data; boundary=\"quoted-boundary\"").as_deref(),
        Some("quoted-boundary")
    );
    assert_eq!(boundary_from_content_type("text/plain"), None);
}

#[test]
fn parses_browser_style_payload() -> TestResult {
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = concat!(
        "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n",
        "Content-Disposition: form-data; name=\"field\"\r\n",
        "\r\n",
        "hello world\r\n",
        "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n",
        "Content-Disposition: form-data; name=\"file\"; filename=\"hello.txt\"\r\n",
        "Content-Type: text/plain\r\n",
        "\r\n",
        "hello file\r\n",
        "------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n",
    );
    let multipart = parse(body.as_bytes(), boundary.as_bytes())?;

    let mut snapshot = String::new();
    for (index, part) in multipart.parts().iter().enumerate() {
        let body_text = core::str::from_utf8(part.body(multipart.body()))?;
        let name = part
            .name()
            .and_then(|value| value.as_str().ok())
            .unwrap_or("-");
        let file_name = part
            .file_name()
            .and_then(|value| value.as_str().ok())
            .unwrap_or("-");
        let content_type = part
            .content_type()
            .and_then(|value| core::str::from_utf8(value).ok())
            .unwrap_or("-");
        snapshot.push_str(&format!(
      "#{index} name={name} file_name={file_name} content_type={content_type} range={:?} body={body_text}\n",
      part.body_range()
    ));
        for header in part.headers() {
            let header_name = core::str::from_utf8(header.name())?;
            let header_value = core::str::from_utf8(header.value())?;
            snapshot.push_str(&format!("header {header_name}={header_value}\n",));
        }
    }

    assert_snapshot!(
      snapshot,
      @r###"
    #0 name=field file_name=- content_type=- range=89..100 body=hello world
    header Content-Disposition=form-data; name="field"
    #1 name=file file_name=hello.txt content_type=text/plain range=238..248 body=hello file
    header Content-Disposition=form-data; name="file"; filename="hello.txt"
    header Content-Type=text/plain
    "###
    );

    Ok(())
}

#[test]
fn prefers_filename_star() -> TestResult {
    let boundary = "abc123";
    let body = concat!(
        "--abc123\r\n",
        "Content-Disposition: form-data; name=\"file\"; filename=\"fallback.txt\"; filename*=UTF-8''fancy%20name.txt\r\n",
        "\r\n",
        "payload\r\n",
        "--abc123--\r\n",
    );

    let multipart = parse(body.as_bytes(), boundary.as_bytes())?;
    let file_name = multipart.parts()[0]
        .file_name()
        .and_then(|value| value.as_str().ok())
        .ok_or_else(|| std::io::Error::other("filename should exist"))?;
    assert_eq!(file_name, "fancy name.txt");
    Ok(())
}

#[test]
fn accepts_epilogue_after_closing_boundary() -> TestResult {
    let boundary = "abc123";
    let body = concat!(
        "--abc123\r\n",
        "Content-Disposition: form-data; name=\"field\"\r\n",
        "\r\n",
        "payload\r\n",
        "--abc123--\r\n",
        "extra",
    );

    let multipart = parse(body.as_bytes(), boundary.as_bytes())?;
    assert_eq!(multipart.parts().len(), 1);
    Ok(())
}

#[test]
fn multipart_parser_iterator_stops_after_part_error() -> TestResult {
    let boundary = "abc123";
    let body = concat!(
        "--abc123\r\n",
        "Content-Disposition: form-data; name=\"field\"\n",
    );

    let mut parser = MultipartParser::new(body.as_bytes(), boundary.as_bytes())?;

    assert!(matches!(
        parser.next(),
        Some(Err(Error::InvalidHeaderLineEnding { .. }))
    ));
    assert!(parser.next().is_none());
    Ok(())
}

#[test]
fn ignores_boundary_like_bytes_inside_part_body() -> TestResult {
    let boundary = "abc123";
    let body = concat!(
        "--abc123\r\n",
        "Content-Disposition: form-data; name=\"field\"\r\n",
        "\r\n",
        "payload\r\n",
        "--abc123--x\r\n",
        "still payload\r\n",
        "--abc123--\r\n",
    );

    let multipart = parse(body.as_bytes(), boundary.as_bytes())?;
    let payload = core::str::from_utf8(multipart.parts()[0].body(multipart.body()))?;
    assert_eq!(payload, "payload\r\n--abc123--x\r\nstill payload");
    Ok(())
}
