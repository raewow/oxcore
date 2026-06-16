//! Auction packet handlers
//!
//! Handles CMSG_AUCTION_SELL_ITEM and other auction-related client packets.

use anyhow::Result;
use std::sync::Arc;
use tracing::debug;

use crate::shared::game::auction::{AuctionAction, AuctionEntry, AuctionError, AuctionQueryType};
use crate::shared::protocol::{Opcode, WorldPacket};
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::core::session::WorldSession;
use crate::world::game::auction::manager::{AuctionHouseManager, AuctionHouseObject};
use crate::world::game::auction::{
    get_checked_auction_house_for_auctioneer, send_auction_bidder_notification,
    send_auction_command_result, send_auction_owner_notification,
    send_auction_removed_notification,
};
use crate::world::game::creature::CreatureManager;
use crate::world::game::inventory::inventory_types::{is_bank_pos, InventoryResult};
use crate::world::game::player::auras::effects::AURA_FEIGN_DEATH;
use crate::world::game::player::PlayerManager;
use crate::world::messages::auction::{
    MsgAuctionHello, SmsgAuctionBidderListResult, SmsgAuctionCommandResult, SmsgAuctionListResult,
    SmsgAuctionOwnerListResult,
};
use crate::world::World;

/// Hard cap on bid/buyout to prevent gold dupe exploits.
const MAX_AUCTION_PRICE: u32 = 2_000_000_000;

/// Vanilla auctioneer NPC flag.
///
/// The Rust codebase does not yet have a shared NPC-flag enum, so this local
/// constant keeps the hello handler aligned with the C++ gatekeeper branch.
const NPC_FLAG_AUCTIONEER: u32 = 0x0000_0200;

/// Valid auction durations in seconds (matching C++ MIN_AUCTION_TIME = 2h).
const VALID_AUCTION_DURATIONS: [u32; 3] = [7200, 28800, 86400]; // 2h, 8h, 24h

struct AuctionSearchParams {
    search_name: String,
    level_min: u8,
    level_max: u8,
    auction_slot_id: u32,
    auction_main_category: u32,
    auction_sub_category: u32,
    quality: u32,
    usable: u8,
}

struct AuctionHouseClientQueryTask {
    query_type: AuctionQueryType,
    auction_house: Arc<AuctionHouseObject>,
    account_id: u32,
    listfrom: u32,
    outbidded_auction_ids: Vec<u32>,
    search_params: Option<AuctionSearchParams>,
}

fn read_bid_refresh_ids(packet: &mut WorldPacket) -> Vec<u32> {
    let mut ids = Vec::new();

    while let Some(id) = packet.read_u32() {
        ids.push(id);
    }

    if ids.len() > 1 && ids[0] as usize == ids.len() - 1 {
        ids.remove(0);
    }

    ids
}

async fn execute_auction_list_bidder_items_task(
    world: World,
    player_guid: crate::shared::protocol::ObjectGuid,
    task: AuctionHouseClientQueryTask,
) {
    if task.query_type != AuctionQueryType::ListBidder {
        tracing::warn!("Auction query task invoked with unexpected query type");
        return;
    }

    if !task.outbidded_auction_ids.is_empty() {
        tracing::debug!(
            "Auction bidder list refresh requested for {} auction ids",
            task.outbidded_auction_ids.len()
        );
    }

    tracing::debug!(
        "Executing auction bidder list query for account {}",
        task.account_id
    );

    let Some(session) = world.session_mgr.get_session_by_player(player_guid) else {
        return;
    };

    let mut auctions: Vec<AuctionEntry> = task
        .auction_house
        .auctions_snapshot()
        .into_iter()
        .filter(|auction| auction.bidder_guid == player_guid)
        .collect();
    auctions.sort_by_key(|auction| auction.id);

    let total_count = auctions.len() as u32;
    let page: Vec<AuctionEntry> = auctions
        .into_iter()
        .skip(task.listfrom as usize)
        .take(50)
        .collect();
    let refs: Vec<&AuctionEntry> = page.iter().collect();

    let _ = session.send_msg(SmsgAuctionBidderListResult {
        auctions: &refs,
        total_count,
    });

    session.clear_received_ah_list_request();
    eprintln!("BIDDER_TASK: done");
}

/// Handle CMSG_AUCTION_SELL_ITEM (0x0256)
/// Handle the bidder auction list request by validating the auctioneer,
/// removing feign death, and queueing the async bidder query.
pub async fn handle_auction_list_bidder_items(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    if session.received_ah_list_request() {
        return Ok(());
    }
    let auctioneer_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read auctioneer GUID"))?;
    let paging_element_start_index = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read paging element start index"))?;
    let bid_auction_ids_to_refresh = read_bid_refresh_ids(packet);

    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
    let player = world
        .managers
        .player_mgr
        .get_player(player_guid)
        .ok_or_else(|| anyhow::anyhow!("Player not found"))?;

    let auction_house = match get_checked_auction_house_for_auctioneer(
        &player,
        auctioneer_guid,
        &world.managers.auction_mgr,
        None,
    ) {
        Some(house) => house,
        None => return Ok(()),
    };

    // Drop the read lock before acquiring a write lock to avoid deadlock.
    drop(player);
    let _ = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            let removed = player.auras.container.remove_spell_auras(AURA_FEIGN_DEATH);
            if !removed.is_empty() {
                player.auras.needs_client_update = true;
                player.auras.needs_stat_recalc = true;
            }
        });

    let Some(auction_house_map) = world
        .managers
        .auction_mgr
        .get_auctions_map_by_house_id(auction_house.house_id)
    else {
        return Ok(());
    };

    let task = AuctionHouseClientQueryTask {
        query_type: AuctionQueryType::ListBidder,
        auction_house: auction_house_map,
        account_id: session.account_id(),
        listfrom: paging_element_start_index,
        outbidded_auction_ids: bid_auction_ids_to_refresh,
        search_params: None,
    };

    session.set_received_ah_list_request(true);

    let world_clone = world.clone();
    tokio::spawn(execute_auction_list_bidder_items_task(
        world_clone,
        player_guid,
        task,
    ));

    Ok(())
}

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
            AuctionError::BidIncrement,
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
    let house_entry = get_checked_auction_house_for_auctioneer(
        &player,
        auctioneer_guid,
        &world.managers.auction_mgr,
        None, // NPC interaction not yet ported
    );

    let house_entry = match house_entry {
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

    // Drop read lock before acquiring write lock later (feign death removal).
    drop(player);

    // --- concurrent auction limit ---
    // TODO: add Config::account_concurrent_auction_limit field (u32, default 0 = no limit).
    // When non-zero: get_auctions_map_by_house_id(house_entry.house_id)
    //   .get_account_auction_count(session.account_id()) >= limit
    //   → send sysmessage + AUCTION_ERR_DATABASE, return.

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

    // --- remove feign death ---
    let _ = world.managers.player_mgr.with_player_mut(player_guid, |p| {
        let removed = p.auras.container.remove_spell_auras(AURA_FEIGN_DEATH);
        if !removed.is_empty() {
            p.auras.needs_client_update = true;
            p.auras.needs_stat_recalc = true;
        }
    });

    // --- item_guid must be non-zero ---
    if item_guid.is_empty() {
        send_auction_command_result(
            session,
            None,
            AuctionAction::Started,
            AuctionError::ItemNotFound,
            None,
        )?;
        return Ok(());
    }

    // --- item must not already be in another auction ---
    let item_guid_low = item_guid.low();
    if world
        .managers
        .auction_mgr
        .get_a_item(item_guid_low)
        .is_some()
    {
        debug!(
            "CMSG_AUCTION_SELL_ITEM: item {} already in auction, rejecting",
            item_guid_low
        );
        send_auction_command_result(
            session,
            None,
            AuctionAction::Started,
            AuctionError::Inventory,
            Some(InventoryResult::ItemNotFound),
        )?;
        return Ok(());
    }

    // --- item must exist in player inventory ---
    let item_info = world
        .systems
        .inventory
        .cache()
        .get_item_info(player_guid, item_guid);
    let item_info = match item_info {
        Some(i) => i,
        None => {
            send_auction_command_result(
                session,
                None,
                AuctionAction::Started,
                AuctionError::Inventory,
                Some(InventoryResult::ItemNotFound),
            )?;
            return Ok(());
        }
    };

    // --- item must not be in bank ---
    if is_bank_pos(item_info.bag, item_info.slot) {
        send_auction_command_result(
            session,
            None,
            AuctionAction::Started,
            AuctionError::Inventory,
            Some(InventoryResult::ItemNotFound),
        )?;
        return Ok(());
    }

    // --- item must be tradeable (not bound), not conjured, not temporary ---
    let item_arc = world
        .systems
        .inventory
        .cache()
        .get_item(player_guid, item_guid);
    let item_arc = match item_arc {
        Some(i) => i,
        None => {
            send_auction_command_result(
                session,
                None,
                AuctionAction::Started,
                AuctionError::Inventory,
                Some(InventoryResult::ItemNotFound),
            )?;
            return Ok(());
        }
    };

    {
        let item = item_arc.read();
        // ITEM_FLAG_CONJURED = 0x2, or item has a limited duration.
        let is_conjured = item.flags & 0x2 != 0;
        // ITEM_FLAG_NO_TRADE: 0x4 (lootable bond), also check 0x10000 (bind on equip already equipped).
        // C++ CanBeTraded() returns false when IS_BOUND and item.IsBoundByEnchant() etc.
        // Minimal Vanilla port: reject if flags indicate soulbound (0x1) or conjured (0x2).
        let is_bound = item.flags & 0x1 != 0;
        let has_duration = item.duration > 0;

        if is_bound || is_conjured || has_duration {
            send_auction_command_result(
                session,
                None,
                AuctionAction::Started,
                AuctionError::Inventory,
                Some(InventoryResult::ItemNotFound),
            )?;
            return Ok(());
        }
    }

    // --- calculate deposit ---
    let deposit = {
        let item = item_arc.read();
        world.managers.auction_mgr.get_auction_deposit(
            &house_entry,
            etime_secs,
            &item,
            world.config.auction_deposit_min,
            world.config.rate_auction_deposit,
        )
    };

    // --- player must have enough gold for deposit ---
    let player_money = world.systems.inventory.get_money(player_guid).unwrap_or(0);
    if player_money < deposit {
        send_auction_command_result(
            session,
            None,
            AuctionAction::Started,
            AuctionError::NotEnoughMoney,
            None,
        )?;
        return Ok(());
    }

    // --- get the auction house map early (needed for limit check and registration) ---
    let auction_house_map = match world
        .managers
        .auction_mgr
        .get_auctions_map_by_house_id(house_entry.house_id)
    {
        Some(m) => m,
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

    // --- deduct deposit ---
    world.systems.inventory.remove_gold(player_guid, deposit);

    // --- remove item from player inventory ---
    let item_template = { item_arc.read().entry };
    let auction_item = match world
        .systems
        .inventory
        .move_item_to_auction(player_guid, item_guid)
        .await
    {
        Some(i) => i,
        None => {
            // Item disappeared between check and removal; refund deposit.
            world.systems.inventory.add_gold(player_guid, deposit);
            send_auction_command_result(
                session,
                None,
                AuctionAction::Started,
                AuctionError::Inventory,
                Some(InventoryResult::ItemNotFound),
            )?;
            return Ok(());
        }
    };

    // --- build auction entry ---
    let auction_id = world.managers.auction_mgr.generate_auction_id();
    let expire_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + etime_secs as u64;

    let mut entry = AuctionEntry::new(
        auction_id,
        house_entry.house_id,
        item_guid,
        item_template,
        player_guid,
        session.account_id(),
        bid,
        buyout,
        expire_time,
        deposit,
    );

    // --- register in memory ---
    world.managers.auction_mgr.add_a_item(auction_item);
    auction_house_map.add_auction(entry.clone());

    // --- persist to DB ---
    world
        .managers
        .auction_mgr
        .save_auction_to_db(&entry)
        .await?;

    // --- notify client ---
    send_auction_command_result(
        session,
        Some(&entry),
        AuctionAction::Started,
        AuctionError::Ok,
        None,
    )?;

    Ok(())
}

/// Handle CMSG_AUCTION_REMOVE_ITEM (0x0257)
///
/// Packet format (vanilla 1.12.1):
/// - auctioneerGuid (packed u64)
/// - auctionId     (u32)
///
/// Mirrors C++ `WorldSession::HandleAuctionRemoveItem`.
pub async fn handle_auction_remove_item(
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
    let auction_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read auction ID"))?;

    debug!(
        "CMSG_AUCTION_REMOVE_ITEM: player={:?} auctioneer={:?} auction_id={}",
        player_guid, auctioneer_guid, auction_id
    );

    let player = world
        .managers
        .player_mgr
        .get_player(player_guid)
        .ok_or_else(|| anyhow::anyhow!("Player not found"))?;

    let Some(house_entry) = get_checked_auction_house_for_auctioneer(
        &player,
        auctioneer_guid,
        &world.managers.auction_mgr,
        None,
    ) else {
        return Ok(());
    };

    drop(player);

    // Remove feign death before any auction interaction.
    let _ = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            let removed = player.auras.container.remove_spell_auras(AURA_FEIGN_DEATH);
            if !removed.is_empty() {
                player.auras.needs_client_update = true;
                player.auras.needs_stat_recalc = true;
            }
        });

    let Some(auction_house_map) = world
        .managers
        .auction_mgr
        .get_auctions_map_by_house_id(house_entry.house_id)
    else {
        return Ok(());
    };

    let auction = auction_house_map.get_auction(auction_id);

    // Ownership check: must exist and belong to this player.
    let Some(auction) = auction.filter(|a| a.seller_guid == player_guid) else {
        debug!(
            "HandleAuctionRemoveItem: auction {} not found or ownership mismatch for player {:?}",
            auction_id, player_guid
        );
        send_auction_command_result(
            session,
            None,
            AuctionAction::Removed,
            AuctionError::DatabaseError,
            None,
        )?;
        return Ok(());
    };

    // If there is an active bid, the owner must pay an auction cut and the bidder
    // is refunded. If the owner cannot cover the cut, silently abort (matches C++).
    if auction.has_bid() {
        let cut = auction.get_auction_cut(house_entry.cut_percent as f32, 1.0);
        let player_money = world.systems.inventory.get_money(player_guid).unwrap_or(0);
        if player_money < cut {
            return Ok(());
        }
        world
            .managers
            .auction_mgr
            .send_auction_cancelled_to_bidder_mail(&auction)
            .await?;
        if let Some(bidder_session) = world.session_mgr.get_session_by_player(auction.bidder_guid) {
            let _ = send_auction_removed_notification(
                &bidder_session,
                &auction,
                &world.managers.auction_mgr,
            );
        }
        world.systems.inventory.remove_gold(player_guid, cut);
    }

    // Item must exist in the auction manager cache.
    let item_guid_low = auction.item_guid.low();
    if world
        .managers
        .auction_mgr
        .get_a_item(item_guid_low)
        .is_none()
    {
        debug!(
            "HandleAuctionRemoveItem: item {} not found for auction {}",
            item_guid_low, auction_id
        );
        send_auction_command_result(
            session,
            None,
            AuctionAction::Removed,
            AuctionError::Inventory,
            Some(InventoryResult::ItemNotFound),
        )?;
        return Ok(());
    }

    // Mail item back to owner; also removes item from the auction item cache.
    world
        .managers
        .auction_mgr
        .send_auction_cancel_mail_to_owner(&auction)
        .await?;

    send_auction_removed_notification(session, &auction, &world.managers.auction_mgr)?;

    send_auction_command_result(
        session,
        Some(&auction),
        AuctionAction::Removed,
        AuctionError::Ok,
        None,
    )?;

    // Persist: remove auction row from DB.
    world
        .managers
        .auction_mgr
        .delete_auction_from_db(auction.id)
        .await?;
    // TODO: persist player gold deduction to DB (no character_repo.update_money yet)

    // Remove auction from in-memory house map.
    auction_house_map.remove_auction(auction_id);

    Ok(())
}

/// Handle CMSG_AUCTION_PLACE_BID (0x025A)
///
/// Packet format (vanilla 1.12.1):
/// - auctioneerGuid (packed u64)
/// - auctionId     (u32)
/// - price         (u32)
///
/// Mirrors C++ `WorldSession::HandleAuctionPlaceBid`.
pub async fn handle_auction_place_bid(
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
    let auction_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read auction ID"))?;
    let price = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read bid price"))?;

    debug!(
        "CMSG_AUCTION_PLACE_BID: player={:?} auctioneer={:?} auction_id={} price={}",
        player_guid, auctioneer_guid, auction_id, price
    );

    // GM trade restriction.
    let gm_allow_trades = world.config.gm_allow_trades.unwrap_or(false);
    if !gm_allow_trades && session.security() > 0 {
        send_auction_command_result(
            session,
            None,
            AuctionAction::BidPlaced,
            AuctionError::RestrictedAccount,
            None,
        )?;
        return Ok(());
    }

    // auction_id or price of zero is a cheater signal.
    if auction_id == 0 || price == 0 {
        send_auction_command_result(
            session,
            None,
            AuctionAction::BidPlaced,
            AuctionError::ItemNotFound,
            None,
        )?;
        return Ok(());
    }

    // Auctioneer validation.
    let player = world
        .managers
        .player_mgr
        .get_player(player_guid)
        .ok_or_else(|| anyhow::anyhow!("Player not found"))?;

    let house_entry = get_checked_auction_house_for_auctioneer(
        &player,
        auctioneer_guid,
        &world.managers.auction_mgr,
        None,
    );
    let Some(house_entry) = house_entry else {
        send_auction_command_result(
            session,
            None,
            AuctionAction::BidPlaced,
            AuctionError::ItemNotFound,
            None,
        )?;
        return Ok(());
    };

    drop(player);

    // Remove feign death.
    let _ = world.managers.player_mgr.with_player_mut(player_guid, |p| {
        let removed = p.auras.container.remove_spell_auras(AURA_FEIGN_DEATH);
        if !removed.is_empty() {
            p.auras.needs_client_update = true;
            p.auras.needs_stat_recalc = true;
        }
    });

    let Some(auction_house_map) = world
        .managers
        .auction_mgr
        .get_auctions_map_by_house_id(house_entry.house_id)
    else {
        return Ok(());
    };

    // Auction must exist.
    let Some(auction) = auction_house_map.get_auction(auction_id) else {
        send_auction_command_result(
            session,
            None,
            AuctionAction::BidPlaced,
            AuctionError::ItemNotFound,
            None,
        )?;
        return Ok(());
    };

    // Can't bid on your own auction.
    if auction.seller_guid == player_guid {
        send_auction_command_result(
            session,
            None,
            AuctionAction::BidPlaced,
            AuctionError::BidOwn,
            None,
        )?;
        return Ok(());
    }

    // Cross-account ownership check: same account even if seller is offline.
    if auction.seller_account != 0 && auction.seller_account == session.account_id() {
        send_auction_command_result(
            session,
            None,
            AuctionAction::BidPlaced,
            AuctionError::BidOwn,
            None,
        )?;
        return Ok(());
    }

    // Bid must meet the starting price.
    if price < auction.start_bid {
        send_auction_command_result(
            session,
            None,
            AuctionAction::BidPlaced,
            AuctionError::BidIncrement,
            None,
        )?;
        return Ok(());
    }

    // Bid must exceed the current bid.
    if price <= auction.current_bid {
        send_auction_command_result(
            session,
            Some(&auction),
            AuctionAction::BidPlaced,
            AuctionError::HigherBid,
            None,
        )?;
        return Ok(());
    }

    let buyout = auction.buyout_price;

    // Bid must meet the minimum increment when not buying out.
    if (buyout == 0 || price < buyout) && price < auction.current_bid + auction.get_outbid_amount()
    {
        send_auction_command_result(
            session,
            Some(&auction),
            AuctionAction::BidPlaced,
            AuctionError::BidIncrement,
            None,
        )?;
        return Ok(());
    }

    let player_money = world.systems.inventory.get_money(player_guid).unwrap_or(0);
    // C++ silently returns without a packet when the player can't afford it.
    if price > player_money {
        return Ok(());
    }

    let is_rebid = auction.has_bid() && auction.bidder_guid == player_guid;

    // Displace existing bidder (if any and it's not a re-bid by the same player).
    if !is_rebid && auction.has_bid() {
        world
            .managers
            .auction_mgr
            .notify_bidder_outbid_or_won(&auction)
            .await?;
        if let Some(old_session) = world.session_mgr.get_session_by_player(auction.bidder_guid) {
            let random_prop = world
                .managers
                .auction_mgr
                .get_a_item(auction.item_guid.low())
                .map(|i| i.random_property_id as u32)
                .unwrap_or(0);
            let _ = send_auction_bidder_notification(
                &old_session,
                &auction,
                house_entry.house_id,
                false,
                random_prop,
            );
        }
    }

    if buyout != 0 && price >= buyout {
        // --- BUYOUT PATH ---
        let deduct = if is_rebid {
            buyout - auction.current_bid
        } else {
            buyout
        };
        world.systems.inventory.remove_gold(player_guid, deduct);

        let mut settled_auction = auction.clone();
        settled_auction.bidder_guid = player_guid;
        settled_auction.current_bid = buyout;

        // Snapshot random property before send_auction_won_mail removes the item.
        let random_prop = world
            .managers
            .auction_mgr
            .get_a_item(settled_auction.item_guid.low())
            .map(|i| i.random_property_id as u32)
            .unwrap_or(0);

        let cut_percent = house_entry.cut_percent as f32;
        world
            .managers
            .auction_mgr
            .send_auction_successful_mail(&settled_auction, cut_percent, 1.0)
            .await?;

        if let Some(seller_session) = world
            .session_mgr
            .get_session_by_player(settled_auction.seller_guid)
        {
            let _ = send_auction_owner_notification(
                &seller_session,
                &settled_auction,
                true,
                random_prop as i32,
            );
        }

        world
            .managers
            .auction_mgr
            .send_auction_won_mail(&settled_auction)
            .await?;

        // Live won notification to buyer (current session).
        let _ = send_auction_bidder_notification(
            session,
            &settled_auction,
            house_entry.house_id,
            true,
            random_prop,
        );

        send_auction_command_result(
            session,
            Some(&settled_auction),
            AuctionAction::BidPlaced,
            AuctionError::Ok,
            None,
        )?;

        auction_house_map.remove_auction(auction_id);
        world
            .managers
            .auction_mgr
            .delete_auction_from_db(auction_id)
            .await?;
        // TODO: persist player gold deduction to DB (no character_repo.update_money yet)
    } else {
        // --- BID PATH ---
        let deduct = if is_rebid {
            price - auction.current_bid
        } else {
            price
        };
        world.systems.inventory.remove_gold(player_guid, deduct);

        auction_house_map.update_bid(auction_id, player_guid, price);

        let mut updated_auction = auction.clone();
        updated_auction.bidder_guid = player_guid;
        updated_auction.current_bid = price;

        if let Some(seller_session) = world
            .session_mgr
            .get_session_by_player(updated_auction.seller_guid)
        {
            let random_prop = world
                .managers
                .auction_mgr
                .get_a_item(updated_auction.item_guid.low())
                .map(|i| i.random_property_id as u32)
                .unwrap_or(0);
            let _ = send_auction_owner_notification(
                &seller_session,
                &updated_auction,
                false,
                random_prop as i32,
            );
        }

        world
            .managers
            .auction_mgr
            .update_bid_in_db(auction_id, player_guid.low(), price)
            .await?;

        send_auction_command_result(
            session,
            Some(&updated_auction),
            AuctionAction::BidPlaced,
            AuctionError::Ok,
            None,
        )?;
        // TODO: persist player gold deduction to DB (no character_repo.update_money yet)
    }

    Ok(())
}

/// Handle the auction hello packet by validating the target auctioneer,
/// clearing feign death if needed, and sending the open-auction response.
fn send_auction_hello_response(
    session: &WorldSession,
    player_guid: crate::shared::protocol::ObjectGuid,
    auctioneer_guid: crate::shared::protocol::ObjectGuid,
    player_mgr: &PlayerManager,
    creature_mgr: &CreatureManager,
    auction_mgr: &AuctionHouseManager,
) -> Result<()> {
    let Some(creature) = creature_mgr
        .get_creature(auctioneer_guid)
        .map(|creature| creature.value().clone())
    else {
        debug!(
            "MSG_AUCTION_HELLO: auctioneer {:?} not found or you can't interact with him.",
            auctioneer_guid
        );
        return Ok(());
    };

    if creature.npc_flags & NPC_FLAG_AUCTIONEER == 0 {
        debug!(
            "MSG_AUCTION_HELLO: auctioneer {:?} missing auctioneer flag.",
            auctioneer_guid
        );
        return Ok(());
    }

    let Some(auction_house) = auction_mgr.get_auction_house_for_npc(creature.faction) else {
        debug!(
            "MSG_AUCTION_HELLO: auctioneer {:?} resolved to no auction house.",
            auctioneer_guid
        );
        return Ok(());
    };

    let removed_feign_death = player_mgr
        .with_player_mut(player_guid, |player| {
            let removed = player.auras.container.remove_spell_auras(AURA_FEIGN_DEATH);
            if !removed.is_empty() {
                player.auras.needs_client_update = true;
                player.auras.needs_stat_recalc = true;
            }
            removed.len()
        })
        .ok_or_else(|| anyhow::anyhow!("Player not found"))?;

    if removed_feign_death > 0 {
        debug!(
            "MSG_AUCTION_HELLO: cleared feign death aura(s) for player {:?}",
            player_guid
        );
    }

    session.send_msg(MsgAuctionHello {
        auctioneer_guid,
        house_id: auction_house.house_id,
    })?;

    Ok(())
}

pub async fn handle_auction_hello(
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

    debug!(
        "MSG_AUCTION_HELLO: player={:?} auctioneer={:?}",
        player_guid, auctioneer_guid
    );

    send_auction_hello_response(
        session,
        player_guid,
        auctioneer_guid,
        &world.managers.player_mgr,
        &world.managers.creature_mgr,
        &world.managers.auction_mgr,
    )
}

/// Handle CMSG_AUCTION_LIST_OWNER_ITEMS (0x0259)
///
/// Packet format (vanilla 1.12.1):
/// - auctioneerGuid (packed u64)
/// - listfrom     (u32) -- paging offset
///
/// Mirrors C++ `WorldSession::HandleAuctionListOwnerItems`.
/// Rejects duplicate in-flight requests, validates the auction house,
/// clears feign-death auras, then enqueues an async owner-query task.
pub async fn handle_auction_list_owner_items(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    if session.received_ah_list_request() {
        return Ok(());
    }

    let auctioneer_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read auctioneer GUID"))?;
    let listfrom = packet.read_u32().unwrap_or(0);

    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
    let player = world
        .managers
        .player_mgr
        .get_player(player_guid)
        .ok_or_else(|| anyhow::anyhow!("Player not found"))?;

    let auction_house = match get_checked_auction_house_for_auctioneer(
        &player,
        auctioneer_guid,
        &world.managers.auction_mgr,
        None,
    ) {
        Some(house) => house,
        None => return Ok(()),
    };

    // Drop the read lock before acquiring a write lock to avoid deadlock.
    drop(player);
    let _ = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            let removed = player.auras.container.remove_spell_auras(AURA_FEIGN_DEATH);
            if !removed.is_empty() {
                player.auras.needs_client_update = true;
                player.auras.needs_stat_recalc = true;
            }
        });

    let Some(auction_house_map) = world
        .managers
        .auction_mgr
        .get_auctions_map_by_house_id(auction_house.house_id)
    else {
        return Ok(());
    };

    let task = AuctionHouseClientQueryTask {
        query_type: AuctionQueryType::ListOwner,
        auction_house: auction_house_map,
        account_id: session.account_id(),
        listfrom,
        outbidded_auction_ids: Vec::new(),
        search_params: None,
    };

    session.set_received_ah_list_request(true);
    eprintln!("OWNER: spawning task");

    let world_clone = world.clone();
    tokio::spawn(execute_auction_list_owner_items_task(
        world_clone,
        player_guid,
        task,
    ));

    Ok(())
}

async fn execute_auction_list_owner_items_task(
    world: World,
    player_guid: crate::shared::protocol::ObjectGuid,
    task: AuctionHouseClientQueryTask,
) {
    if task.query_type != AuctionQueryType::ListOwner {
        tracing::warn!("Auction query task invoked with unexpected query type");
        return;
    }

    tracing::debug!(
        "Executing auction owner list query for account {}",
        task.account_id
    );

    let Some(session) = world.session_mgr.get_session_by_player(player_guid) else {
        return;
    };

    let mut auctions: Vec<AuctionEntry> = task
        .auction_house
        .auctions_snapshot()
        .into_iter()
        .filter(|auction| auction.seller_account == task.account_id)
        .collect();
    auctions.sort_by_key(|auction| auction.id);

    let total_count = auctions.len() as u32;
    let page: Vec<AuctionEntry> = auctions
        .into_iter()
        .skip(task.listfrom as usize)
        .take(50)
        .collect();
    let refs: Vec<&AuctionEntry> = page.iter().collect();

    if let Err(e) = session.send_msg(SmsgAuctionOwnerListResult {
        auctions: &refs,
        total_count,
    }) {
        tracing::error!("Failed to send auction owner list result: {}", e);
    }

    session.clear_received_ah_list_request();
    eprintln!("OWNER_TASK: done");
}

/// Handle CMSG_AUCTION_LIST_ITEMS (0x0258)
///
/// Packet format (vanilla 1.12.1):
/// - auctioneerGuid (packed u64)
/// - listfrom       (u32)
/// - searchedname   (null-terminated string)
/// - levelmin       (u8)
/// - levelmax       (u8)
/// - auctionSlotID  (u32)
/// - auctionMainCategory (u32)
/// - auctionSubCategory  (u32)
/// - quality        (u32)
/// - usable         (u8)
///
/// Mirrors C++ `WorldSession::HandleAuctionListItems`.
pub async fn handle_auction_list_items(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    if session.received_ah_list_request() {
        return Ok(());
    }

    let auctioneer_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read auctioneer GUID"))?;
    let listfrom = packet.read_u32().unwrap_or(0);
    let search_name_raw = packet.read_string().unwrap_or_default();
    let level_min = packet.read_u8().unwrap_or(0);
    let level_max = packet.read_u8().unwrap_or(0);
    let auction_slot_id = packet.read_u32().unwrap_or(0);
    let auction_main_category = packet.read_u32().unwrap_or(u32::MAX);
    let auction_sub_category = packet.read_u32().unwrap_or(u32::MAX);
    let quality = packet.read_u32().unwrap_or(u32::MAX);
    let usable = packet.read_u8().unwrap_or(0);

    // Reject invalid (non-UTF-8) search names; mirrors C++ Utf8toWStr failure branch.
    // Empty string is always valid.
    if !search_name_raw.is_empty() && search_name_raw.contains('\u{FFFD}') {
        return Ok(());
    }

    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
    let player = world
        .managers
        .player_mgr
        .get_player(player_guid)
        .ok_or_else(|| anyhow::anyhow!("Player not found"))?;

    let auction_house = match get_checked_auction_house_for_auctioneer(
        &player,
        auctioneer_guid,
        &world.managers.auction_mgr,
        None,
    ) {
        Some(house) => house,
        None => return Ok(()),
    };

    drop(player);
    let _ = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            let removed = player.auras.container.remove_spell_auras(AURA_FEIGN_DEATH);
            if !removed.is_empty() {
                player.auras.needs_client_update = true;
                player.auras.needs_stat_recalc = true;
            }
        });

    let Some(auction_house_map) = world
        .managers
        .auction_mgr
        .get_auctions_map_by_house_id(auction_house.house_id)
    else {
        return Ok(());
    };

    let task = AuctionHouseClientQueryTask {
        query_type: AuctionQueryType::List,
        auction_house: auction_house_map,
        account_id: session.account_id(),
        listfrom,
        outbidded_auction_ids: Vec::new(),
        search_params: Some(AuctionSearchParams {
            search_name: search_name_raw.to_lowercase(),
            level_min,
            level_max,
            auction_slot_id,
            auction_main_category,
            auction_sub_category,
            quality,
            usable,
        }),
    };

    session.set_received_ah_list_request(true);

    let world_clone = world.clone();
    tokio::spawn(execute_auction_list_items_task(
        world_clone,
        player_guid,
        task,
    ));

    Ok(())
}

async fn execute_auction_list_items_task(
    world: World,
    player_guid: crate::shared::protocol::ObjectGuid,
    task: AuctionHouseClientQueryTask,
) {
    let Some(session) = world.session_mgr.get_session_by_player(player_guid) else {
        return;
    };

    let params = match &task.search_params {
        Some(p) => p,
        None => {
            tracing::warn!("auction list items task missing search params");
            session.clear_received_ah_list_request();
            return;
        }
    };

    let mut auctions: Vec<AuctionEntry> = task
        .auction_house
        .auctions_snapshot()
        .into_iter()
        .filter(|auction| {
            let Some(tmpl) = world.managers.item_mgr.get_template(auction.item_template) else {
                return params.search_name.is_empty();
            };

            if !params.search_name.is_empty()
                && !tmpl.name.to_lowercase().contains(&params.search_name)
            {
                return false;
            }

            if params.level_min > 0 && (tmpl.required_level as u8) < params.level_min {
                return false;
            }
            if params.level_max > 0 && (tmpl.required_level as u8) > params.level_max {
                return false;
            }

            // u32::MAX means "all" in the client protocol
            if params.auction_main_category != u32::MAX
                && tmpl.item_class != params.auction_main_category
            {
                return false;
            }
            if params.auction_sub_category != u32::MAX
                && tmpl.item_subclass != params.auction_sub_category
            {
                return false;
            }
            if params.quality != u32::MAX && (tmpl.quality as u32) < params.quality {
                return false;
            }

            true
        })
        .collect();
    auctions.sort_by_key(|a| a.id);

    let total_count = auctions.len() as u32;
    let page: Vec<AuctionEntry> = auctions
        .into_iter()
        .skip(task.listfrom as usize)
        .take(50)
        .collect();
    let refs: Vec<&AuctionEntry> = page.iter().collect();

    if let Err(e) = session.send_msg(SmsgAuctionListResult {
        auctions: &refs,
        total_count,
    }) {
        tracing::error!("Failed to send auction list result: {}", e);
    }

    session.clear_received_ah_list_request();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::database::characters::repositories::auction_repository_trait::MockAuctionRepositoryTrait;
    use crate::shared::database::characters::repositories::character_repository::CharacterRepository;
    use crate::shared::database::characters::repositories::item_repository::ItemRepository;
    use crate::shared::database::characters::repositories::item_repository_trait::ItemRepositoryTrait;
    use crate::shared::database::characters::repositories::mail_repository::MailRepository;
    use crate::shared::protocol::{ObjectGuid, Opcode, Position, WorldPacket};
    use crate::world::core::session::WorldSession;
    use crate::world::dbc::manager::DbcManager;
    use crate::world::dbc::structures::AuctionHouseEntry;
    use crate::world::game::auction::manager::AuctionHouseManager;
    use crate::world::game::creature::{Creature, CreatureManager, CreatureTemplate};
    use crate::world::game::items::manager::ItemManager;
    use crate::world::game::player::auras::aura::{Aura, AuraFlags};
    use crate::world::game::player::player::Player;
    use crate::world::game::player::PlayerManager;
    use parking_lot::RwLock;
    use sqlx::mysql::MySqlPoolOptions;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    fn test_player_guid() -> ObjectGuid {
        ObjectGuid::new_player(1)
    }

    fn test_auctioneer_guid(entry: u32) -> ObjectGuid {
        ObjectGuid::new_creature(entry, 1)
    }

    fn test_creature_template(entry: u32, npc_flags: u32, faction: u32) -> CreatureTemplate {
        CreatureTemplate {
            entry,
            name: format!("Auctioneer{}", entry),
            subname: None,
            min_level: 1,
            max_level: 1,
            faction,
            model_id_1: 1,
            model_id_2: 0,
            model_id_3: 0,
            model_id_4: 0,
            scale: 1.0,
            npc_flags,
            unit_flags: 0,
            static_flags1: 0,
            flags_extra: 0,
            creature_type: 1,
            unit_class: 1,
            health_multiplier: 1.0,
            power_multiplier: 1.0,
            armor_multiplier: 1.0,
            damage_multiplier: 1.0,
            damage_variance: 0.0,
            attack_time: 2000,
            rank: 0,
            gossip_menu_id: 0,
            vendor_id: 0,
            trainer_id: 0,
            trainer_type: 0,
            spells: [0; 4],
        }
    }

    fn make_session() -> (WorldSession, mpsc::UnboundedReceiver<WorldPacket>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let session = WorldSession::new(1, 1, "Test".to_string(), 0, tx);
        session.set_player_guid(Some(test_player_guid()));
        (session, rx)
    }

    fn make_player_mgr(with_feign_death: bool, auction_access_mode: i8) -> Arc<PlayerManager> {
        let player_mgr = Arc::new(PlayerManager::new());
        let player_guid = test_player_guid();
        let mut player = Player::new(player_guid, "Tester".to_string(), 0, 0, 0, 1, 1, 1, 0);
        player.auction_access_mode = auction_access_mode;

        if with_feign_death {
            let aura = Aura::new(
                AURA_FEIGN_DEATH,
                player_guid,
                0,
                crate::world::game::player::auras::effects::AURA_DUMMY,
                0,
                0,
                Some(60_000),
                0,
                1,
                0,
                AuraFlags::default(),
            );
            let _ = player.auras.container.add_aura(aura);
        }

        player_mgr.add_player(player, 1);
        player_mgr
    }

    fn make_creature_mgr(entry: u32, npc_flags: u32, faction: u32) -> Arc<CreatureManager> {
        let pool = Arc::new(
            MySqlPoolOptions::new()
                .connect_lazy("mysql://test:test@localhost/test")
                .expect("lazy pool"),
        );
        let creature_mgr = Arc::new(CreatureManager::new(pool));
        let template = test_creature_template(entry, npc_flags, faction);
        creature_mgr.add_template(template.clone());

        let creature = Creature::new(
            test_auctioneer_guid(entry),
            entry,
            1,
            Position::default(),
            0,
            0,
            &template,
            1,
            None,
        );
        creature_mgr.add_creature(creature);
        creature_mgr
    }

    fn make_auction_mgr() -> Arc<AuctionHouseManager> {
        let pool = Arc::new(
            MySqlPoolOptions::new()
                .connect_lazy("mysql://test:test@localhost/test")
                .expect("lazy pool"),
        );
        let character_repo = Arc::new(CharacterRepository::new(Arc::clone(&pool)));
        let mail_repo = Arc::new(MailRepository::new(Arc::clone(&pool)));
        let item_repo: Arc<dyn ItemRepositoryTrait> =
            Arc::new(ItemRepository::new(Arc::clone(&pool)));
        let auction_repo = Arc::new(MockAuctionRepositoryTrait::new());
        let item_mgr = Arc::new(ItemManager::new());

        let mut dbc = DbcManager::new();
        dbc.auction_house.insert(
            1,
            AuctionHouseEntry {
                house_id: 1,
                faction: 0,
                deposit_percent: 5,
                cut_percent: 5,
            },
        );
        dbc.auction_house.insert(
            7,
            AuctionHouseEntry {
                house_id: 7,
                faction: 0,
                deposit_percent: 5,
                cut_percent: 5,
            },
        );
        dbc.auction_house.insert(
            7,
            AuctionHouseEntry {
                house_id: 7,
                faction: 0,
                deposit_percent: 5,
                cut_percent: 5,
            },
        );

        Arc::new(AuctionHouseManager::new(
            auction_repo,
            character_repo,
            mail_repo,
            item_repo,
            Arc::new(RwLock::new(dbc)),
            item_mgr,
        ))
    }

    fn read_packet(mut rx: mpsc::UnboundedReceiver<WorldPacket>) -> WorldPacket {
        rx.try_recv().expect("expected packet")
    }

    #[tokio::test]
    async fn auction_hello_sends_response_and_clears_feign_death() {
        let (session, mut rx) = make_session();
        let player_mgr = make_player_mgr(true, 0);
        let creature_mgr = make_creature_mgr(100, NPC_FLAG_AUCTIONEER, 11);
        let auction_mgr = make_auction_mgr();
        let auctioneer_guid = test_auctioneer_guid(100);

        let result = send_auction_hello_response(
            &session,
            test_player_guid(),
            auctioneer_guid,
            &player_mgr,
            &creature_mgr,
            &auction_mgr,
        );

        assert!(result.is_ok());
        let packet = read_packet(rx);
        assert_eq!(packet.opcode(), Opcode::MSG_AUCTION_HELLO);
        assert_eq!(packet.data().len(), 12);
        assert_eq!(
            u64::from_le_bytes(packet.data()[0..8].try_into().unwrap()),
            auctioneer_guid.raw()
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[8..12].try_into().unwrap()),
            1
        );
        let player = player_mgr.get_player(test_player_guid()).expect("player");
        assert!(!player.auras.container.has_aura(AURA_FEIGN_DEATH));
    }

    #[tokio::test]
    async fn auction_hello_rejects_missing_auctioneer() {
        let (session, mut rx) = make_session();
        let player_mgr = make_player_mgr(false, 0);
        let creature_mgr = Arc::new(CreatureManager::new(Arc::new(
            MySqlPoolOptions::new()
                .connect_lazy("mysql://test:test@localhost/test")
                .expect("lazy pool"),
        )));
        let auction_mgr = make_auction_mgr();

        let result = send_auction_hello_response(
            &session,
            test_player_guid(),
            test_auctioneer_guid(200),
            &player_mgr,
            &creature_mgr,
            &auction_mgr,
        );

        assert!(result.is_ok());
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn auction_hello_rejects_non_auctioneer() {
        let (session, mut rx) = make_session();
        let player_mgr = make_player_mgr(false, 0);
        let creature_mgr = make_creature_mgr(101, 0, 11);
        let auction_mgr = make_auction_mgr();

        let result = send_auction_hello_response(
            &session,
            test_player_guid(),
            test_auctioneer_guid(101),
            &player_mgr,
            &creature_mgr,
            &auction_mgr,
        );

        assert!(result.is_ok());
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn auction_hello_rejects_unmapped_house() {
        let (session, mut rx) = make_session();
        let player_mgr = make_player_mgr(false, 0);
        let creature_mgr = make_creature_mgr(102, NPC_FLAG_AUCTIONEER, 99_999);
        // Use an auction manager with only house 1, so faction 99_999 -> house 7 is missing
        let auction_mgr = {
            let pool = Arc::new(
                MySqlPoolOptions::new()
                    .connect_lazy("mysql://test:test@localhost/test")
                    .expect("lazy pool"),
            );
            let character_repo = Arc::new(CharacterRepository::new(Arc::clone(&pool)));
            let mail_repo = Arc::new(MailRepository::new(Arc::clone(&pool)));
            let item_repo: Arc<dyn ItemRepositoryTrait> =
                Arc::new(ItemRepository::new(Arc::clone(&pool)));
            let auction_repo = Arc::new(MockAuctionRepositoryTrait::new());
            let item_mgr = Arc::new(ItemManager::new());
            let mut dbc = DbcManager::new();
            dbc.auction_house.insert(
                1,
                AuctionHouseEntry {
                    house_id: 1,
                    faction: 0,
                    deposit_percent: 5,
                    cut_percent: 5,
                },
            );
            Arc::new(AuctionHouseManager::new(
                auction_repo,
                character_repo,
                mail_repo,
                item_repo,
                Arc::new(RwLock::new(dbc)),
                item_mgr,
            ))
        };

        let result = send_auction_hello_response(
            &session,
            test_player_guid(),
            test_auctioneer_guid(102),
            &player_mgr,
            &creature_mgr,
            &auction_mgr,
        );

        assert!(result.is_ok());
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn auction_list_owner_rejects_duplicate_request() {
        let (session, mut rx) = make_session();
        session.set_received_ah_list_request(true);

        let mut packet = WorldPacket::new(Opcode::CMSG_AUCTION_LIST_OWNER_ITEMS);
        packet.write_guid(test_auctioneer_guid(100));
        packet.write_u32(0);

        let result =
            handle_auction_list_owner_items(&session, &mut packet, &make_world_fixture()).await;

        assert!(result.is_ok());
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn auction_list_owner_rejects_invalid_auctioneer() {
        let (session, mut rx) = make_session();
        let mut packet = WorldPacket::new(Opcode::CMSG_AUCTION_LIST_OWNER_ITEMS);
        packet.write_guid(test_auctioneer_guid(999)); // Unknown auctioneer
        packet.write_u32(0);

        let result =
            handle_auction_list_owner_items(&session, &mut packet, &make_world_fixture()).await;

        assert!(result.is_ok());
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn auction_list_owner_success_sets_gate_and_spawns_task() {
        let (session, mut rx) = make_session();
        let world = make_world_fixture();
        world
            .managers
            .auction_mgr
            .load_auction_houses(false, false, 112)
            .unwrap();
        let mut packet = WorldPacket::new(Opcode::CMSG_AUCTION_LIST_OWNER_ITEMS);
        // Use player GUID as auctioneer (self/GM path) so validation passes
        packet.write_guid(test_player_guid());
        packet.write_u32(0);

        let result = handle_auction_list_owner_items(&session, &mut packet, &world).await;

        assert!(result.is_ok());
        assert!(session.received_ah_list_request());
        // Allow spawned task to be processed so the runtime can shut down
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn auction_list_bidder_success_sends_bidder_list_result() {
        let (session, mut rx) = make_session();
        let session = Arc::new(session);
        let world = make_world_fixture();
        world.session_mgr.add_session(Arc::clone(&session));
        world
            .session_mgr
            .register_player(session.id(), test_player_guid());
        world
            .managers
            .player_mgr
            .with_player_mut(test_player_guid(), |player| {
                player.auction_access_mode = 1;
            });
        world
            .managers
            .auction_mgr
            .load_auction_houses(false, false, 112)
            .unwrap();

        let bidder_guid = test_player_guid();
        let house = world
            .managers
            .auction_mgr
            .get_auctions_map_by_house_id(7)
            .expect("auction house map");
        house.add_auction(AuctionEntry {
            id: 9001,
            house_id: 7,
            item_guid: crate::shared::protocol::ObjectGuid::new_without_entry(
                crate::shared::protocol::HighGuid::Item,
                77,
            ),
            item_template: 1234,
            seller_guid: crate::shared::protocol::ObjectGuid::new_player(2),
            seller_account: 2,
            start_bid: 100,
            current_bid: 125,
            buyout_price: 500,
            expire_time: 4_102_444_800,
            bidder_guid,
            deposit: 10,
            deposit_time: 0,
            locked_ip_address: String::new(),
        });

        let mut packet = WorldPacket::new(Opcode::CMSG_AUCTION_LIST_BIDDER_ITEMS);
        packet.write_guid(bidder_guid);
        packet.write_u32(0);

        let result = handle_auction_list_bidder_items(session.as_ref(), &mut packet, &world).await;
        assert!(result.is_ok());

        let packet = {
            let mut packet = None;
            for _ in 0..1000 {
                match rx.try_recv() {
                    Ok(value) => {
                        packet = Some(value);
                        break;
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                    }
                    Err(err) => panic!("expected bidder list packet: {err}"),
                }
            }
            packet.expect("expected bidder list packet")
        };

        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_BIDDER_LIST_RESULT);
        assert_eq!(
            u32::from_le_bytes(packet.data()[0..4].try_into().unwrap()),
            1
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[4..8].try_into().unwrap()),
            9001
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[8..12].try_into().unwrap()),
            1234
        );
        assert_eq!(
            u64::from_le_bytes(packet.data()[32..40].try_into().unwrap()),
            crate::shared::protocol::ObjectGuid::new_player(2).raw()
        );
        assert_eq!(
            u64::from_le_bytes(packet.data()[56..64].try_into().unwrap()),
            bidder_guid.raw()
        );
        assert!(!session.received_ah_list_request());
    }

    fn make_list_items_packet(
        auctioneer_guid: crate::shared::protocol::ObjectGuid,
        search: &str,
    ) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::CMSG_AUCTION_LIST_ITEMS);
        packet.write_guid(auctioneer_guid);
        packet.write_u32(0); // listfrom
                             // null-terminated search string
        for b in search.as_bytes() {
            packet.write_u8(*b);
        }
        packet.write_u8(0); // null terminator
        packet.write_u8(0); // levelmin
        packet.write_u8(0); // levelmax
        packet.write_u32(u32::MAX); // auctionSlotID (all)
        packet.write_u32(u32::MAX); // auctionMainCategory (all)
        packet.write_u32(u32::MAX); // auctionSubCategory (all)
        packet.write_u32(u32::MAX); // quality (all)
        packet.write_u8(0); // usable (all)
        packet
    }

    #[tokio::test]
    async fn auction_list_items_rejects_duplicate_request() {
        let (session, mut rx) = make_session();
        session.set_received_ah_list_request(true);
        let mut packet = make_list_items_packet(test_auctioneer_guid(100), "");
        let result = handle_auction_list_items(&session, &mut packet, &make_world_fixture()).await;
        assert!(result.is_ok());
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn auction_list_items_rejects_invalid_auctioneer() {
        let (session, mut rx) = make_session();
        let mut packet = make_list_items_packet(test_auctioneer_guid(999), "");
        let result = handle_auction_list_items(&session, &mut packet, &make_world_fixture()).await;
        assert!(result.is_ok());
        assert!(rx.try_recv().is_err());
        assert!(!session.received_ah_list_request());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn auction_list_items_success_sends_list_result() {
        let (session, mut rx) = make_session();
        let session = Arc::new(session);
        let world = make_world_fixture();
        world.session_mgr.add_session(Arc::clone(&session));
        world
            .session_mgr
            .register_player(session.id(), test_player_guid());
        world
            .managers
            .player_mgr
            .with_player_mut(test_player_guid(), |player| {
                player.auction_access_mode = 1;
            });
        world
            .managers
            .auction_mgr
            .load_auction_houses(false, false, 112)
            .unwrap();

        let house = world
            .managers
            .auction_mgr
            .get_auctions_map_by_house_id(7)
            .expect("auction house map");
        house.add_auction(AuctionEntry {
            id: 8001,
            house_id: 7,
            item_guid: crate::shared::protocol::ObjectGuid::new_without_entry(
                crate::shared::protocol::HighGuid::Item,
                88,
            ),
            item_template: 5678,
            seller_guid: crate::shared::protocol::ObjectGuid::new_player(3),
            seller_account: 3,
            start_bid: 200,
            current_bid: 200,
            buyout_price: 1000,
            expire_time: 4_102_444_800,
            bidder_guid: crate::shared::protocol::ObjectGuid::empty(),
            deposit: 20,
            deposit_time: 0,
            locked_ip_address: String::new(),
        });

        let mut packet = make_list_items_packet(test_player_guid(), "");

        let result = handle_auction_list_items(session.as_ref(), &mut packet, &world).await;
        assert!(result.is_ok());
        assert!(session.received_ah_list_request());

        let packet = {
            let mut out = None;
            for _ in 0..1000 {
                match rx.try_recv() {
                    Ok(p) => {
                        out = Some(p);
                        break;
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                    }
                    Err(e) => panic!("channel error: {e}"),
                }
            }
            out.expect("expected list result packet")
        };

        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_LIST_RESULT);
        // count = 1
        assert_eq!(
            u32::from_le_bytes(packet.data()[0..4].try_into().unwrap()),
            1
        );
        // first auction id = 8001
        assert_eq!(
            u32::from_le_bytes(packet.data()[4..8].try_into().unwrap()),
            8001
        );
        assert!(!session.received_ah_list_request());
    }

    fn make_world_fixture() -> World {
        let pool = Arc::new(
            MySqlPoolOptions::new()
                .connect_lazy("mysql://test:test@localhost/test")
                .expect("lazy pool"),
        );
        let databases = Arc::new(crate::shared::database::Databases {
            world: (*pool).clone(),
            character: (*pool).clone(),
            auth: (*pool).clone(),
            logs: (*pool).clone(),
        });
        let config = Arc::new(crate::world::config::Config::default());
        let mut world = World::new(databases, config, 50, std::path::PathBuf::from("."));
        world.managers.auction_mgr = make_auction_mgr();
        world.managers.player_mgr = make_player_mgr(false, 1);
        world.managers.creature_mgr = make_creature_mgr(100, NPC_FLAG_AUCTIONEER, 11);
        world
    }
}
