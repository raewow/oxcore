//! Modern world opcode numbers for the auth handshake.
//!
//! These are **build-specific** — the values here are for 1.14.1 build **40688**, taken from
//! HermesProxy's `V1_14_1_40688/Opcode.cs`. Other 1.14.x builds renumber opcodes, so a build
//! table will be needed once more than one build is targeted. Only the handshake opcodes are
//! listed; the full set is a separate reconstruction (milestone C4+).

/// Server → client: the auth challenge (DoS challenge + server challenge).
pub const SMSG_AUTH_CHALLENGE: u16 = 0x3048;
/// Client → server: the auth session (realm-join ticket + digest proving the session key).
pub const CMSG_AUTH_SESSION: u16 = 0x3765;
/// Server → client: switch to AES-GCM encrypted mode (RSA-signed key).
pub const SMSG_ENTER_ENCRYPTED_MODE: u16 = 0x3049;
/// Client → server: acknowledgement that encrypted mode is active.
pub const CMSG_ENTER_ENCRYPTED_MODE_ACK: u16 = 0x3767;
/// Server → client: the auth response (realm/character enablement). Built in a later milestone.
pub const SMSG_AUTH_RESPONSE: u16 = 0x256D;
/// Server → client: redirect to an instance socket. Built in a later milestone.
pub const SMSG_CONNECT_TO: u16 = 0x304D;
/// Client → server: the continued-session handshake on an instance socket.
pub const CMSG_AUTH_CONTINUED_SESSION: u16 = 0x3766;
/// Client → server: latency ping.
pub const CMSG_PING: u16 = 0x3768;
/// Server → client: pong.
pub const SMSG_PONG: u16 = 0x304E;
