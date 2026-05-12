//! Common types shared across the world server

pub mod guid;
pub mod movement;
pub mod packet;
pub mod packet_compression;
pub mod position;
pub mod unit;

pub use guid::{HighGuid, ObjectGuidGenerator};
pub use movement::{MoveFlags, MovementInfo};
pub use packet_compression::{compress_update_object, compress_update_packet_if_needed};
pub use position::Position;

// V2 internal types (not shared)
pub use guid::ObjectGuid;
