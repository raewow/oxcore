//! Vendor packet handlers
//!
//! Slim handlers that parse packets and delegate to VendorSystem.

use anyhow::Result;
use tracing::debug;

use crate::shared::protocol::WorldPacket;
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::core::session::WorldSession;
use crate::world::World;

/// Handle CMSG_LIST_INVENTORY (0x19E)
///
/// Sent when player opens a vendor window.
/// Packet format: GUID (packed)
pub async fn handle_list_inventory(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let vendor_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read vendor GUID"))?;

    debug!(
        "CMSG_LIST_INVENTORY: player={:?}, vendor={:?}",
        player_guid, vendor_guid
    );

    // Delegate to vendor system
    world
        .systems
        .vendor
        .send_vendor_list(player_guid, vendor_guid)
        .await?;

    Ok(())
}

/// Handle CMSG_BUY_ITEM (0x1A3)
///
/// Sent when player buys an item from a vendor.
/// Packet format: GUID (unpacked), item_id (u32), count (u8)
pub async fn handle_buy_item(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let vendor_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read vendor GUID"))?;

    let item_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read item ID"))?;

    let count = packet
        .read_u8()
        .ok_or_else(|| anyhow::anyhow!("Failed to read count"))?;

    debug!(
        "CMSG_BUY_ITEM: player={:?}, vendor={:?}, item={}, count={}",
        player_guid, vendor_guid, item_id, count
    );

    // Delegate to vendor system
    world
        .systems
        .vendor
        .handle_buy_item(player_guid, vendor_guid, item_id, count)
        .await?;

    Ok(())
}

/// Handle CMSG_SELL_ITEM (0x1A7)
///
/// Sent when player sells an item to a vendor.
/// Packet format: vendor_guid (u64), item_guid (u64), amount (u8)
pub async fn handle_sell_item(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let vendor_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read vendor GUID"))?;

    let item_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read item GUID"))?;

    let _amount = packet.read_u8().unwrap_or(0);

    debug!(
        "CMSG_SELL_ITEM: player={:?}, vendor={:?}, item={:?}",
        player_guid, vendor_guid, item_guid
    );

    // Delegate to vendor system
    world
        .systems
        .vendor
        .handle_sell_item(player_guid, vendor_guid, item_guid)
        .await?;

    Ok(())
}
