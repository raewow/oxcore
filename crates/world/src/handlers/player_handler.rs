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
}
