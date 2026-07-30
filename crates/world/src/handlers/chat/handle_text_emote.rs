//! CMSG_TEXT_EMOTE handler - player performs a text emote

use anyhow::{anyhow, Result};

use crate::core::common::packet::WorldPacketGuidExt;
use crate::core::session::WorldSession;
use crate::World;
use oxcore_shared::messages::chat::SmsgTextEmote;
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::bitbuf::BitReader;
use oxcore_shared::protocol::{ObjectGuid, Protocol, WorldPacket};

fn read_text_emote(protocol: Protocol, packet: &mut WorldPacket) -> Result<(u32, u32, ObjectGuid)> {
    if protocol == Protocol::Modern {
        let mut reader = BitReader::new(packet.contents());
        let (high, low) = reader
            .read_packed_guid_128()
            .ok_or_else(|| anyhow!("Failed to read modern target GUID from CMSG_TEXT_EMOTE"))?;
        let text_emote = reader
            .read_u32()
            .ok_or_else(|| anyhow!("Failed to read text_emote from CMSG_TEXT_EMOTE"))?;
        let emote_num = reader
            .read_u32()
            .ok_or_else(|| anyhow!("Failed to read emote_num from CMSG_TEXT_EMOTE"))?;
        Ok((text_emote, emote_num, ObjectGuid::from_guid128(high, low)))
    } else {
        let text_emote = packet
            .read_u32()
            .ok_or_else(|| anyhow!("Failed to read text_emote from CMSG_TEXT_EMOTE"))?;
        let emote_num = packet
            .read_u32()
            .ok_or_else(|| anyhow!("Failed to read emote_num from CMSG_TEXT_EMOTE"))?;
        let target_guid = packet
            .read_packed_guid()
            .ok_or_else(|| anyhow!("Failed to read target GUID from CMSG_TEXT_EMOTE"))?;
        Ok((text_emote, emote_num, target_guid))
    }
}

/// Handle CMSG_TEXT_EMOTE - player performs a text emote
pub async fn handle_text_emote(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let (text_emote, emote_num, target_guid) = read_text_emote(session.protocol(), packet)?;

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
        .broadcast_msg_nearby(player_guid, &text_emote_msg, true);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxcore_shared::protocol::bitbuf::BitWriter;
    use oxcore_shared::protocol::Opcode;

    #[test]
    fn modern_text_emote_starts_with_packed_target_guid() {
        let target = ObjectGuid::new_creature(299, 464);
        let (high, low) = target.to_guid128(1);
        let mut writer = BitWriter::new();
        writer.write_packed_guid_128(high, low);
        writer.write_u32(42);
        writer.write_u32(7);
        let mut packet = WorldPacket::new(Opcode::CMSG_TEXT_EMOTE);
        packet.write_bytes(&writer.into_bytes());

        assert_eq!(
            read_text_emote(Protocol::Modern, &mut packet).unwrap(),
            (42, 7, target)
        );
    }
}
