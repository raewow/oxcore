//! Trade System - player-to-player trading for world

use anyhow::Result;
use std::sync::Arc;

use crate::shared::messages::trade::{SmsgTradeStatusExtendedV2, SmsgTradeStatusV2, TradeSlotInfoV2};
use crate::shared::protocol::{ObjectGuid, Position};
use crate::world::game::trade::types::TradeStatus;
use crate::world::game::player::PlayerManager;
use crate::world::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::world::game::inventory::InventorySystem;
use crate::world::game::ItemManager;

use super::cache::{ActiveTrade, TradeCache};
use super::types::{
    TradeData, TradeError, TradeState, BANK_SLOT_END, BANK_SLOT_START, MAX_MONEY,
    TRADE_SLOT_COUNT, TRADE_SLOT_TRADED_COUNT,
};

/// Trade System - manages player-to-player trading
pub struct TradeSystem {
    /// Trade session cache
    cache: TradeCache,
    /// Broadcast manager for sending packets
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
    /// Player manager for player lookups
    player_mgr: Arc<PlayerManager>,
    /// Inventory system for item/gold operations
    inventory: Arc<InventorySystem>,
    /// Item manager for item templates (display_id, etc.)
    item_mgr: Arc<ItemManager>,
    /// Whether cross-faction trading is allowed
    allow_cross_faction_trade: bool,
}

impl TradeSystem {
    /// Create a new trade system
    pub fn new(
        broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
        player_mgr: Arc<PlayerManager>,
        inventory: Arc<InventorySystem>,
        item_mgr: Arc<ItemManager>,
        allow_cross_faction_trade: bool,
    ) -> Self {
        Self {
            cache: TradeCache::new(),
            broadcast_mgr,
            player_mgr,
            inventory,
            item_mgr,
            allow_cross_faction_trade,
        }
    }

    // ========== PUBLIC API (called from handlers) ==========

    /// Initiate a trade with another player (CMSG_INITIATE_TRADE)
    pub async fn initiate_trade(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
    ) -> Result<(), TradeError> {
        // Validate trade conditions
        self.validate_initiate_trade(player_guid, target_guid)?;

        // Create trade in cache
        let _trade = self.cache.create_trade(player_guid, target_guid)?;

        // Send BEGIN_TRADE to target with initiator's GUID
        // The client needs the initiator's GUID to display trade request UI
        let msg = SmsgTradeStatusV2 {
            status: TradeStatus::BeginTrade,
            partner_guid: Some(player_guid),
        };
        self.broadcast_mgr.send_msg_to_player(target_guid, msg);

        tracing::debug!(
            "[TRADE] Player {:?} initiated trade with {:?}",
            player_guid,
            target_guid
        );

        Ok(())
    }

    /// Accept trade request and open window (CMSG_BEGIN_TRADE)
    pub async fn begin_trade(&self, player_guid: ObjectGuid) -> Result<(), TradeError> {
        let trade = self
            .cache
            .get_trade(player_guid)
            .ok_or(TradeError::NotInTrade)?;

        // Verify this is the target (initiator can't BEGIN_TRADE on their own request)
        if trade.is_initiator(player_guid) {
            // Initiator is waiting for target to accept
            return Err(TradeError::NotInTrade);
        }

        // Update state to Open
        trade.set_state(TradeState::Open);

        let partner_guid = trade
            .get_partner_guid(player_guid)
            .ok_or(TradeError::Internal("Partner not found".into()))?;

        // Send OPEN_WINDOW to both players
        let open_msg = SmsgTradeStatusV2 {
            status: TradeStatus::OpenWindow,
            partner_guid: None,
        };
        self.broadcast_mgr
            .send_msg_to_player(player_guid, open_msg.clone());
        self.broadcast_mgr
            .send_msg_to_player(partner_guid, open_msg);

        // Send initial trade window contents to both
        self.send_trade_update(player_guid, &trade);
        self.send_trade_update(partner_guid, &trade);

        tracing::debug!(
            "[TRADE] Trade window opened: {:?} <-> {:?}",
            player_guid,
            partner_guid
        );

        Ok(())
    }

    /// Set item in trade slot (CMSG_SET_TRADE_ITEM)
    pub async fn set_trade_item(
        &self,
        player_guid: ObjectGuid,
        trade_slot: u8,
        bag: u8,
        slot: u8,
    ) -> Result<(), TradeError> {
        // Validate slot range
        if trade_slot as usize >= TRADE_SLOT_COUNT {
            return Err(TradeError::InvalidTradeSlot);
        }

        let trade = self
            .cache
            .get_trade(player_guid)
            .ok_or(TradeError::NotInTrade)?;

        // Verify trade is open
        if trade.get_state() != TradeState::Open {
            return Err(TradeError::TradeNotOpen);
        }

        // Get item from inventory
        let item_guid = self
            .inventory
            .get_item_at(player_guid, bag, slot)
            .ok_or(TradeError::ItemNotFound)?;

        // Validate item tradeability
        self.validate_item_tradeable(player_guid, item_guid, bag, slot)?;

        // Check item not already in trade
        {
            let player_data = trade
                .get_player_data(player_guid)
                .ok_or(TradeError::Internal("Player data not found".into()))?;
            let data = player_data.read();
            if data.has_item(item_guid) {
                return Err(TradeError::ItemAlreadyInTrade);
            }
        }

        // Set item in trade data
        {
            let player_data = trade
                .get_player_data(player_guid)
                .ok_or(TradeError::Internal("Player data not found".into()))?;
            let mut data = player_data.write();
            data.set_item(trade_slot as usize, Some(item_guid));
        }

        // Reset partner's accepted flag
        self.reset_partner_accepted(&trade, player_guid);

        // Send updated trade window to both players
        let partner_guid = trade.get_partner_guid(player_guid).unwrap();
        self.send_trade_update(player_guid, &trade);
        self.send_trade_update(partner_guid, &trade);

        tracing::debug!(
            "[TRADE] Player {:?} set item {:?} in slot {}",
            player_guid,
            item_guid,
            trade_slot
        );

        Ok(())
    }

    /// Clear item from trade slot (CMSG_CLEAR_TRADE_ITEM)
    pub async fn clear_trade_item(
        &self,
        player_guid: ObjectGuid,
        trade_slot: u8,
    ) -> Result<(), TradeError> {
        if trade_slot as usize >= TRADE_SLOT_COUNT {
            return Err(TradeError::InvalidTradeSlot);
        }

        let trade = self
            .cache
            .get_trade(player_guid)
            .ok_or(TradeError::NotInTrade)?;

        // Verify trade is open
        if trade.get_state() != TradeState::Open {
            return Err(TradeError::TradeNotOpen);
        }

        // Clear item
        {
            let player_data = trade
                .get_player_data(player_guid)
                .ok_or(TradeError::Internal("Player data not found".into()))?;
            let mut data = player_data.write();
            data.clear_item(trade_slot as usize);
        }

        // Reset partner's accepted flag
        self.reset_partner_accepted(&trade, player_guid);

        // Send updates
        let partner_guid = trade.get_partner_guid(player_guid).unwrap();
        self.send_trade_update(player_guid, &trade);
        self.send_trade_update(partner_guid, &trade);

        tracing::debug!(
            "[TRADE] Player {:?} cleared slot {}",
            player_guid,
            trade_slot
        );

        Ok(())
    }

    /// Set gold amount (CMSG_SET_TRADE_GOLD)
    pub async fn set_trade_gold(
        &self,
        player_guid: ObjectGuid,
        gold: u32,
    ) -> Result<(), TradeError> {
        let trade = self
            .cache
            .get_trade(player_guid)
            .ok_or(TradeError::NotInTrade)?;

        // Verify trade is open
        if trade.get_state() != TradeState::Open {
            return Err(TradeError::TradeNotOpen);
        }

        // Validate player has enough gold
        let player_money = self
            .inventory
            .get_money(player_guid)
            .ok_or(TradeError::PlayerNotFound)?;

        if gold > player_money {
            return Err(TradeError::NotEnoughGold);
        }

        // Check gold cap for partner
        let partner_guid = trade.get_partner_guid(player_guid).unwrap();
        let partner_money = self
            .inventory
            .get_money(partner_guid)
            .ok_or(TradeError::TargetNotFound)?;

        if partner_money.saturating_add(gold) > MAX_MONEY {
            return Err(TradeError::GoldCapExceeded);
        }

        // Set gold
        {
            let player_data = trade
                .get_player_data(player_guid)
                .ok_or(TradeError::Internal("Player data not found".into()))?;
            let mut data = player_data.write();
            data.gold = gold;
            data.mark_modified();
        }

        // Reset partner's accepted flag
        self.reset_partner_accepted(&trade, player_guid);

        // Send updates
        self.send_trade_update(player_guid, &trade);
        self.send_trade_update(partner_guid, &trade);

        tracing::debug!("[TRADE] Player {:?} set gold to {}", player_guid, gold);

        Ok(())
    }

    /// Accept trade (CMSG_ACCEPT_TRADE)
    pub async fn accept_trade(&self, player_guid: ObjectGuid) -> Result<(), TradeError> {
        let trade = self
            .cache
            .get_trade(player_guid)
            .ok_or(TradeError::NotInTrade)?;

        // Verify trade is open
        if trade.get_state() != TradeState::Open {
            return Err(TradeError::TradeNotOpen);
        }

        let partner_guid = trade
            .get_partner_guid(player_guid)
            .ok_or(TradeError::Internal("Partner not found".into()))?;

        // Get player's data
        let player_data = trade.get_player_data(player_guid).unwrap();

        // Check scam prevention and processing
        {
            let data = player_data.read();
            if data.accept_process {
                return Err(TradeError::TradeAlreadyProcessing);
            }
            if !data.can_accept() {
                return Err(TradeError::ScamPreventionDelay);
            }
        }

        // Set accepted
        {
            let mut data = player_data.write();
            data.accepted = true;
        }

        // Check if both accepted
        let both_accepted = trade.both_accepted();

        if both_accepted {
            // Mark as processing to prevent double-processing
            {
                trade.initiator_data.write().accept_process = true;
                trade.target_data.write().accept_process = true;
            }

            trade.set_state(TradeState::Processing);

            // Execute trade
            match self.execute_trade(&trade, player_guid).await {
                Ok(()) => {
                    // Send completion status
                    let complete_msg = SmsgTradeStatusV2 {
                        status: TradeStatus::TradeComplete,
                        partner_guid: None,
                    };
                    self.broadcast_mgr
                        .send_msg_to_player(player_guid, complete_msg.clone());
                    self.broadcast_mgr
                        .send_msg_to_player(partner_guid, complete_msg);

                    // Remove trade from cache
                    self.cache.remove_trade(player_guid);

                    tracing::info!(
                        "[TRADE] Trade completed: {:?} <-> {:?}",
                        player_guid,
                        partner_guid
                    );
                }
                Err(e) => {
                    tracing::error!("[TRADE] Trade execution failed: {:?}", e);

                    // Reset state
                    trade.set_state(TradeState::Open);
                    trade.reset_accepted();
                    {
                        trade.initiator_data.write().accept_process = false;
                        trade.target_data.write().accept_process = false;
                    }

                    // Send cancel status
                    let cancel_msg = SmsgTradeStatusV2 {
                        status: TradeStatus::TradeCanceled,
                        partner_guid: None,
                    };
                    self.broadcast_mgr
                        .send_msg_to_player(player_guid, cancel_msg.clone());
                    self.broadcast_mgr
                        .send_msg_to_player(partner_guid, cancel_msg);

                    return Err(e);
                }
            }
        } else {
            // Notify partner that player accepted
            let accept_msg = SmsgTradeStatusV2 {
                status: TradeStatus::TradeAccept,
                partner_guid: None,
            };
            self.broadcast_mgr
                .send_msg_to_player(partner_guid, accept_msg);

            tracing::debug!("[TRADE] Player {:?} accepted trade", player_guid);
        }

        Ok(())
    }

    /// Unaccept trade (CMSG_UNACCEPT_TRADE)
    pub async fn unaccept_trade(&self, player_guid: ObjectGuid) -> Result<(), TradeError> {
        let trade = self
            .cache
            .get_trade(player_guid)
            .ok_or(TradeError::NotInTrade)?;

        {
            let player_data = trade
                .get_player_data(player_guid)
                .ok_or(TradeError::Internal("Player data not found".into()))?;
            let mut data = player_data.write();
            data.accepted = false;
        }

        let partner_guid = trade.get_partner_guid(player_guid).unwrap();
        let unaccept_msg = SmsgTradeStatusV2 {
            status: TradeStatus::BackToTrade,
            partner_guid: None,
        };
        self.broadcast_mgr
            .send_msg_to_player(partner_guid, unaccept_msg);

        tracing::debug!("[TRADE] Player {:?} unaccepted trade", player_guid);

        Ok(())
    }

    /// Cancel trade (CMSG_CANCEL_TRADE, CMSG_BUSY_TRADE, CMSG_IGNORE_TRADE)
    pub async fn cancel_trade(
        &self,
        player_guid: ObjectGuid,
        status: TradeStatus,
    ) -> Result<(), TradeError> {
        let trade = match self.cache.get_trade(player_guid) {
            Some(t) => t,
            None => return Ok(()), // Not in trade, nothing to cancel
        };

        let partner_guid = trade.get_partner_guid(player_guid);

        // Remove from cache
        self.cache.remove_trade(player_guid);

        // Notify both players
        let cancel_msg = SmsgTradeStatusV2 {
            status,
            partner_guid: None,
        };
        self.broadcast_mgr
            .send_msg_to_player(player_guid, cancel_msg.clone());
        if let Some(partner) = partner_guid {
            self.broadcast_mgr.send_msg_to_player(partner, cancel_msg);
        }

        tracing::debug!("[TRADE] Trade canceled by {:?}", player_guid);

        Ok(())
    }

    /// Send error status to player
    pub fn send_trade_error(&self, player_guid: ObjectGuid, error: TradeError) {
        let status = error.to_trade_status();
        let msg = SmsgTradeStatusV2 {
            status,
            partner_guid: None,
        };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
    }

    /// Check if player is currently trading
    pub fn is_player_trading(&self, player_guid: ObjectGuid) -> bool {
        self.cache.is_player_trading(player_guid)
    }

    // ========== PRIVATE HELPERS ==========

    /// Validate all conditions for trade initiation
    fn validate_initiate_trade(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
    ) -> Result<(), TradeError> {
        // 1. Cannot trade with self
        if player_guid == target_guid {
            return Err(TradeError::SelfTrade);
        }

        // 2. Check player exists
        if self.player_mgr.get_player(player_guid).is_none() {
            return Err(TradeError::PlayerNotFound);
        }

        // 3. Check target exists
        if self.player_mgr.get_player(target_guid).is_none() {
            return Err(TradeError::TargetNotFound);
        }

        // 4. Check neither is already trading
        if self.cache.is_player_trading(player_guid) {
            return Err(TradeError::AlreadyTrading);
        }
        if self.cache.is_player_trading(target_guid) {
            return Err(TradeError::TargetAlreadyTrading);
        }

        // 5-6. Check alive status
        // TODO: Check when health tracking is available in world

        // 7-8. Check stunned status
        // TODO: Check when aura system is available in world

        // 9-10. Check taxi status
        // TODO: Check when taxi status tracking is available

        // 11. Check faction (if cross-faction disabled)
        if !self.allow_cross_faction_trade {
            // TODO: Check team/faction when available
        }

        // 12. Check distance
        // TODO: Implement distance validation - for now we rely on client-side checks
        // Per MaNGOS: should check GetDistance3dToCenter(trader) > TRADE_DISTANCE

        // 13. Check trial account
        // TODO: Check when account type is tracked

        Ok(())
    }

    /// Validate item can be traded
    fn validate_item_tradeable(
        &self,
        _player_guid: ObjectGuid,
        item_guid: ObjectGuid,
        bag: u8,
        slot: u8,
    ) -> Result<(), TradeError> {
        // Check not bank slot (39-68)
        if bag == 255 && slot >= BANK_SLOT_START && slot <= BANK_SLOT_END {
            return Err(TradeError::BankItemNotAllowed);
        }

        // Check item flags to ensure it can be traded
        if let Some(item) = self.inventory.cache().get_item(_player_guid, item_guid) {
            let item = item.read();
            
            // Check if item is soulbound (BIND_ON_PICKUP)
            if item.flags & 0x00000001 != 0 { // ITEM_FLAG_SOULBOUND
                return Err(TradeError::ItemSoulbound);
            }
            
            // Check if item is a quest item (ITEM_FLAG_QUEST)
            if item.flags & 0x00000004 != 0 { // ITEM_FLAG_QUEST
                return Err(TradeError::ItemNotTradeable);
            }
            
            // Check if item is conjured (ITEM_FLAG_CONJURED)
            // Conjured items disappear when logged out, so shouldn't be traded
            if item.flags & 0x00000002 != 0 { // ITEM_FLAG_CONJURED
                return Err(TradeError::ItemNotTradeable);
            }
        }

        Ok(())
    }

    /// Execute the trade transaction
    async fn execute_trade(
        &self,
        trade: &Arc<ActiveTrade>,
        _player_guid: ObjectGuid,
    ) -> Result<(), TradeError> {
        let initiator_guid = trade.initiator_guid;
        let target_guid = trade.target_guid;

        let initiator_gold;
        let target_gold;
        let initiator_items: Vec<(usize, ObjectGuid)>;
        let target_items: Vec<(usize, ObjectGuid)>;

        // Collect trade data under read locks
        {
            let initiator_data = trade.initiator_data.read();
            let target_data = trade.target_data.read();

            initiator_gold = initiator_data.gold;
            target_gold = target_data.gold;

            // Collect items (excluding non-traded slot)
            initiator_items = initiator_data
                .items
                .iter()
                .take(TRADE_SLOT_TRADED_COUNT)
                .enumerate()
                .filter_map(|(i, opt)| opt.map(|g| (i, g)))
                .collect();

            target_items = target_data
                .items
                .iter()
                .take(TRADE_SLOT_TRADED_COUNT)
                .enumerate()
                .filter_map(|(i, opt)| opt.map(|g| (i, g)))
                .collect();
        }

        // Validate final conditions
        self.validate_trade_completion(
            initiator_guid,
            target_guid,
            initiator_gold,
            target_gold,
            &initiator_items,
            &target_items,
        )?;

        tracing::info!(
            "[TRADE] Executing trade: initiator={:?} ({} gold, {} items) <-> target={:?} ({} gold, {} items)",
            initiator_guid, initiator_gold, initiator_items.len(),
            target_guid, target_gold, target_items.len()
        );

        // Execute gold transfer with proper error handling
        if initiator_gold > 0 {
            tracing::info!("[TRADE] Transferring {} gold from {:?} to {:?}", initiator_gold, initiator_guid, target_guid);
            // Remove gold from initiator
            match self.inventory.remove_gold(initiator_guid, initiator_gold) {
                crate::world::game::inventory::GoldResult::Success { .. } => {}
                crate::world::game::inventory::GoldResult::InsufficientFunds => {
                    tracing::error!(
                        "[TRADE] Initiator {:?} insufficient funds during trade execution",
                        initiator_guid
                    );
                    return Err(TradeError::GoldChangedDuringTrade);
                }
                crate::world::game::inventory::GoldResult::DatabaseError(e) => {
                    tracing::error!(
                        "[TRADE] DB error removing initiator gold: {}",
                        e
                    );
                    return Err(TradeError::Internal(e));
                }
                _ => {
                    tracing::error!("[TRADE] Failed to remove initiator gold");
                    return Err(TradeError::Internal("Failed to remove gold".into()));
                }
            }

            // Add gold to target
            match self.inventory.add_gold(target_guid, initiator_gold) {
                crate::world::game::inventory::GoldResult::Success { .. } => {
                    tracing::info!("[TRADE] Successfully transferred {} gold from {:?} to {:?}", initiator_gold, initiator_guid, target_guid);
                }
                crate::world::game::inventory::GoldResult::CapExceeded => {
                    tracing::error!(
                        "[TRADE] Target {:?} gold cap exceeded, rolling back",
                        target_guid
                    );
                    // CRITICAL: Rollback - return gold to initiator
                    let _ = self.inventory.add_gold(initiator_guid, initiator_gold);
                    return Err(TradeError::GoldCapExceeded);
                }
                crate::world::game::inventory::GoldResult::DatabaseError(e) => {
                    tracing::error!(
                        "[TRADE] DB error adding target gold: {}, rolling back",
                        e
                    );
                    // CRITICAL: Rollback - return gold to initiator
                    let _ = self.inventory.add_gold(initiator_guid, initiator_gold);
                    return Err(TradeError::Internal(e));
                }
                _ => {
                    tracing::error!("[TRADE] Failed to add target gold, rolling back");
                    let _ = self.inventory.add_gold(initiator_guid, initiator_gold);
                    return Err(TradeError::Internal("Failed to add gold".into()));
                }
            }
        }

        if target_gold > 0 {
            tracing::info!("[TRADE] Transferring {} gold from {:?} to {:?}", target_gold, target_guid, initiator_guid);
            // Remove gold from target
            match self.inventory.remove_gold(target_guid, target_gold) {
                crate::world::game::inventory::GoldResult::Success { .. } => {}
                crate::world::game::inventory::GoldResult::InsufficientFunds => {
                    tracing::error!(
                        "[TRADE] Target {:?} insufficient funds during trade execution",
                        target_guid
                    );
                    // Rollback initiator's gold if it was transferred
                    if initiator_gold > 0 {
                        let _ = self.inventory.remove_gold(target_guid, initiator_gold);
                        let _ = self.inventory.add_gold(initiator_guid, initiator_gold);
                    }
                    return Err(TradeError::GoldChangedDuringTrade);
                }
                crate::world::game::inventory::GoldResult::DatabaseError(e) => {
                    tracing::error!(
                        "[TRADE] DB error removing target gold: {}, rolling back",
                        e
                    );
                    // Rollback initiator's gold if it was transferred
                    if initiator_gold > 0 {
                        let _ = self.inventory.remove_gold(target_guid, initiator_gold);
                        let _ = self.inventory.add_gold(initiator_guid, initiator_gold);
                    }
                    return Err(TradeError::Internal(e));
                }
                _ => {
                    tracing::error!("[TRADE] Failed to remove target gold, rolling back");
                    if initiator_gold > 0 {
                        let _ = self.inventory.remove_gold(target_guid, initiator_gold);
                        let _ = self.inventory.add_gold(initiator_guid, initiator_gold);
                    }
                    return Err(TradeError::Internal("Failed to remove gold".into()));
                }
            }

            // Add gold to initiator
            match self.inventory.add_gold(initiator_guid, target_gold) {
                crate::world::game::inventory::GoldResult::Success { .. } => {}
                crate::world::game::inventory::GoldResult::CapExceeded => {
                    tracing::error!(
                        "[TRADE] Initiator {:?} gold cap exceeded, rolling back all",
                        initiator_guid
                    );
                    // CRITICAL: Rollback everything
                    let _ = self.inventory.add_gold(target_guid, target_gold);
                    if initiator_gold > 0 {
                        let _ = self.inventory.remove_gold(target_guid, initiator_gold);
                        let _ = self.inventory.add_gold(initiator_guid, initiator_gold);
                    }
                    return Err(TradeError::GoldCapExceeded);
                }
                crate::world::game::inventory::GoldResult::DatabaseError(e) => {
                    tracing::error!(
                        "[TRADE] DB error adding initiator gold: {}, rolling back all",
                        e
                    );
                    // CRITICAL: Rollback everything
                    let _ = self.inventory.add_gold(target_guid, target_gold);
                    if initiator_gold > 0 {
                        let _ = self.inventory.remove_gold(target_guid, initiator_gold);
                        let _ = self.inventory.add_gold(initiator_guid, initiator_gold);
                    }
                    return Err(TradeError::Internal(e));
                }
                _ => {
                    tracing::error!("[TRADE] Failed to add initiator gold, rolling back all");
                    let _ = self.inventory.add_gold(target_guid, target_gold);
                    if initiator_gold > 0 {
                        let _ = self.inventory.remove_gold(target_guid, initiator_gold);
                        let _ = self.inventory.add_gold(initiator_guid, initiator_gold);
                    }
                    return Err(TradeError::Internal("Failed to add gold".into()));
                }
            }
        }

        // Execute item transfer - initiator's items to target
        tracing::info!("[TRADE] Transferring {} items from {:?} to {:?}", initiator_items.len(), initiator_guid, target_guid);
        let mut transferred_items: Vec<(ObjectGuid, ObjectGuid)> = Vec::new(); // (from_player, item_guid) pairs
        
        for (_slot, item_guid) in &initiator_items {
            tracing::info!("[TRADE] Transferring item {:?} from {:?} to {:?}", item_guid, initiator_guid, target_guid);
            match self
                .inventory
                .transfer_item(initiator_guid, target_guid, *item_guid).await

            {
                crate::world::game::inventory::TransferItemResult::Success { .. } => {
                    tracing::info!(
                        "[TRADE] Transferred item {:?} from {:?} to {:?}",
                        item_guid,
                        initiator_guid,
                        target_guid
                    );
                    transferred_items.push((initiator_guid, *item_guid));
                }
                crate::world::game::inventory::TransferItemResult::TargetInventoryFull => {
                    tracing::error!(
                        "[TRADE] Target inventory full during item transfer, rolling back {} transferred items",
                        transferred_items.len()
                    );
                    // CRITICAL: Rollback all already-transferred items
                    self.rollback_trade(&transferred_items, initiator_gold, target_gold, initiator_guid, target_guid);
                    return Err(TradeError::TargetInventoryFull);
                }
                crate::world::game::inventory::TransferItemResult::ItemNotFound => {
                    tracing::error!(
                        "[TRADE] Item {:?} disappeared during trade execution, rolling back",
                        item_guid
                    );
                    self.rollback_trade(&transferred_items, initiator_gold, target_gold, initiator_guid, target_guid);
                    return Err(TradeError::ItemDisappeared);
                }
                crate::world::game::inventory::TransferItemResult::DatabaseError(e) => {
                    tracing::error!("[TRADE] DB error transferring item: {}, rolling back", e);
                    self.rollback_trade(&transferred_items, initiator_gold, target_gold, initiator_guid, target_guid);
                    return Err(TradeError::Internal(e));
                }
                _ => {
                    tracing::error!("[TRADE] Failed to transfer item, rolling back");
                    self.rollback_trade(&transferred_items, initiator_gold, target_gold, initiator_guid, target_guid);
                    return Err(TradeError::Internal("Item transfer failed".into()));
                }
            }
        }

        // Execute item transfer - target's items to initiator
        tracing::info!("[TRADE] Transferring {} items from {:?} to {:?}", target_items.len(), target_guid, initiator_guid);
        for (_slot, item_guid) in &target_items {
            tracing::info!("[TRADE] Transferring item {:?} from {:?} to {:?}", item_guid, target_guid, initiator_guid);
            match self
                .inventory
                .transfer_item(target_guid, initiator_guid, *item_guid).await

            {
                crate::world::game::inventory::TransferItemResult::Success { .. } => {
                    tracing::info!(
                        "[TRADE] Transferred item {:?} from {:?} to {:?}",
                        item_guid,
                        target_guid,
                        initiator_guid
                    );
                    transferred_items.push((target_guid, *item_guid));
                }
                crate::world::game::inventory::TransferItemResult::TargetInventoryFull => {
                    tracing::error!(
                        "[TRADE] Initiator inventory full during item transfer, rolling back {} transferred items",
                        transferred_items.len()
                    );
                    self.rollback_trade(&transferred_items, initiator_gold, target_gold, initiator_guid, target_guid);
                    return Err(TradeError::PlayerInventoryFull);
                }
                crate::world::game::inventory::TransferItemResult::ItemNotFound => {
                    tracing::error!(
                        "[TRADE] Item {:?} disappeared during trade execution, rolling back",
                        item_guid
                    );
                    self.rollback_trade(&transferred_items, initiator_gold, target_gold, initiator_guid, target_guid);
                    return Err(TradeError::ItemDisappeared);
                }
                crate::world::game::inventory::TransferItemResult::DatabaseError(e) => {
                    tracing::error!("[TRADE] DB error transferring item: {}, rolling back", e);
                    self.rollback_trade(&transferred_items, initiator_gold, target_gold, initiator_guid, target_guid);
                    return Err(TradeError::Internal(e));
                }
                _ => {
                    tracing::error!("[TRADE] Failed to transfer item, rolling back");
                    self.rollback_trade(&transferred_items, initiator_gold, target_gold, initiator_guid, target_guid);
                    return Err(TradeError::Internal("Item transfer failed".into()));
                }
            }
        }

        // TODO: Cast enchantment spells on non-traded slot items (for enchanting profession)

        // Send full inventory refresh to both players to ensure UI is in sync
        // This fixes the "items stuck in move mode" bug after trades
        tracing::debug!("[TRADE] Sending inventory refresh to both players");
        self.inventory.send_player_inventory(initiator_guid);
        self.inventory.send_player_inventory(target_guid);

        Ok(())
    }

    /// Validate trade can complete
    fn validate_trade_completion(
        &self,
        initiator_guid: ObjectGuid,
        target_guid: ObjectGuid,
        initiator_gold: u32,
        target_gold: u32,
        initiator_items: &[(usize, ObjectGuid)],
        target_items: &[(usize, ObjectGuid)],
    ) -> Result<(), TradeError> {
        // 1. Verify gold amounts still valid
        let initiator_money = self
            .inventory
            .get_money(initiator_guid)
            .ok_or(TradeError::PlayerNotFound)?;
        let target_money = self
            .inventory
            .get_money(target_guid)
            .ok_or(TradeError::TargetNotFound)?;

        if initiator_gold > initiator_money {
            return Err(TradeError::GoldChangedDuringTrade);
        }
        if target_gold > target_money {
            return Err(TradeError::GoldChangedDuringTrade);
        }

        // 2. Check gold cap not exceeded
        let initiator_final = initiator_money - initiator_gold + target_gold;
        let target_final = target_money - target_gold + initiator_gold;

        if initiator_final > MAX_MONEY || target_final > MAX_MONEY {
            return Err(TradeError::GoldCapExceeded);
        }

        // 3. Verify inventory space for receiving items
        let initiator_receiving = target_items.len() as u32;
        let target_receiving = initiator_items.len() as u32;

        if initiator_receiving > 0 && !self.inventory.has_free_slots(initiator_guid, initiator_receiving) {
            return Err(TradeError::PlayerInventoryFull);
        }
        if target_receiving > 0 && !self.inventory.has_free_slots(target_guid, target_receiving) {
            return Err(TradeError::TargetInventoryFull);
        }

        // 4. Verify items still exist and are tradeable
        // Check all items from initiator
        for (_, item_guid) in initiator_items {
            if let Some(item) = self.inventory.cache().get_item(initiator_guid, *item_guid) {
                let item = item.read();
                if item.flags & 0x00000001 != 0 { // ITEM_FLAG_SOULBOUND
                    return Err(TradeError::ItemSoulbound);
                }
                if item.flags & 0x00000004 != 0 { // ITEM_FLAG_QUEST
                    return Err(TradeError::ItemNotTradeable);
                }
            } else {
                return Err(TradeError::ItemDisappeared);
            }
        }
        
        // Check all items from target
        for (_, item_guid) in target_items {
            if let Some(item) = self.inventory.cache().get_item(target_guid, *item_guid) {
                let item = item.read();
                if item.flags & 0x00000001 != 0 { // ITEM_FLAG_SOULBOUND
                    return Err(TradeError::ItemSoulbound);
                }
                if item.flags & 0x00000004 != 0 { // ITEM_FLAG_QUEST
                    return Err(TradeError::ItemNotTradeable);
                }
            } else {
                return Err(TradeError::ItemDisappeared);
            }
        }

        Ok(())
    }

    /// Rollback a partially completed trade
    /// This is called when a trade fails after some items have been transferred
    async fn rollback_trade(
        &self,
        transferred_items: &[(ObjectGuid, ObjectGuid)],
        initiator_gold: u32,
        target_gold: u32,
        initiator_guid: ObjectGuid,
        target_guid: ObjectGuid,
    ) {
        tracing::error!("[TRADE] Rolling back trade: {} items transferred back", transferred_items.len());

        // Rollback gold transfers
        if initiator_gold > 0 {
            tracing::info!("[TRADE] Rolling back {} gold from initiator transfer", initiator_gold);
            // Remove gold from target (who received it)
            match self.inventory.remove_gold(target_guid, initiator_gold) {
                crate::world::game::inventory::GoldResult::Success { .. } => {
                    // Return gold to initiator
                    match self.inventory.add_gold(initiator_guid, initiator_gold) {
                        crate::world::game::inventory::GoldResult::Success { .. } => {
                            tracing::info!("[TRADE] Successfully rolled back {} gold to initiator", initiator_gold);
                        }
                        _ => {
                            tracing::error!("[TRADE] CRITICAL: Failed to return {} gold to initiator!", initiator_gold);
                        }
                    }
                }
                _ => {
                    tracing::error!("[TRADE] CRITICAL: Failed to remove {} gold from target for rollback!", initiator_gold);
                }
            }
        }

        if target_gold > 0 {
            tracing::info!("[TRADE] Rolling back {} gold from target transfer", target_gold);
            // Remove gold from initiator (who received it)
            match self.inventory.remove_gold(initiator_guid, target_gold) {
                crate::world::game::inventory::GoldResult::Success { .. } => {
                    // Return gold to target
                    match self.inventory.add_gold(target_guid, target_gold) {
                        crate::world::game::inventory::GoldResult::Success { .. } => {
                            tracing::info!("[TRADE] Successfully rolled back {} gold to target", target_gold);
                        }
                        _ => {
                            tracing::error!("[TRADE] CRITICAL: Failed to return {} gold to target!", target_gold);
                        }
                    }
                }
                _ => {
                    tracing::error!("[TRADE] CRITICAL: Failed to remove {} gold from initiator for rollback!", target_gold);
                }
            }
        }

        // Rollback item transfers (reverse direction)
        for (from_player, item_guid) in transferred_items {
            let to_player = if *from_player == initiator_guid {
                target_guid
            } else {
                initiator_guid
            };

            tracing::info!("[TRADE] Rolling back item {:?} from {:?} back to {:?}", item_guid, to_player, from_player);
            
            match self.inventory.transfer_item(to_player, *from_player, *item_guid).await {
                crate::world::game::inventory::TransferItemResult::Success { .. } => {
                    tracing::info!("[TRADE] Successfully rolled back item {:?}", item_guid);
                }
                crate::world::game::inventory::TransferItemResult::TargetInventoryFull => {
                    // This shouldn't happen since we just removed the item from the source
                    tracing::error!("[TRADE] CRITICAL: Target inventory full during rollback of item {:?}!", item_guid);
                }
                e => {
                    tracing::error!("[TRADE] CRITICAL: Failed to rollback item {:?}: {:?}", item_guid, e);
                }
            }
        }

        // Send inventory refreshes to both players
        self.inventory.send_player_inventory(initiator_guid);
        self.inventory.send_player_inventory(target_guid);

        tracing::error!("[TRADE] Rollback completed");
    }

    /// Reset partner's accepted flag when trade is modified
    fn reset_partner_accepted(&self, trade: &Arc<ActiveTrade>, player_guid: ObjectGuid) {
        if let Some(partner_data) = trade.get_partner_data(player_guid) {
            let mut data = partner_data.write();
            data.accepted = false;
        }
    }

    /// Send trade window update to a player
    fn send_trade_update(&self, player_guid: ObjectGuid, trade: &Arc<ActiveTrade>) {
        // Send player's own trade data (is_trader_view = false)
        if let Some(player_data) = trade.get_player_data(player_guid) {
            let data = player_data.read();
            let msg = self.build_trade_extended_msg(player_guid, &data, false);
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
        }

        // Send partner's trade data (is_trader_view = true)
        if let Some(partner_guid) = trade.get_partner_guid(player_guid) {
            if let Some(partner_data) = trade.get_partner_data(player_guid) {
                let data = partner_data.read();
                // IMPORTANT: Use partner_guid to look up items from partner's inventory
                let msg = self.build_trade_extended_msg(partner_guid, &data, true);
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            }
        }
    }

    /// Build SMSG_TRADE_STATUS_EXTENDED message
    fn build_trade_extended_msg(
        &self,
        owner_guid: ObjectGuid,
        data: &TradeData,
        is_trader_view: bool,
    ) -> SmsgTradeStatusExtendedV2 {
        let mut slots: [Option<TradeSlotInfoV2>; TRADE_SLOT_COUNT] = Default::default();

        for (i, item_opt) in data.items.iter().enumerate() {
            if let Some(item_guid) = item_opt {
                // Get item from owner's inventory cache
                if let Some(item_arc) = self.inventory.cache().get_item(owner_guid, *item_guid) {
                    let item = item_arc.read();

                    // Get template for display_id
                    let display_id = self
                        .item_mgr
                        .get_template(item.entry)
                        .map(|t| t.display_id)
                        .unwrap_or(item.entry); // Fallback to entry if template not found

                    // Get first enchantment (permanent enchant)
                    let permanent_enchant = item
                        .enchantments
                        .first()
                        .map(|(id, _, _)| *id)
                        .unwrap_or(0);

                    // Find first non-zero spell charge
                    let charges = item
                        .spell_charges
                        .iter()
                        .find(|&&c| c != 0)
                        .copied()
                        .unwrap_or(0);

                    slots[i] = Some(TradeSlotInfoV2 {
                        slot_index: i as u8,
                        item_entry: item.entry,
                        display_id,
                        count: item.count,
                        wrapped: false, // Gift wrapping not implemented in v2 yet
                        gift_creator_guid: item.gift_creator_guid.unwrap_or_else(ObjectGuid::empty),
                        permanent_enchant,
                        creator_guid: item.creator_guid.unwrap_or_else(ObjectGuid::empty),
                        charges,
                        suffix_factor: 0, // Suffix factor not implemented yet
                        random_property_id: item.random_property_id,
                        lock_id: 0,      // Lock ID not used in vanilla
                        max_durability: item.max_durability,
                        durability: item.durability,
                    });
                } else {
                    // Item not found in cache - log warning
                    tracing::warn!(
                        "[TRADE] Item {:?} not found in cache for player {:?}",
                        item_guid,
                        owner_guid
                    );
                }
            }
        }

        SmsgTradeStatusExtendedV2 {
            is_trader_view,
            trade_slots: slots,
            gold: data.gold,
            spell_id: data.spell_id,
        }
    }
}

// ========== LIFECYCLE METHODS ==========

impl TradeSystem {
    pub async fn init(&self) -> Result<()> {

        Ok(())
    }

    pub fn update(&self, _diff: std::time::Duration) -> Result<()> {
        // TODO: Optionally check for stale/timed-out trades
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.cache.clear();

        Ok(())
    }

    pub fn on_player_logout(&self, guid: ObjectGuid) -> Result<()> {
        // Cancel any active trade on logout
        if let Some(trade) = self.cache.get_trade(guid) {
            let partner_guid = trade.get_partner_guid(guid);
            self.cache.remove_trade(guid);

            // Notify partner
            if let Some(partner) = partner_guid {
                let logout_msg = SmsgTradeStatusV2 {
                    status: TradeStatus::TargetLogout,
                    partner_guid: None,
                };
                let broadcast_mgr = Arc::clone(&self.broadcast_mgr);
                tokio::spawn(async move {
                    broadcast_mgr.send_msg_to_player(partner, logout_msg);
                });
            }

            tracing::debug!("[TRADE] Trade canceled due to logout: {:?}", guid);
        }

        Ok(())
    }
}

