//! CMSG_TEXT_EMOTE handler - player performs a text emote

use anyhow::{anyhow, Result};

use crate::shared::messages::chat::SmsgTextEmote;
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::WorldPacket;
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::core::session::WorldSession;
use crate::world::World;

/// Handle CMSG_TEXT_EMOTE - player performs a text emote
pub async fn handle_text_emote(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    // Read packet data: textEmote (u32), emoteNum (u32), targetGUID (packed)
    let text_emote = packet
        .read_u32()
        .ok_or_else(|| anyhow!("Failed to read text_emote from CMSG_TEXT_EMOTE"))?;

    let emote_num = packet
        .read_u32()
        .ok_or_else(|| anyhow!("Failed to read emote_num from CMSG_TEXT_EMOTE"))?;

    let target_guid = packet
        .read_packed_guid()
        .ok_or_else(|| anyhow!("Failed to read target GUID from CMSG_TEXT_EMOTE"))?;

    // Basic validation
    if text_emote > 1000 {
        tracing::warn!("CMSG_TEXT_EMOTE: Invalid text_emote ID {}", text_emote);
        return Ok(());
    }

    // Look up target name if target provided
    let target_name = if !target_guid.is_empty() {
        world.managers.player_mgr.get_player_name(target_guid)
    } else {
        None
    };

    // Build SMSG_TEXT_EMOTE packet using message struct
    let text_emote_msg = SmsgTextEmote {
        guid: player_guid,
        text_emote,
        emote_num,
        target_name: target_name.as_deref(),
    };

    // Broadcast to nearby players (including self)
    world
        .managers
        .broadcast_mgr
        .broadcast_nearby(player_guid, &text_emote_msg.to_world_packet(), true)
        ;

    Ok(())
}
