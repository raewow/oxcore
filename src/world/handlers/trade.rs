//! Trade packet handlers - thin handlers that parse packets and delegate to TradeSystem

use anyhow::Result;
use tracing::{debug, warn};

use crate::shared::protocol::WorldPacket;
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::core::session::WorldSession;
use crate::world::game::trade::TradeStatus;
use crate::world::World;

/// CMSG_INITIATE_TRADE handler - start a trade with another player
pub async fn handle_initiate_trade(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_INITIATE_TRADE: Not logged in");
            return Ok(());
        }
    };

    let target_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read target GUID"))?;

    debug!(
        "CMSG_INITIATE_TRADE: player={:?}, target={:?}",
        player_guid, target_guid
    );

    if let Err(e) = world
        .systems
        .trade
        .initiate_trade(player_guid, target_guid)
        .await
    {
        debug!("[TRADE] Initiate failed: {:?}", e);
        world.systems.trade.send_trade_error(player_guid, e);
    }

    Ok(())
}

/// CMSG_BEGIN_TRADE handler - accept a trade request and open window
pub async fn handle_begin_trade(session: &WorldSession, world: &World) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_BEGIN_TRADE: Not logged in");
            return Ok(());
        }
    };

    debug!("CMSG_BEGIN_TRADE: player={:?}", player_guid);

    if let Err(e) = world.systems.trade.begin_trade(player_guid).await {
        debug!("[TRADE] Begin failed: {:?}", e);
        world.systems.trade.send_trade_error(player_guid, e);
    }

    Ok(())
}

/// CMSG_SET_TRADE_ITEM handler - add an item to the trade window
pub async fn handle_set_trade_item(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_SET_TRADE_ITEM: Not logged in");
            return Ok(());
        }
    };

    let trade_slot = packet.read_u8().unwrap_or(0);
    let bag = packet.read_u8().unwrap_or(0);
    let slot = packet.read_u8().unwrap_or(0);

    debug!(
        "CMSG_SET_TRADE_ITEM: player={:?}, trade_slot={}, bag={}, slot={}",
        player_guid, trade_slot, bag, slot
    );

    if let Err(e) = world
        .systems
        .trade
        .set_trade_item(player_guid, trade_slot, bag, slot)
        .await
    {
        debug!("[TRADE] Set item failed: {:?}", e);
        world.systems.trade.send_trade_error(player_guid, e);
    }

    Ok(())
}

/// CMSG_CLEAR_TRADE_ITEM handler - remove an item from the trade window
pub async fn handle_clear_trade_item(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_CLEAR_TRADE_ITEM: Not logged in");
            return Ok(());
        }
    };

    let trade_slot = packet.read_u8().unwrap_or(0);

    debug!(
        "CMSG_CLEAR_TRADE_ITEM: player={:?}, trade_slot={}",
        player_guid, trade_slot
    );

    if let Err(e) = world
        .systems
        .trade
        .clear_trade_item(player_guid, trade_slot)
        .await
    {
        debug!("[TRADE] Clear item failed: {:?}", e);
        world.systems.trade.send_trade_error(player_guid, e);
    }

    Ok(())
}

/// CMSG_SET_TRADE_GOLD handler - set gold amount in trade
pub async fn handle_set_trade_gold(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_SET_TRADE_GOLD: Not logged in");
            return Ok(());
        }
    };

    let gold = packet.read_u32().unwrap_or(0);

    debug!(
        "CMSG_SET_TRADE_GOLD: player={:?}, gold={}",
        player_guid, gold
    );

    if let Err(e) = world.systems.trade.set_trade_gold(player_guid, gold).await {
        debug!("[TRADE] Set gold failed: {:?}", e);
        world.systems.trade.send_trade_error(player_guid, e);
    }

    Ok(())
}

/// CMSG_ACCEPT_TRADE handler - accept the current trade
pub async fn handle_accept_trade(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_ACCEPT_TRADE: Not logged in");
            return Ok(());
        }
    };

    // Unknown field - always 1 in client
    let _unknown = packet.read_u32().unwrap_or(1);

    debug!("CMSG_ACCEPT_TRADE: player={:?}", player_guid);

    if let Err(e) = world.systems.trade.accept_trade(player_guid).await {
        debug!("[TRADE] Accept failed: {:?}", e);
        world.systems.trade.send_trade_error(player_guid, e);
    }

    Ok(())
}

/// CMSG_UNACCEPT_TRADE handler - revoke acceptance
pub async fn handle_unaccept_trade(session: &WorldSession, world: &World) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_UNACCEPT_TRADE: Not logged in");
            return Ok(());
        }
    };

    debug!("CMSG_UNACCEPT_TRADE: player={:?}", player_guid);

    if let Err(e) = world.systems.trade.unaccept_trade(player_guid).await {
        debug!("[TRADE] Unaccept failed: {:?}", e);
        world.systems.trade.send_trade_error(player_guid, e);
    }

    Ok(())
}

/// CMSG_CANCEL_TRADE handler - cancel the trade
pub async fn handle_cancel_trade(session: &WorldSession, world: &World) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_CANCEL_TRADE: Not logged in");
            return Ok(());
        }
    };

    debug!("CMSG_CANCEL_TRADE: player={:?}", player_guid);

    if let Err(e) = world
        .systems
        .trade
        .cancel_trade(player_guid, TradeStatus::TradeCanceled)
        .await
    {
        debug!("[TRADE] Cancel failed: {:?}", e);
    }

    Ok(())
}

/// CMSG_BUSY_TRADE handler - decline trade (busy)
pub async fn handle_busy_trade(session: &WorldSession, world: &World) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_BUSY_TRADE: Not logged in");
            return Ok(());
        }
    };

    debug!("CMSG_BUSY_TRADE: player={:?}", player_guid);

    if let Err(e) = world
        .systems
        .trade
        .cancel_trade(player_guid, TradeStatus::Busy)
        .await
    {
        debug!("[TRADE] Busy trade failed: {:?}", e);
    }

    Ok(())
}

/// CMSG_IGNORE_TRADE handler - decline trade (ignore)
pub async fn handle_ignore_trade(session: &WorldSession, world: &World) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_IGNORE_TRADE: Not logged in");
            return Ok(());
        }
    };

    debug!("CMSG_IGNORE_TRADE: player={:?}", player_guid);

    if let Err(e) = world
        .systems
        .trade
        .cancel_trade(player_guid, TradeStatus::IgnoreYou)
        .await
    {
        debug!("[TRADE] Ignore trade failed: {:?}", e);
    }

    Ok(())
}
