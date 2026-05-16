use crate::shared::protocol::ObjectGuid;
use crate::shared::protocol::WorldPacket;
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::core::session::WorldSession;
use crate::world::World;

/// Handle CMSG_LOOT (0x015D)
pub async fn handle_loot(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> anyhow::Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Player not logged in"))?;
    let target_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read target GUID"))?;

    // Delegate everything to LootSystem
    world
        .systems
        .loot
        .handle_loot_request(player_guid, target_guid, world)
        .await
}

/// Handle CMSG_LOOT_MONEY (0x015E)
pub async fn handle_loot_money(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> anyhow::Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Player not logged in"))?;

    // Get what the player is looting
    let target_guid = world
        .managers
        .player_mgr
        .get_looting_target(player_guid)
        .ok_or_else(|| anyhow::anyhow!("Not looting anything"))?;

    // Delegate to LootSystem
    world
        .systems
        .loot
        .handle_loot_money(player_guid, target_guid, world)
        .await
}

/// Handle CMSG_AUTOSTORE_LOOT_ITEM (0x0108)
pub async fn handle_loot_item(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> anyhow::Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Player not logged in"))?;
    let slot = packet
        .read_u8()
        .ok_or_else(|| anyhow::anyhow!("Failed to read slot"))?;

    // Get what the player is looting
    let target_guid = world
        .managers
        .player_mgr
        .get_looting_target(player_guid)
        .ok_or_else(|| anyhow::anyhow!("Not looting anything"))?;

    // Delegate to LootSystem
    world
        .systems
        .loot
        .handle_loot_item(player_guid, target_guid, slot, world)
        .await
}

/// Handle CMSG_LOOT_RELEASE (0x015F)
pub async fn handle_loot_release(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> anyhow::Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Player not logged in"))?;
    let target_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read target GUID"))?;

    // Delegate to LootSystem
    world
        .systems
        .loot
        .handle_loot_release(player_guid, target_guid, world)
        .await
}
