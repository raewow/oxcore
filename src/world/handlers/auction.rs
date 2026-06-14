//! Auction packet handlers
//!
//! Handles CMSG_AUCTION_SELL_ITEM and other auction-related client packets.

use anyhow::Result;
use tracing::{debug, warn};

use crate::shared::game::auction::{AuctionAction, AuctionError, AuctionEntry};
use crate::shared::messages::auction::SmsgAuctionCommandResult;
use crate::shared::protocol::{Opcode, WorldPacket};
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::core::session::WorldSession;
use crate::world::game::auction::{
    get_checked_auction_house_for_auctioneer, send_auction_command_result,
};
use crate::world::World;

/// Hard cap on bid/buyout to prevent gold dupe exploits.
const MAX_AUCTION_PRICE: u32 = 2_000_000_000;

/// Valid auction durations in seconds (matching C++ MIN_AUCTION_TIME = 2h).
const VALID_AUCTION_DURATIONS: [u32; 3] = [7200, 28800, 86400]; // 2h, 8h, 24h

/// Handle CMSG_AUCTION_SELL_ITEM (0x0256)
///
/// Packet format (vanilla 1.12.1):
/// - auctioneerGuid (packed u64)
/// - itemGuid     (packed u64)
/// - bid          (u32)
/// - buyout       (u32)
/// - etime        (u32)  -- minutes
///
/// Mirrors C++ `WorldSession::HandleAuctionSellItem`.
/// Many item/inventory validations are TODO stubs because the inventory system
/// is not fully ported.
pub async fn handle_auction_sell_item(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let auctioneer_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read auctioneer GUID"))?;
    let item_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read item GUID"))?;
    let bid = packet.read_u32().unwrap_or(0);
    let buyout = packet.read_u32().unwrap_or(0);
    let etime_minutes = packet.read_u32().unwrap_or(0);

    debug!(
        "CMSG_AUCTION_SELL_ITEM: player={:?} auctioneer={:?} item={:?} bid={} buyout={} etime={}min",
        player_guid, auctioneer_guid, item_guid, bid, buyout, etime_minutes
    );

    // --- validation: bid and etime must be non-zero ---
    if bid == 0 || etime_minutes == 0 {
        debug!("Auction sell rejected: bid or etime is zero (cheater check)");
        return Ok(());
    }

    // --- validation: price cap ---
    if bid > MAX_AUCTION_PRICE || buyout > MAX_AUCTION_PRICE {
        send_auction_command_result(
            session,
            None,
            AuctionAction::Started,
            AuctionError::NotEnoughMoney,
            None,
        )?;
        // TODO: ProcessAnticheatAction("GoldDupe", "Putting too high auction price", CHEAT_ACTION_LOG)
        return Ok(());
    }

    // --- validation: bid > buyout ---
    if buyout != 0 && bid > buyout {
        send_auction_command_result(
            session,
            None,
            AuctionAction::Started,
            AuctionError::HigherBid,
            None,
        )?;
        // TODO: ProcessAnticheatAction("GoldDupe", "bid > buyout", CHEAT_ACTION_LOG)
        return Ok(());
    }

    // --- player lookup ---
    let player = world
        .managers
        .player_mgr
        .get_player(player_guid)
        .ok_or_else(|| anyhow::anyhow!("Player not found"))?;

    // --- security / GM checks ---
    let gm_allow_trades = world.config.gm_allow_trades.unwrap_or(false);
    if !gm_allow_trades && session.security() > 0 {
        // SEC_PLAYER = 0; anything higher is GM
        send_auction_command_result(
            session,
            None,
            AuctionAction::Started,
            AuctionError::RestrictedAccount,
            None,
        )?;
        return Ok(());
    }

    // TODO: HasTrialRestrictions() check
    // TODO: CONFIG_UINT32_ACCOUNT_CONCURRENT_AUCTION_LIMIT check

    // --- auctioneer validation ---
    let auction_house = get_checked_auction_house_for_auctioneer(
        &player,
        auctioneer_guid,
        &world.managers.auction_mgr,
        None, // NPC interaction not yet ported
    );

    let auction_house = match auction_house {
        Some(h) => h,
        None => {
            send_auction_command_result(
                session,
                None,
                AuctionAction::Started,
                AuctionError::DatabaseError,
                None,
            )?;
            return Ok(());
        }
    };

    // --- duration validation ---
    let etime_secs = etime_minutes * 60;
    if !VALID_AUCTION_DURATIONS.contains(&etime_secs) {
        send_auction_command_result(
            session,
            None,
            AuctionAction::Started,
            AuctionError::DatabaseError,
            None,
        )?;
        return Ok(());
    }

    // --- item validation (many checks are TODO stubs) ---
    // TODO: itemGuid == 0 -> AUCTION_ERR_ITEM_NOT_FOUND
    // TODO: GetAItem(item_guid_low) already in auction -> AUCTION_ERR_INVENTORY
    // TODO: GetItemByGuid(itemGuid) == null -> AUCTION_ERR_INVENTORY
    // TODO: IsBankPos -> AUCTION_ERR_INVENTORY
    // TODO: CanBeTraded -> AUCTION_ERR_INVENTORY
    // TODO: conjured / duration -> AUCTION_ERR_INVENTORY

    // --- deposit calculation ---
    let min_deposit = world.config.auction_deposit_min;
    let deposit_rate = world.config.rate_auction_deposit;
    // TODO: we need the actual Item object to calculate deposit
    // For now, stub with 0 deposit
    let deposit = 0u32;

    // --- money check ---
    if player.money < deposit {
        send_auction_command_result(
            session,
            None,
            AuctionAction::Started,
            AuctionError::NotEnoughMoney,
            None,
        )?;
        return Ok(());
    }

    // TODO: remove feign death if active
    // TODO: GM log trade
    // TODO: deduct deposit
    // TODO: create AuctionEntry
    // TODO: add to auction house
    // TODO: remove item from inventory
    // TODO: persist to DB

    // --- success ---
    // TODO: send success response with actual auction
    send_auction_command_result(
        session,
        None,
        AuctionAction::Started,
        AuctionError::Ok,
        None,
    )?;

    Ok(())
}
