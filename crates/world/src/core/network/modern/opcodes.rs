//! Handshake opcode numbers for the modern (1.14.x) wire.
//!
//! These are thin aliases over the shared [`Opcode`] table, which is the single source of truth for
//! every protocol's numbering. They exist because the handshake predates that table and reads
//! better as bare `u16`s: the framing layer works in raw wire numbers, not `Opcode` values, since
//! it runs before any session exists to say which protocol is in play.
//!
//! The values are asserted against the shared table below, so the two cannot drift.
//!
//! [`Opcode`]: oxcore_shared::protocol::Opcode

use oxcore_shared::protocol::Opcode;

/// Server → client: the auth challenge (DoS challenge + server challenge).
pub const SMSG_AUTH_CHALLENGE: u16 = Opcode::SMSG_AUTH_CHALLENGE.modern();
/// Client → server: the auth session (realm-join ticket + digest proving the session key).
pub const CMSG_AUTH_SESSION: u16 = Opcode::CMSG_AUTH_SESSION.modern();
/// Server → client: switch to AES-GCM encrypted mode (RSA-signed key).
pub const SMSG_ENTER_ENCRYPTED_MODE: u16 = Opcode::SMSG_ENTER_ENCRYPTED_MODE.modern();
/// Client → server: acknowledgement that encrypted mode is active.
pub const CMSG_ENTER_ENCRYPTED_MODE_ACK: u16 = Opcode::CMSG_ENTER_ENCRYPTED_MODE_ACK.modern();
/// Server → client: the auth response (realm/character enablement).
pub const SMSG_AUTH_RESPONSE: u16 = Opcode::SMSG_AUTH_RESPONSE.modern();
/// Server → client: redirect to an instance socket.
pub const SMSG_CONNECT_TO: u16 = Opcode::SMSG_CONNECT_TO.modern();
/// Client → server: the continued-session handshake on an instance socket.
pub const CMSG_AUTH_CONTINUED_SESSION: u16 = Opcode::CMSG_AUTH_CONTINUED_SESSION.modern();
/// Client → server: latency ping.
pub const CMSG_PING: u16 = Opcode::CMSG_PING.modern();
/// Server → client: pong.
pub const SMSG_PONG: u16 = Opcode::SMSG_PONG.modern();

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the values a live 1.14.2 client was observed using. Deriving them from the shared table
    /// means a regeneration against the wrong build would silently move them; this catches that.
    #[test]
    fn handshake_opcodes_match_the_observed_client() {
        assert_eq!(SMSG_AUTH_CHALLENGE, 0x3048);
        assert_eq!(CMSG_AUTH_SESSION, 0x3765);
        assert_eq!(SMSG_ENTER_ENCRYPTED_MODE, 0x3049);
        assert_eq!(CMSG_ENTER_ENCRYPTED_MODE_ACK, 0x3767);
        assert_eq!(SMSG_AUTH_RESPONSE, 0x256D);
        assert_eq!(SMSG_CONNECT_TO, 0x304D);
        assert_eq!(CMSG_AUTH_CONTINUED_SESSION, 0x3766);
        assert_eq!(CMSG_PING, 0x3768);
        assert_eq!(SMSG_PONG, 0x304E);
    }

    /// None of these may be zero: zero means "absent from this protocol", which for a handshake
    /// opcode would mean the table lost it.
    #[test]
    fn every_handshake_opcode_exists_in_the_modern_table() {
        for (name, value) in [
            ("SMSG_AUTH_CHALLENGE", SMSG_AUTH_CHALLENGE),
            ("CMSG_AUTH_SESSION", CMSG_AUTH_SESSION),
            ("SMSG_ENTER_ENCRYPTED_MODE", SMSG_ENTER_ENCRYPTED_MODE),
            (
                "CMSG_ENTER_ENCRYPTED_MODE_ACK",
                CMSG_ENTER_ENCRYPTED_MODE_ACK,
            ),
            ("SMSG_AUTH_RESPONSE", SMSG_AUTH_RESPONSE),
            ("SMSG_CONNECT_TO", SMSG_CONNECT_TO),
            ("CMSG_AUTH_CONTINUED_SESSION", CMSG_AUTH_CONTINUED_SESSION),
            ("CMSG_PING", CMSG_PING),
            ("SMSG_PONG", SMSG_PONG),
        ] {
            assert_ne!(value, 0, "{name} is missing a modern wire number");
        }
    }
}
