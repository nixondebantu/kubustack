//! Raw HTTP/1.0 over a Unix socket and response parsing.

use super::DockerError;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

/// Sends a request and returns the whole response body.
/// Statuses >= 400 become `DockerError::Api`; 2xx and 304 are returned as-is.
pub fn request(
    socket: &Path,
    method: &str,
    path: &str,
    timeout: Duration,
) -> Result<Vec<u8>, DockerError> {
    let stream = connect(socket, method, path, Some(timeout))?;
    let mut reader = BufReader::new(stream);
    read_response(&mut reader).map_err(|e| io_or(e, socket))
}

fn connect(
    socket: &Path,
    method: &str,
    path: &str,
    timeout: Option<Duration>,
) -> Result<UnixStream, DockerError> {
    let io = |e| DockerError::from_io(e, socket);
    let mut s = UnixStream::connect(socket).map_err(io)?;
    s.set_read_timeout(timeout).map_err(io)?;
    s.set_write_timeout(Some(Duration::from_secs(15)))
        .map_err(io)?;
    // Connection: close because some socket proxies (e.g. OrbStack) keep HTTP/1.0 connections open.
    let len = if method == "POST" || method == "DELETE" {
        "Content-Length: 0\r\n"
    } else {
        ""
    };
    let req = format!(
        "{method} {path} HTTP/1.0\r\nHost: docker\r\nUser-Agent: kubustack/{}\r\nConnection: close\r\n{len}\r\n",
        env!("CARGO_PKG_VERSION")
    );
    s.write_all(req.as_bytes()).map_err(io)?;
    Ok(s)
}

/// Internal error: either a socket I/O error (mapped later with the socket path) or a final error.
enum ReadError {
    Io(std::io::Error),
    Docker(DockerError),
}

impl From<std::io::Error> for ReadError {
    fn from(e: std::io::Error) -> Self {
        ReadError::Io(e)
    }
}

fn io_or(e: ReadError, socket: &Path) -> DockerError {
    match e {
        ReadError::Io(e) => DockerError::from_io(e, socket),
        ReadError::Docker(e) => e,
    }
}

fn parse_err(msg: impl Into<String>) -> ReadError {
    ReadError::Docker(DockerError::Parse(msg.into()))
}

fn read_response(r: &mut dyn BufRead) -> Result<Vec<u8>, ReadError> {
    let head = read_head(r)?;
    let body = read_body(r, &head)?;
    let status = head.status;
    if status >= 400 {
        return Err(ReadError::Docker(api_error(status, &body)));
    }
    Ok(body)
}

struct Head {
    status: u16,
    chunked: bool,
    content_length: Option<u64>,
}

/// Reads the status line and headers.
fn read_head(r: &mut dyn BufRead) -> Result<Head, ReadError> {
    let mut line = String::new();
    r.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let status = match (parts.next(), parts.next()) {
        (Some(v), Some(code)) if v.starts_with("HTTP/") => code.parse::<u16>().ok(),
        _ => None,
    }
    .ok_or_else(|| parse_err(format!("bad status line {:?}", line.trim_end())))?;

    let mut head = Head {
        status,
        chunked: false,
        content_length: None,
    };
    loop {
        line.clear();
        if r.read_line(&mut line)? == 0 {
            return Err(parse_err("response ended inside the headers"));
        }
        let h = line.trim_end();
        if h.is_empty() {
            return Ok(head);
        }
        if let Some((name, value)) = h.split_once(':') {
            if name.eq_ignore_ascii_case("transfer-encoding") {
                head.chunked = value.to_ascii_lowercase().contains("chunked");
            } else if name.eq_ignore_ascii_case("content-length") {
                head.content_length = value.trim().parse().ok();
            }
        }
    }
}

/// Reads the body: chunked, exactly Content-Length bytes, or until the server closes.
fn read_body(r: &mut dyn BufRead, head: &Head) -> Result<Vec<u8>, ReadError> {
    let mut body = Vec::new();
    match head.content_length {
        Some(len) if !head.chunked => {
            Read::take(&mut *r, len).read_to_end(&mut body)?;
            if (body.len() as u64) < len {
                return Err(parse_err("response was cut short"));
            }
        }
        _ => {
            r.read_to_end(&mut body)?;
        }
    }
    if head.chunked {
        body = dechunk(&body).ok_or_else(|| parse_err("malformed chunked body"))?;
    }
    Ok(body)
}

/// Percent-encodes a query value (everything except RFC 3986 unreserved characters).
pub fn encode(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Decodes a `Transfer-Encoding: chunked` body. Returns `None` if it is malformed.
fn dechunk(mut data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let eol = data.windows(2).position(|w| w == b"\r\n")?;
        let size_line = std::str::from_utf8(&data[..eol]).ok()?;
        let size_hex = size_line.split(';').next()?.trim();
        let size = usize::from_str_radix(size_hex, 16).ok()?;
        data = &data[eol + 2..];
        if size == 0 {
            return Some(out);
        }
        if data.len() < size + 2 || &data[size..size + 2] != b"\r\n" {
            return None;
        }
        out.extend_from_slice(&data[..size]);
        data = &data[size + 2..];
    }
}

/// Builds an API error from Docker's `{"message": "..."}` body, falling back to the raw text.
fn api_error(status: u16, body: &[u8]) -> DockerError {
    #[derive(serde::Deserialize)]
    struct ErrBody {
        message: String,
    }
    let message = match serde_json::from_slice::<ErrBody>(body) {
        Ok(e) => e.message,
        Err(_) => String::from_utf8_lossy(body).trim().to_string(),
    };
    DockerError::Api { status, message }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Parses a raw HTTP response (used by fixture tests in other modules too).
    pub fn parse(raw: &[u8]) -> Result<Vec<u8>, DockerError> {
        let mut r: &[u8] = raw;
        read_response(&mut r).map_err(|e| io_or(e, Path::new("/test.sock")))
    }

    #[test]
    fn ping_fixture() {
        assert_eq!(
            parse(include_bytes!("../../tests/fixtures/ping.http")).unwrap(),
            b"OK"
        );
    }

    #[test]
    fn version_fixture_body_is_json() {
        let body = parse(include_bytes!("../../tests/fixtures/version.http")).unwrap();
        assert!(body.starts_with(b"{\"Platform\""));
    }

    #[test]
    fn empty_204() {
        assert!(parse(b"HTTP/1.1 204 No Content\r\nServer: Docker\r\n\r\n")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn not_modified_is_not_an_error() {
        assert!(parse(b"HTTP/1.1 304 Not Modified\r\n\r\n").is_ok());
    }

    #[test]
    fn error_json_fixture() {
        let err = parse(include_bytes!("../../tests/fixtures/error_404.http")).err();
        match err {
            Some(DockerError::Api { status, message }) => {
                assert_eq!(status, 404);
                assert_eq!(message, "No such container: kubustack-test-missing");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn error_without_json_uses_text_or_status() {
        let e = parse(b"HTTP/1.1 500 Oops\r\n\r\nboom\n").err().unwrap();
        assert_eq!(e.to_string(), "boom");
        let e = parse(b"HTTP/1.1 500 Oops\r\n\r\n").err().unwrap();
        assert_eq!(e.to_string(), "Docker returned HTTP 500");
    }

    #[test]
    fn bad_responses_are_parse_errors() {
        for raw in [
            &b""[..],
            b"garbage\r\n\r\n",
            b"HTTP/1.1 abc\r\n\r\n",
            b"HTTP/1.1 200 OK\r\nX: y\r\n",
        ] {
            assert!(matches!(parse(raw), Err(DockerError::Parse(_))), "{raw:?}");
        }
    }

    #[test]
    fn content_length_stops_reading() {
        // Bytes after Content-Length belong to nobody (the connection may stay open).
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOKextra";
        assert_eq!(parse(raw).unwrap(), b"OK");
        let short = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nOK";
        assert!(matches!(parse(short), Err(DockerError::Parse(_))));
    }

    #[test]
    fn real_chunked_fixture() {
        // OrbStack answers HTTP/1.0 requests with chunked bodies.
        let body = parse(include_bytes!("../../tests/fixtures/containers.http")).unwrap();
        assert!(body.starts_with(b"[{") && body.trim_ascii_end().ends_with(b"}]"));
    }

    #[test]
    fn chunked_response() {
        let raw = b"HTTP/1.1 200 OK\r\ntransfer-encoding: Chunked\r\n\r\n4\r\nWiki\r\n6;ext=1\r\npedia \r\nE\r\nin \r\n\r\nchunks.\r\n0\r\n\r\n";
        assert_eq!(parse(raw).unwrap(), b"Wikipedia in \r\n\r\nchunks.");
    }

    #[test]
    fn encodes_filters_json() {
        assert_eq!(
            encode(r#"{"dangling":["true"]}"#),
            "%7B%22dangling%22%3A%5B%22true%22%5D%7D"
        );
        assert_eq!(encode("a-b_c.d~e"), "a-b_c.d~e");
        assert_eq!(encode("é /"), "%C3%A9%20%2F");
    }

    #[test]
    fn dechunk_edge_cases() {
        assert_eq!(dechunk(b"0\r\n\r\n").unwrap(), b"");
        assert_eq!(dechunk(b"a\r\n0123456789\r\n0\r\n").unwrap(), b"0123456789");
        assert!(dechunk(b"5\r\nabc\r\n0\r\n").is_none()); // short chunk
        assert!(dechunk(b"zz\r\nabc\r\n").is_none()); // bad size
        assert!(dechunk(b"3\r\nabc\r\n").is_none()); // missing final chunk
    }
}
