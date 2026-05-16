//! Player packet handlers
//!
//! Handlers for player-specific opcodes like SET_SELECTION.

use anyhow::Result;
use tracing::debug;

use crate::shared::protocol::WorldPacket;
use crate::world::core::session::WorldSession;
use crate::world::World;

/// Handle CMSG_SET_SELECTION (0x13D / 317)
///
/// Sent when player clicks/targets a unit, object, or NPC.
/// Packet format: target_guid (u64, unpacked)
pub async fn handle_set_selection(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    // Read target GUID (8 bytes, unpacked)
    let target_guid = packet
        .read_u64()
        .ok_or_else(|| anyhow::anyhow!("Failed to read target GUID"))?;

    let target = crate::shared::protocol::ObjectGuid::from(target_guid);

    debug!(
        "CMSG_SET_SELECTION: player={:?}, target={:?}",
        player_guid, target
    );

    // Update player's selection
    if target.is_empty() {
        world.systems.player.manager().clear_selection(player_guid);
    } else {
        world
            .systems
            .player
            .manager()
            .set_selection(player_guid, target);
    }

    Ok(())
}
