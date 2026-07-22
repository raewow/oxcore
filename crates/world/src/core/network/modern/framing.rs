//! Modern (1.14.x) world packet framing.
//!
//! A modern connection opens with a plaintext handshake — the server sends
//! [`CONNECTION_INITIALIZE_SERVER`], the client replies with [`CONNECTION_INITIALIZE_CLIENT`] —
//! after which every packet is framed as:
//!
//! ```text
//! [u32 size, little-endian][12-byte GCM tag][ciphertext]
//! ```
//!
//! where `size` is the plaintext length (which AES-GCM preserves, so it is also the ciphertext
//! length) and the ciphertext decrypts to `opcode (u16 LE) ‖ body`. Transcribed from HermesProxy's
//! modern server `WorldSocket.cs`.

use anyhow::{bail, Result};

use super::crypt::{WorldCrypt, TAG_SIZE};

/// Plaintext greeting the server sends on connect, before any framed packet.
pub const CONNECTION_INITIALIZE_SERVER: &str =
    "WORLD OF WARCRAFT CONNECTION - SERVER TO CLIENT - V2";
/// Plaintext greeting the client sends back.
pub const CONNECTION_INITIALIZE_CLIENT: &str =
    "WORLD OF WARCRAFT CONNECTION - CLIENT TO SERVER - V2";

/// Size of the framed header: a u32 size plus the 12-byte tag.
pub const HEADER_SIZE: usize = 4 + TAG_SIZE;

/// A decoded modern packet: its opcode and the plaintext body (opcode stripped).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModernPacket {
    pub opcode: u16,
    pub body: Vec<u8>,
}

/// Frame and encrypt one packet for sending. Encrypts `opcode ‖ body` in place, then prepends the
/// plaintext size and the GCM tag.
pub fn encode(crypt: &mut WorldCrypt, opcode: u16, body: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(2 + body.len());
    data.extend_from_slice(&opcode.to_le_bytes());
    data.extend_from_slice(body);

    let size = data.len() as u32;
    let tag = crypt.encrypt(&mut data);

    let mut out = Vec::with_capacity(HEADER_SIZE + data.len());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&tag);
    out.extend_from_slice(&data);
    out
}

/// Try to decode one packet from the front of `buf`.
///
/// Returns `Ok(Some((packet, consumed)))` when a whole frame is present and its tag verifies,
/// `Ok(None)` when more bytes are needed, or `Err` when the tag fails or the frame is malformed.
/// `consumed` is how many bytes to drain on success.
pub fn decode(crypt: &mut WorldCrypt, buf: &[u8]) -> Result<Option<(ModernPacket, usize)>> {
    if buf.len() < HEADER_SIZE {
        return Ok(None);
    }

    let size = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    let mut tag = [0u8; TAG_SIZE];
    tag.copy_from_slice(&buf[4..HEADER_SIZE]);

    let end = HEADER_SIZE + size;
    if buf.len() < end {
        return Ok(None);
    }

    let mut data = buf[HEADER_SIZE..end].to_vec();
    crypt.decrypt(&mut data, &tag)?;

    if data.len() < 2 {
        bail!("decrypted packet too small to hold an opcode ({} bytes)", data.len());
    }
    let opcode = u16::from_le_bytes([data[0], data[1]]);
    let body = data[2..].to_vec();

    Ok(Some((ModernPacket { opcode, body }, end)))
}

/// Frame a packet **before encryption is enabled**. The header is the same 16 bytes, but the
/// crypt is a no-op: the tag is all zeros and the `opcode ‖ body` is plaintext. This is how the
/// auth challenge and session are exchanged before `SMSG_ENTER_ENCRYPTED_MODE`.
pub fn encode_plaintext(opcode: u16, body: &[u8]) -> Vec<u8> {
    let size = (2 + body.len()) as u32;
    let mut out = Vec::with_capacity(HEADER_SIZE + 2 + body.len());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&[0u8; TAG_SIZE]); // zero tag: no authentication before encryption
    out.extend_from_slice(&opcode.to_le_bytes());
    out.extend_from_slice(body);
    out
}

/// Decode a pre-encryption frame written by [`encode_plaintext`]: the 16-byte header is skipped
/// (the tag is ignored) and the `opcode ‖ body` is read as plaintext.
pub fn decode_plaintext(buf: &[u8]) -> Result<Option<(ModernPacket, usize)>> {
    if buf.len() < HEADER_SIZE {
        return Ok(None);
    }
    let size = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    let end = HEADER_SIZE + size;
    if buf.len() < end {
        return Ok(None);
    }
    let data = &buf[HEADER_SIZE..end];
    if data.len() < 2 {
        bail!("plaintext packet too small to hold an opcode ({} bytes)", data.len());
    }
    let opcode = u16::from_le_bytes([data[0], data[1]]);
    let body = data[2..].to_vec();
    Ok(Some((ModernPacket { opcode, body }, end)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 16] = [7u8; 16];

    #[test]
    fn plaintext_frame_round_trips_without_a_key() {
        let frame = encode_plaintext(0x3048, b"challenge bytes");
        // Header size counts opcode + body; the 12 tag bytes are zero.
        let size = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
        assert_eq!(size, 2 + b"challenge bytes".len());
        assert_eq!(&frame[4..HEADER_SIZE], &[0u8; TAG_SIZE]);

        let (packet, consumed) = decode_plaintext(&frame).unwrap().unwrap();
        assert_eq!(consumed, frame.len());
        assert_eq!(packet.opcode, 0x3048);
        assert_eq!(packet.body, b"challenge bytes");
    }

    #[test]
    fn init_strings_are_the_v2_greetings() {
        assert!(CONNECTION_INITIALIZE_SERVER.ends_with("SERVER TO CLIENT - V2"));
        assert!(CONNECTION_INITIALIZE_CLIENT.ends_with("CLIENT TO SERVER - V2"));
    }

    #[test]
    fn encode_produces_the_expected_header_layout() {
        let mut server = WorldCrypt::server(&KEY);
        let body = b"hello world server".to_vec();
        let frame = encode(&mut server, 0x01F4, &body);

        // Header: u32 size (= opcode + body), then a 12-byte tag, then ciphertext of that size.
        let size = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
        assert_eq!(size, 2 + body.len());
        assert_eq!(frame.len(), HEADER_SIZE + size);
    }

    #[test]
    fn server_encoded_packet_decodes_on_the_client() {
        let mut server = WorldCrypt::server(&KEY);
        let mut client = WorldCrypt::client(&KEY);

        let frame = encode(&mut server, 0x0ABC, b"payload bytes");
        let (packet, consumed) = decode(&mut client, &frame).unwrap().expect("a whole frame");
        assert_eq!(consumed, frame.len());
        assert_eq!(packet.opcode, 0x0ABC);
        assert_eq!(packet.body, b"payload bytes");
    }

    #[test]
    fn decode_waits_for_the_full_header_then_the_full_body() {
        let mut server = WorldCrypt::server(&KEY);
        let mut client = WorldCrypt::client(&KEY);

        let frame = encode(&mut server, 1, b"abcdef");
        assert!(decode(&mut client, &frame[..HEADER_SIZE - 1]).unwrap().is_none());
        assert!(decode(&mut client, &frame[..frame.len() - 1]).unwrap().is_none());
        assert!(decode(&mut client, &frame).unwrap().is_some());
    }

    #[test]
    fn two_concatenated_packets_decode_in_sequence() {
        let mut server = WorldCrypt::server(&KEY);
        let mut client = WorldCrypt::client(&KEY);

        let mut stream = encode(&mut server, 10, b"first");
        stream.extend(encode(&mut server, 20, b"second"));

        let (p1, c1) = decode(&mut client, &stream).unwrap().unwrap();
        assert_eq!(p1.opcode, 10);
        assert_eq!(p1.body, b"first");

        let (p2, c2) = decode(&mut client, &stream[c1..]).unwrap().unwrap();
        assert_eq!(p2.opcode, 20);
        assert_eq!(p2.body, b"second");
        assert_eq!(c1 + c2, stream.len());
    }

    #[test]
    fn a_tampered_body_is_rejected() {
        let mut server = WorldCrypt::server(&KEY);
        let mut client = WorldCrypt::client(&KEY);

        let mut frame = encode(&mut server, 1, b"authentic");
        let last = frame.len() - 1;
        frame[last] ^= 0xFF; // flip a ciphertext byte
        assert!(decode(&mut client, &frame).is_err());
    }
}
