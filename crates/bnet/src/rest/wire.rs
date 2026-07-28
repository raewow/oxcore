//! A hand-rolled HTTP/1.1 writer for the login service.
//!
//! The routes and handlers are still an ordinary axum [`Router`]; only the bytes on the wire are
//! ours. hyper normalises response header names to lowercase, while this client requires
//! canonical header case and a minimal HTTP/1.1 response.
//!
//! `Content-Length` counts only the body even though a trailing CRLF is written after it.

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use axum::body::Body;
use axum::http::{HeaderName, HeaderValue, Method, Request, Uri};
use axum::Router;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tower::ServiceExt;
use tracing::debug;

/// Cap on a request's header block, so a client cannot make us buffer without bound.
const MAX_HEADER_BYTES: usize = 16 * 1024;
/// Cap on a request body. The largest thing the client posts is a filled-in login form.
const MAX_BODY_BYTES: usize = 64 * 1024;

/// Serve HTTP/1.1 requests off one (already TLS-wrapped) stream until the peer closes it.
pub async fn serve_connection<S>(mut stream: S, router: Router) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf: Vec<u8> = Vec::with_capacity(2048);

    loop {
        let Some(head_len) = read_until_headers_complete(&mut stream, &mut buf).await? else {
            return Ok(()); // clean EOF between requests
        };

        let (request, consumed) = parse_request(&mut stream, &mut buf, head_len).await?;
        buf.drain(..consumed);

        let keep_alive = wants_keep_alive(&request);
        let response = router
            .clone()
            .oneshot(request)
            .await
            .map_err(|e| anyhow::anyhow!("router failed: {e}"))?;

        let (parts, body) = response.into_parts();
        let body = axum::body::to_bytes(body, MAX_BODY_BYTES)
            .await
            .context("failed to collect response body")?;

        // The login browser requires this exact JSON content type. Non-JSON responses pass through.
        let declared = parts
            .headers
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/json");
        let content_type = if declared.starts_with("application/json") {
            "application/json;charset=UTF-8"
        } else {
            declared
        };

        debug!(
            status = parts.status.as_u16(),
            content_type,
            body = %String::from_utf8_lossy(&body),
            "REST wire response"
        );

        stream
            .write_all(&render_response(parts.status, content_type, &body))
            .await?;
        stream.flush().await?;

        if !keep_alive {
            return Ok(());
        }
    }
}

/// Serialise a response in the form required by the login browser.
fn render_response(status: axum::http::StatusCode, content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len() + 128);
    out.extend_from_slice(
        format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: {}\r\n\r\n",
            status.as_u16(),
            reason_phrase(status),
            body.len(),
            content_type,
        )
        .as_bytes(),
    );
    out.extend_from_slice(body);
    // The browser expects a final CRLF outside the declared body length.
    out.extend_from_slice(b"\r\n");
    out
}

/// Reason phrases used by the client's expected HTTP response format.
fn reason_phrase(status: axum::http::StatusCode) -> &'static str {
    match status.as_u16() {
        200 => "Ok",
        400 => "BadRequest",
        404 => "NotFound",
        500 => "InternalServerError",
        _ => status.canonical_reason().unwrap_or("Unknown"),
    }
}

/// Read until the header block is terminated, returning its length including the blank line, or
/// `None` on a clean EOF with nothing buffered.
async fn read_until_headers_complete<S>(stream: &mut S, buf: &mut Vec<u8>) -> Result<Option<usize>>
where
    S: AsyncRead + Unpin,
{
    let mut chunk = [0u8; 2048];
    loop {
        if let Some(pos) = find_header_end(buf) {
            return Ok(Some(pos));
        }
        if buf.len() > MAX_HEADER_BYTES {
            bail!("request headers exceeded {MAX_HEADER_BYTES} bytes");
        }

        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            if buf.is_empty() {
                return Ok(None);
            }
            bail!("connection closed mid-request");
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

/// Parse the request line, headers and (per `Content-Length`) body into an axum request. Returns
/// the request and how many bytes of `buf` it consumed.
async fn parse_request<S>(
    stream: &mut S,
    buf: &mut Vec<u8>,
    head_len: usize,
) -> Result<(Request<Body>, usize)>
where
    S: AsyncRead + Unpin,
{
    // Own the head before touching `buf` again — the body read below extends it.
    let head = String::from_utf8(buf[..head_len].to_vec()).context("request head is not UTF-8")?;
    let mut lines = head.split("\r\n");

    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split(' ');
    let (Some(method), Some(target)) = (parts.next(), parts.next()) else {
        bail!("malformed request line: {request_line:?}");
    };

    let mut headers: HashMap<String, String> = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let content_length: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if content_length > MAX_BODY_BYTES {
        bail!("request body of {content_length} bytes exceeds the {MAX_BODY_BYTES} cap");
    }

    // Pull in the rest of the body if it has not all arrived yet.
    let total = head_len + content_length;
    let mut chunk = [0u8; 2048];
    while buf.len() < total {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            bail!("connection closed mid-body");
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let body = buf[head_len..total].to_vec();

    let mut request = Request::builder()
        .method(Method::from_bytes(method.as_bytes()).context("unsupported HTTP method")?)
        .uri(
            target
                .parse::<Uri>()
                .context("unparseable request target")?,
        )
        .body(Body::from(body))
        .context("failed to build request")?;

    for (name, value) in &headers {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            request.headers_mut().insert(name, value);
        }
    }

    debug!(%method, %target, "REST wire request");
    Ok((request, total))
}

/// HTTP/1.1 keeps the connection alive unless the client says otherwise.
fn wants_keep_alive(request: &Request<Body>) -> bool {
    request
        .headers()
        .get(axum::http::header::CONNECTION)
        .and_then(|v| v.to_str().ok())
        .map(|v| !v.eq_ignore_ascii_case("close"))
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn response_uses_canonical_header_case_and_expected_reason_phrase() {
        let out = render_response(StatusCode::OK, "application/json;charset=UTF-8", b"{}");
        let text = String::from_utf8(out).unwrap();

        assert!(text.starts_with("HTTP/1.1 200 Ok\r\n"), "got {text:?}");
        assert!(text.contains("Content-Length: 2\r\n"));
        assert!(text.contains("Content-Type: application/json;charset=UTF-8\r\n"));
        // Never lowercase — that is the whole reason this module exists.
        assert!(!text.contains("content-length:"));
        assert!(!text.contains("content-type:"));
        // Keep the response header block minimal.
        assert!(!text.to_ascii_lowercase().contains("date:"));
        assert!(!text.to_ascii_lowercase().contains("server:"));
    }

    #[test]
    fn body_is_followed_by_a_crlf_past_the_declared_length() {
        let out = render_response(StatusCode::OK, "application/json", b"{\"a\":1}");
        assert!(out.ends_with(b"{\"a\":1}\r\n"));
        assert!(String::from_utf8_lossy(&out).contains("Content-Length: 7\r\n"));
    }

    #[test]
    fn header_end_is_found_only_after_the_blank_line() {
        assert_eq!(find_header_end(b"GET / HTTP/1.1\r\nHost: x\r\n"), None);
        assert_eq!(
            find_header_end(b"GET / HTTP/1.1\r\nHost: x\r\n\r\nbody"),
            Some(27)
        );
    }

    #[tokio::test]
    async fn a_request_with_a_body_is_parsed_whole() {
        let raw = b"POST /bnetserver/login/ HTTP/1.1\r\nContent-Length: 10\r\nContent-Type: application/json\r\n\r\n{\"a\":1234}";
        let mut buf = raw[..raw.len() - 1].to_vec(); // body arrives one byte short
        let mut rest: &[u8] = b"}";

        let head_len = find_header_end(&buf).unwrap();
        let (request, consumed) = parse_request(&mut rest, &mut buf, head_len).await.unwrap();

        assert_eq!(request.method(), Method::POST);
        assert_eq!(request.uri().path(), "/bnetserver/login/");
        assert_eq!(consumed, head_len + 10);
        let body = axum::body::to_bytes(request.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(&body[..], b"{\"a\":1234}".as_slice());
    }
}
