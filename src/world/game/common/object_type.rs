// Re-export from old world modules so types match shared::messages::update expectations
pub use crate::world::core::common::unit::{TYPEMASK_ITEM, TYPEMASK_OBJECT};
pub use oxcore_shared::protocol::updates::movement_block::MovementSpeeds;
pub use oxcore_shared::protocol::updates::update_block_builder::update_flags;
pub use oxcore_shared::protocol::updates::update_types::ObjectTypeId;
