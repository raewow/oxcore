//! Modern (1.14.x) world protocol — a parallel network stack to the vanilla one.
//!
//! A patched 1.14 client that has completed bnet realm-join connects here (not TLS). It speaks a
//! different protocol family from the 1.12 client: a plaintext connection-init exchange, a 16-byte
//! `[u32 size][12-byte GCM tag]` header, and AES-128-GCM packet encryption over `opcode ‖ body`.
//! See `crates/world/docs/part-c-modern-world-handshake.md` for the full design.
//!
//! **C1 (this module):** the transport primitives — [`crypt::WorldCrypt`] (AES-GCM) and
//! [`framing`] (the header codec + connection-init strings). Auth, key derivation, and opcode
//! handling follow in later milestones. Nothing here is wired into the accept loop yet.

pub mod crypt;
pub mod framing;

pub use crypt::WorldCrypt;
pub use framing::{
    decode, encode, ModernPacket, CONNECTION_INITIALIZE_CLIENT, CONNECTION_INITIALIZE_SERVER,
    HEADER_SIZE,
};
