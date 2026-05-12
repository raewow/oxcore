//! Movement validation - anti-cheat checks

use anyhow::Result;

use crate::shared::protocol::{ObjectGuid, Position};
use crate::world::core::common::MovementInfo;

/// Validate movement for anti-cheat
pub fn validate_movement(
    player_guid: ObjectGuid,
    movement_info: &MovementInfo,
    old_pos: &Position,
) -> Result<()> {
    // Mover must be self (no mind control yet)
    // Compare counter parts only, since client receives GUID as low-only in UPDATE_OBJECT
    if movement_info.mover_guid.counter() != player_guid.counter() {
        anyhow::bail!("Movement for non-self GUID");
    }

    let new_pos = &movement_info.position;

    // Teleport distance check (50 yards)
    let distance = old_pos.distance_to(new_pos);
    if distance > 50.0 {
        // Large movement - could be legitimate teleport
        // TODO: Implement proper teleport detection
    }

    // Map bounds check
    if new_pos.x.abs() > 20000.0 || new_pos.y.abs() > 20000.0 || new_pos.z.abs() > 20000.0 {
        anyhow::bail!("Position out of map bounds: {:?}", new_pos);
    }

    Ok(())
}
