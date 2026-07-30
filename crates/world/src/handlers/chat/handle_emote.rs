//! CMSG_EMOTE handler - player performs an animated emote

use anyhow::{anyhow, Result};

use crate::core::session::WorldSession;
use crate::World;
use oxcore_shared::messages::chat::SmsgEmote;
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::{Protocol, WorldPacket};

fn read_emote_id(protocol: Protocol, packet: &mut WorldPacket) -> Result<Option<u32>> {
    if protocol == Protocol::Modern {
        // Modern CMSG_EMOTE has an empty body.
        return Ok(None);
    }

    packet
        .read_u32()
        .map(Some)
        .ok_or_else(|| anyhow!("Failed to read emote ID from CMSG_EMOTE"))
}

/// Handle CMSG_EMOTE - player performs an emote
pub async fn handle_emote(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let Some(emote_id) = read_emote_id(session.protocol(), packet)? else {
        return Ok(());
    };

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
        .broadcast_msg_nearby(player_guid, &emote_msg, true);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxcore_shared::protocol::Opcode;

    #[test]
    fn modern_emote_has_no_body() {
        let mut packet = WorldPacket::new(Opcode::CMSG_EMOTE);
        assert_eq!(read_emote_id(Protocol::Modern, &mut packet).unwrap(), None);
    }

    #[test]
    fn vanilla_emote_reads_id() {
        let mut packet = WorldPacket::new(Opcode::CMSG_EMOTE);
        packet.write_u32(17);
        assert_eq!(
            read_emote_id(Protocol::Vanilla, &mut packet).unwrap(),
            Some(17)
        );
    }
}
