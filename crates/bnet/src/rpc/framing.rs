//! BGS RPC frame codec.
//!
//! Every message on the :1119 channel is framed as:
//!
//! ```text
//! [u16 header length, big-endian][Header protobuf][payload]
//! ```
//!
//! where the payload is exactly `header.size` bytes. Requests carry a `service_hash`
//! (fixed32) + `method_id` identifying the call; responses set `service_id = 0xFE` and echo the
//! request `token`.

use anyhow::{bail, Result};
use prost::Message;

use super::proto::Header;

/// Service id marking a frame as a response rather than a request.
pub const RESPONSE_SERVICE_ID: u32 = 0xFE;

/// A decoded frame: its header plus the raw payload bytes.
#[derive(Debug, Clone)]
pub struct Frame {
    pub header: Header,
    pub payload: Vec<u8>,
}

/// Encode a header + payload into wire bytes. The header's `size` is overwritten to match the
/// payload length, so callers never have to keep the two in sync by hand.
pub fn encode(mut header: Header, payload: &[u8]) -> Result<Vec<u8>> {
    header.size = Some(payload.len() as u32);

    let header_bytes = header.encode_to_vec();
    if header_bytes.len() > u16::MAX as usize {
        bail!("header too large to frame: {} bytes", header_bytes.len());
    }

    let mut out = Vec::with_capacity(2 + header_bytes.len() + payload.len());
    out.extend_from_slice(&(header_bytes.len() as u16).to_be_bytes());
    out.extend_from_slice(&header_bytes);
    out.extend_from_slice(payload);
    Ok(out)
}

/// Build the header for a response to `request`: `service_id = 0xFE`, the request routing keys
/// and token echoed, and the given status. The client requires service hash/method id here to
/// 1.14.2 uses them to complete the Logon call after handling its listener callback.
pub fn response_header(request: &Header, status: u32) -> Header {
    Header {
        service_id: Some(RESPONSE_SERVICE_ID),
        method_id: request.method_id,
        token: request.token,
        status: Some(status),
        service_hash: request.service_hash,
        ..Default::default()
    }
}

/// Build the header for a *server-initiated* request (a listener callback): `service_id = 0`, the
/// target `service_hash` + `method_id`, and a server-chosen `token`. `size` is filled in by
/// [`encode`].
pub fn request_header(service_hash: u32, method_id: u32, token: u32) -> Header {
    Header {
        service_id: Some(0),
        service_hash: Some(service_hash),
        method_id: Some(method_id),
        token: Some(token),
        // The bundled 1.14.2 client rejects the callback when this proto2 field is omitted.
        status: Some(0),
        ..Default::default()
    }
}

/// Try to decode a single frame from the front of `buf`.
///
/// Returns `Ok(Some((frame, consumed)))` when a whole frame is present, or `Ok(None)` when more
/// bytes are needed — the caller keeps the buffer and retries after reading more. `consumed` is
/// how many bytes to drain on success.
pub fn decode(buf: &[u8]) -> Result<Option<(Frame, usize)>> {
    if buf.len() < 2 {
        return Ok(None);
    }
    let header_len = u16::from_be_bytes([buf[0], buf[1]]) as usize;

    let header_end = 2 + header_len;
    if buf.len() < header_end {
        return Ok(None);
    }

    let header = Header::decode(&buf[2..header_end])
        .map_err(|e| anyhow::anyhow!("failed to decode frame header: {e}"))?;

    let payload_len = header.size.unwrap_or(0) as usize;
    let payload_end = header_end + payload_len;
    if buf.len() < payload_end {
        return Ok(None);
    }

    let payload = buf[header_end..payload_end].to_vec();
    Ok(Some((Frame { header, payload }, payload_end)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_header(service_hash: u32, method_id: u32, token: u32) -> Header {
        Header {
            service_id: Some(0),
            method_id: Some(method_id),
            token: Some(token),
            service_hash: Some(service_hash),
            ..Default::default()
        }
    }

    #[test]
    fn round_trips_a_frame() {
        let payload = b"hello bgs".to_vec();
        let bytes = encode(request_header(0x6544_6991, 1, 42), &payload).unwrap();

        // Length prefix is big-endian and covers only the header.
        let header_len = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
        assert_eq!(bytes.len(), 2 + header_len + payload.len());

        let (frame, consumed) = decode(&bytes).unwrap().expect("a whole frame");
        assert_eq!(consumed, bytes.len());
        assert_eq!(frame.header.service_hash, Some(0x6544_6991));
        assert_eq!(frame.header.method_id, Some(1));
        assert_eq!(frame.header.token, Some(42));
        assert_eq!(frame.header.size, Some(payload.len() as u32));
        assert_eq!(frame.payload, payload);
    }

    #[test]
    fn encode_overwrites_size_to_match_payload() {
        // A deliberately-wrong size must be corrected by encode.
        let mut header = request_header(1, 1, 1);
        header.size = Some(9999);
        let bytes = encode(header, b"abcd").unwrap();
        let (frame, _) = decode(&bytes).unwrap().unwrap();
        assert_eq!(frame.header.size, Some(4));
    }

    #[test]
    fn decode_needs_the_full_header_then_the_full_payload() {
        let bytes = encode(request_header(1, 1, 1), b"payload!!").unwrap();

        // Nothing until the 2-byte length is present.
        assert!(decode(&bytes[..1]).unwrap().is_none());
        // Not until the whole header is present.
        assert!(decode(&bytes[..3]).unwrap().is_none());
        // Not until the whole payload is present.
        assert!(decode(&bytes[..bytes.len() - 1]).unwrap().is_none());
        // Complete.
        assert!(decode(&bytes).unwrap().is_some());
    }

    #[test]
    fn decodes_two_concatenated_frames() {
        let mut stream = encode(request_header(1, 1, 10), b"first").unwrap();
        stream.extend(encode(request_header(2, 5, 20), b"second").unwrap());

        let (f1, c1) = decode(&stream).unwrap().unwrap();
        assert_eq!(f1.header.token, Some(10));
        assert_eq!(f1.payload, b"first");

        let (f2, c2) = decode(&stream[c1..]).unwrap().unwrap();
        assert_eq!(f2.header.token, Some(20));
        assert_eq!(f2.payload, b"second");
        assert_eq!(c1 + c2, stream.len());
    }

    #[test]
    fn response_header_marks_response_and_echoes_token() {
        let req = request_header(0x6544_6991, 1, 77);
        let resp = response_header(&req, 0);
        assert_eq!(resp.service_id, Some(RESPONSE_SERVICE_ID));
        assert_eq!(resp.token, Some(77));
        assert_eq!(resp.status, Some(0));
        assert_eq!(resp.service_hash, Some(0x6544_6991));
        assert_eq!(resp.method_id, Some(1));
    }

    #[test]
    fn empty_payload_frame_round_trips() {
        let bytes = encode(request_header(0x6544_6991, 5, 1), &[]).unwrap();
        let (frame, consumed) = decode(&bytes).unwrap().unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(frame.header.size, Some(0));
        assert!(frame.payload.is_empty());
    }
}
