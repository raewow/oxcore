//! CMSG_JOIN_CHANNEL handler

use anyhow::{anyhow, Result};

use crate::core::session::WorldSession;
use crate::World;
use oxcore_shared::game::chat::Team;
use oxcore_shared::protocol::bitbuf::BitReader;
use oxcore_shared::protocol::{Protocol, WorldPacket};

fn read_join_channel(
    protocol: Protocol,
    packet: &mut WorldPacket,
) -> Result<(u32, String, String)> {
    if protocol == Protocol::Modern {
        let mut reader = BitReader::new(packet.contents());
        let channel_id = reader
            .read_u32()
            .ok_or_else(|| anyhow!("Missing channel ID in CMSG_JOIN_CHANNEL"))?;
        let channel_len = reader
            .read_bits(7)
            .ok_or_else(|| anyhow!("Missing channel name length in CMSG_JOIN_CHANNEL"))?
            as usize;
        let password_len = reader
            .read_bits(7)
            .ok_or_else(|| anyhow!("Missing password length in CMSG_JOIN_CHANNEL"))?
            as usize;
        let remaining = &packet.contents()[reader.consumed()..];
        if channel_len + password_len > remaining.len() {
            // The 1.14.2 client uses a zone-channel form here: two undocumented flag bytes
            // precede the raw channel name instead of the documented pair of 7-bit lengths.
            let channel = std::str::from_utf8(remaining)
                .map_err(|_| anyhow!("Invalid zone channel name in CMSG_JOIN_CHANNEL"))?;
            return Ok((channel_id, channel.to_owned(), String::new()));
        }
        let channel = reader
            .read_string(channel_len)
            .ok_or_else(|| anyhow!("Invalid channel name in CMSG_JOIN_CHANNEL"))?;
        let password = reader
            .read_string(password_len)
            .ok_or_else(|| anyhow!("Invalid password in CMSG_JOIN_CHANNEL"))?;
        Ok((channel_id, channel, password))
    } else {
        let channel_id = packet
            .read_u32()
            .ok_or_else(|| anyhow!("Missing channel ID in CMSG_JOIN_CHANNEL"))?;
        Ok((
            channel_id,
            packet.read_string().unwrap_or_default(),
            packet.read_string().unwrap_or_default(),
        ))
    }
}

/// Handle CMSG_JOIN_CHANNEL - player joins a chat channel
pub async fn handle_join_channel(
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

    let (channel_id, channel_name, password) = read_join_channel(session.protocol(), packet)?;
    let password_opt = if password.is_empty() {
        None
    } else {
        Some(password.as_str())
    };

    // System handles everything including error notifications
    world
        .systems
        .chat
        .join_channel_with_id(
            player_guid,
            &channel_name,
            password_opt,
            player_team,
            (session.protocol() == Protocol::Modern).then_some(channel_id),
        )
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxcore_shared::protocol::bitbuf::BitWriter;
    use oxcore_shared::protocol::Opcode;

    #[test]
    fn parses_modern_join_channel() {
        let mut writer = BitWriter::new();
        writer.write_u32(1);
        writer.write_bits(7, 7);
        writer.write_bits(2, 7);
        writer.write_string_raw("General");
        writer.write_string_raw("pw");
        let mut packet = WorldPacket::new(Opcode::CMSG_JOIN_CHANNEL);
        packet.write_bytes(&writer.into_bytes());

        assert_eq!(
            read_join_channel(Protocol::Modern, &mut packet).unwrap(),
            (1, "General".to_string(), "pw".to_string())
        );
    }

    #[test]
    fn parses_modern_zone_channel_form() {
        let mut packet = WorldPacket::new(Opcode::CMSG_JOIN_CHANNEL);
        packet.write_bytes(&[
            2, 0, 0, 0, 0x8A, 0x80, b'T', b'r', b'a', b'd', b'e', b' ', b'-', b' ', b'E', b'l',
            b'w', b'y', b'n', b'n', b' ', b'F', b'o', b'r', b'e', b's', b't',
        ]);

        assert_eq!(
            read_join_channel(Protocol::Modern, &mut packet).unwrap(),
            (2, "Trade - Elwynn Forest".to_string(), String::new())
        );
    }
}
