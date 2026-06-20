//! CMSG_EMOTE handler - player performs an animated emote

use anyhow::{Result, anyhow};

use crate::World;
use crate::core::session::WorldSession;
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::messages::chat::SmsgEmote;
use oxcore_shared::protocol::WorldPacket;

/// Handle CMSG_EMOTE - player performs an emote
pub async fn handle_emote(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    // Read emote ID from packet
    let emote_id = packet
        .read_u32()
        .ok_or_else(|| anyhow!("Failed to read emote ID from CMSG_EMOTE"))?;

    // Basic validation - emote IDs in vanilla are typically 0-500
    if emote_id > 1000 {
        tracing::warn!("CMSG_EMOTE: Invalid emote ID {}", emote_id);
        return Ok(());
    }

    // Build SMSG_EMOTE packet using message struct
    let emote_msg = SmsgEmote {
        emote_id,
        guid: player_guid,
    };

    // Broadcast to nearby players (including self)
    world
        .managers
        .broadcast_mgr
        .broadcast_nearby(player_guid, &emote_msg.to_world_packet(), true);

    Ok(())
}
