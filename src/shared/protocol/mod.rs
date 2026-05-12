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
