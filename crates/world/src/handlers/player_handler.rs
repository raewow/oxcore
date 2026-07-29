//! Player packet handlers
//!
//! Handlers for player-specific opcodes like SET_SELECTION.

use anyhow::Result;
use tracing::debug;

use crate::core::session::WorldSession;
use crate::World;
use oxcore_shared::protocol::bitbuf::BitReader;
use oxcore_shared::protocol::{ObjectGuid, Protocol, WorldPacket};

fn read_selection_target(protocol: Protocol, packet: &mut WorldPacket) -> Result<ObjectGuid> {
    if protocol == Protocol::Modern {
        let mut reader = BitReader::new(packet.contents());
        let (_, low) = reader
            .read_packed_guid_128()
            .ok_or_else(|| anyhow::anyhow!("Failed to read modern target GUID"))?;
        Ok(ObjectGuid::from_raw(low))
    } else {
        packet
            .read_u64()
            .map(ObjectGuid::from_raw)
            .ok_or_else(|| anyhow::anyhow!("Failed to read target GUID"))
    }
}

fn read_stand_state(packet: &mut WorldPacket) -> Result<Option<u8>> {
    let state = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read stand state"))?;
    Ok(match state {
        0 | 1 | 3 | 8 => Some(state as u8),
        _ => None,
    })
}

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

    let target = read_selection_target(session.protocol(), packet)?;

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

/// Handle CMSG_STANDSTATECHANGE.
///
/// The modern and vanilla request bodies both carry the requested state as a u32.
pub async fn handle_stand_state_change(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
    let Some(state) = read_stand_state(packet)? else {
        return Ok(());
    };

    world
        .systems
        .player
        .manager()
        .with_player_mut(player_guid, |player| player.stand_state = state);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxcore_shared::protocol::bitbuf::BitWriter;
    use oxcore_shared::protocol::Opcode;

    #[test]
    fn modern_selection_reads_packed_guid128() {
        let mut writer = BitWriter::new();
        writer.write_packed_guid_128(0x0100, 0x0200_01);
        let mut packet = WorldPacket::new(Opcode::CMSG_SET_SELECTION);
        packet.write_bytes(&writer.into_bytes());

        assert_eq!(
            read_selection_target(Protocol::Modern, &mut packet)
                .unwrap()
                .raw(),
            0x0200_01
        );
    }

    #[test]
    fn stand_state_accepts_only_client_selectable_states() {
        let mut sit = WorldPacket::new(Opcode::CMSG_STANDSTATECHANGE);
        sit.write_u32(1);
        assert_eq!(read_stand_state(&mut sit).unwrap(), Some(1));

        let mut dead = WorldPacket::new(Opcode::CMSG_STANDSTATECHANGE);
        dead.write_u32(7);
        assert_eq!(read_stand_state(&mut dead).unwrap(), None);
    }
}
