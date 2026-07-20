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
pub use position::{is_valid_map_coord, normalize_map_coord, Position};

// V2 internal types (not shared)
pub use guid::ObjectGuid;

use std::sync::OnceLock;
use std::time::Instant;

/// Monotonic server clock millisecond counter, mirroring `WorldTimer::getMSTime()`.
///
/// Returns milliseconds since the first call (process start), wrapping at u32 like
/// the C++ implementation. Used for movement reject windows and other ms-grained
/// server-side timers.
pub fn get_ms_time() -> u32 {
    static START: OnceLock<Instant> = OnceLock::new();
    let start = START.get_or_init(Instant::now);
    start.elapsed().as_millis() as u32
}
