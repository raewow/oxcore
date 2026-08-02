//! CMSG_LEAVE_CHANNEL handler

use anyhow::{anyhow, Result};

use crate::core::session::WorldSession;
use crate::World;
use oxcore_shared::game::chat::Team;
use oxcore_shared::protocol::bitbuf::BitReader;
use oxcore_shared::protocol::{Protocol, WorldPacket};

fn read_leave_channel(protocol: Protocol, packet: &mut WorldPacket) -> Result<String> {
    if protocol == Protocol::Modern {
        let mut reader = BitReader::new(packet.contents());
        reader
            .read_u32()
            .ok_or_else(|| anyhow!("Missing channel ID in CMSG_LEAVE_CHANNEL"))?;
        let channel_len = reader
            .read_bits(7)
            .ok_or_else(|| anyhow!("Missing channel name length in CMSG_LEAVE_CHANNEL"))?
            as usize;
        reader
            .read_string(channel_len)
            .ok_or_else(|| anyhow!("Invalid channel name in CMSG_LEAVE_CHANNEL"))
    } else {
        packet
            .read_u32()
            .ok_or_else(|| anyhow!("Missing channel ID in CMSG_LEAVE_CHANNEL"))?;
        Ok(packet.read_string().unwrap_or_default())
    }
}

/// Handle CMSG_LEAVE_CHANNEL - player leaves a chat channel
pub async fn handle_leave_channel(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let player = world
        .managers
        .player_mgr
        .get_player(player_guid)
        .ok_or_else(|| anyhow!("Player not found"))?;
    let player_team = Team::from_race(player.race);
    drop(player);

    let channel_name = read_leave_channel(session.protocol(), packet)?;

    world
        .systems
        .chat
        .leave_channel(player_guid, &channel_name, player_team)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxcore_shared::protocol::bitbuf::BitWriter;
    use oxcore_shared::protocol::Opcode;

    #[test]
    fn parses_modern_leave_channel() {
        let mut writer = BitWriter::new();
        writer.write_u32(1);
        writer.write_bits(7, 7);
        writer.write_string_raw("General");
        let mut packet = WorldPacket::new(Opcode::CMSG_LEAVE_CHANNEL);
        packet.write_bytes(&writer.into_bytes());

        assert_eq!(
            read_leave_channel(Protocol::Modern, &mut packet).unwrap(),
            "General"
        );
    }
}
