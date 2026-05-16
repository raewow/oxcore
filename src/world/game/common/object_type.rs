// Re-export from old world modules so types match shared::messages::update expectations
pub use crate::shared::protocol::updates::movement_block::MovementSpeeds;
pub use crate::shared::protocol::updates::update_block_builder::update_flags;
pub use crate::shared::protocol::updates::update_types::ObjectTypeId;
pub use crate::world::core::common::unit::{TYPEMASK_ITEM, TYPEMASK_OBJECT};
