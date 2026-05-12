use crate::shared::protocol::WorldPacket;
use crate::world::core::session::WorldSession;
use crate::world::World;
use anyhow::Result;

/// Handle CMSG_LEARN_TALENT (opcode 0x0251)
///
/// Sent by the client when the player clicks a talent in the talent UI.
/// The client already validates prerequisites locally, so a failure here
/// indicates a modified client or desync.
///
/// Packet format:
///   talent_id: u32 - DBC talent ID
///   requested_rank: u32 - The rank the player wants (0-indexed in packet)
///
/// Note: In vanilla 1.12, the requested_rank in the packet is 0-indexed.
/// The client sends rank=0 for rank 1, rank=1 for rank 2, etc.
/// However, we store ranks as 1-indexed internally (rank=1 means 1 point spent).
///
/// The handler only reads the talent_id; the system determines the correct
/// next rank from the player's current state.
pub async fn handle_learn_talent(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session.player_guid().ok_or_else(|| anyhow::anyhow!("Player not logged in"))?;
    let talent_id = packet.read_u32().ok_or_else(|| anyhow::anyhow!("Failed to read talent_id"))?;
    let _requested_rank = packet.read_u32().ok_or_else(|| anyhow::anyhow!("Failed to read requested_rank"))?; // Client sends 0-indexed rank

    world.systems.talents.learn_talent(player_guid, talent_id, world).await?;

    // Send talent list update to client
    // The client updates automatically from PLAYER_CHARACTER_POINTS1
    // and the spell/aura updates sent by the talent system.
    // No explicit talent list packet is needed in vanilla 1.12.

    Ok(())
}

/// Handle CMSG_UNLEARN_TALENTS (opcode 0x0213)
///
/// Sent by the client when the player resets talents at a class trainer.
/// The trainer interaction has already been validated (proximity, faction, etc.)
/// by the time this packet is received.
///
/// This packet has no body -- the client simply requests a full reset.
pub async fn handle_unlearn_talents(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session.player_guid().ok_or_else(|| anyhow::anyhow!("Player not logged in"))?;

    let no_cost = false; // Normal player reset costs gold
    world.systems.talents.reset_talents(player_guid, no_cost, world).await?;

    Ok(())
}
