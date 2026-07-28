//! Protocol module - shared packet opcodes, packets, and update fields
//!
//! This module contains protocol definitions shared between world and world.

pub mod guid;
pub mod movement;
pub mod opcodes;
pub mod packet;
pub mod position;
pub mod update_fields;
pub mod updates;

pub use guid::{HighGuid, ObjectGuid, ObjectGuidGenerator};
pub use movement::{MoveFlags, MovementInfo};
pub use opcodes::Opcode;
pub use packet::{WorldPacket, WorldPacketGuidExt};
pub use position::Position;

/// Which client protocol a connection speaks.
///
/// Two clients can be logged in at once speaking different protocols, so this is per-connection
/// state, never a global. It decides which of a message's encodings goes on the wire and which
/// number is read off its [`Opcode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Protocol {
    /// Vanilla 1.12.x — the original realmd/world protocol.
    #[default]
    Vanilla,
    /// Modern 1.14.x — Battle.net login, AES-GCM framing, bit-packed bodies.
    Modern,
}
