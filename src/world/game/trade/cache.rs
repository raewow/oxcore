//! Trade cache - manages active trade sessions

use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Instant;

use crate::shared::protocol::ObjectGuid;

use super::types::{TradeData, TradeError, TradeState};

/// Active trade session between two players
#[derive(Debug)]
pub struct ActiveTrade {
    /// Player who initiated the trade
    pub initiator_guid: ObjectGuid,
    /// Target player who received the trade request
    pub target_guid: ObjectGuid,
    /// Initiator's trade data
    pub initiator_data: RwLock<TradeData>,
    /// Target's trade data
    pub target_data: RwLock<TradeData>,
    /// Current trade state
    pub state: RwLock<TradeState>,
    /// Creation timestamp (for potential timeout)
    pub created_at: Instant,
}

impl ActiveTrade {
    /// Create a new active trade session
    pub fn new(initiator_guid: ObjectGuid, target_guid: ObjectGuid) -> Self {
        Self {
            initiator_guid,
            target_guid,
            initiator_data: RwLock::new(TradeData::new()),
            target_data: RwLock::new(TradeData::new()),
            state: RwLock::new(TradeState::Initiated),
            created_at: Instant::now(),
        }
    }

    /// Get the partner's GUID for a given player
    pub fn get_partner_guid(&self, player_guid: ObjectGuid) -> Option<ObjectGuid> {
        if player_guid == self.initiator_guid {
            Some(self.target_guid)
        } else if player_guid == self.target_guid {
            Some(self.initiator_guid)
        } else {
            None
        }
    }

    /// Check if a player is part of this trade
    pub fn involves_player(&self, player_guid: ObjectGuid) -> bool {
        player_guid == self.initiator_guid || player_guid == self.target_guid
    }

    /// Check if the given player is the initiator
    pub fn is_initiator(&self, player_guid: ObjectGuid) -> bool {
        player_guid == self.initiator_guid
    }

    /// Check if the given player is the target
    pub fn is_target(&self, player_guid: ObjectGuid) -> bool {
        player_guid == self.target_guid
    }

    /// Get player's own trade data (read lock)
    pub fn get_player_data(&self, player_guid: ObjectGuid) -> Option<&RwLock<TradeData>> {
        if player_guid == self.initiator_guid {
            Some(&self.initiator_data)
        } else if player_guid == self.target_guid {
            Some(&self.target_data)
        } else {
            None
        }
    }

    /// Get partner's trade data (read lock)
    pub fn get_partner_data(&self, player_guid: ObjectGuid) -> Option<&RwLock<TradeData>> {
        if player_guid == self.initiator_guid {
            Some(&self.target_data)
        } else if player_guid == self.target_guid {
            Some(&self.initiator_data)
        } else {
            None
        }
    }

    /// Get current trade state
    pub fn get_state(&self) -> TradeState {
        *self.state.read()
    }

    /// Set trade state
    pub fn set_state(&self, new_state: TradeState) {
        *self.state.write() = new_state;
    }

    /// Check if both players have accepted
    pub fn both_accepted(&self) -> bool {
        let initiator = self.initiator_data.read();
        let target = self.target_data.read();
        initiator.accepted && target.accepted
    }

    /// Reset both players' accepted flags
    pub fn reset_accepted(&self) {
        self.initiator_data.write().accepted = false;
        self.target_data.write().accepted = false;
    }

    /// Duration since trade was created
    pub fn age(&self) -> std::time::Duration {
        self.created_at.elapsed()
    }
}

/// Trade cache - manages all active trades indexed by player GUID
///
/// Each trade is stored twice (once per player) pointing to the same Arc<ActiveTrade>
pub struct TradeCache {
    /// Map of player GUID -> active trade (each trade stored under both player GUIDs)
    trades: DashMap<ObjectGuid, Arc<ActiveTrade>>,
}

impl TradeCache {
    /// Create a new empty trade cache
    pub fn new() -> Self {
        Self {
            trades: DashMap::new(),
        }
    }

    /// Create a new trade between two players
    ///
    /// Returns error if either player is already trading
    pub fn create_trade(
        &self,
        initiator_guid: ObjectGuid,
        target_guid: ObjectGuid,
    ) -> Result<Arc<ActiveTrade>, TradeError> {
        // Check if either player is already trading
        if self.trades.contains_key(&initiator_guid) {
            return Err(TradeError::AlreadyTrading);
        }
        if self.trades.contains_key(&target_guid) {
            return Err(TradeError::TargetAlreadyTrading);
        }

        // Create trade and store under both GUIDs
        let trade = Arc::new(ActiveTrade::new(initiator_guid, target_guid));
        self.trades.insert(initiator_guid, Arc::clone(&trade));
        self.trades.insert(target_guid, Arc::clone(&trade));

        Ok(trade)
    }

    /// Get active trade for a player
    pub fn get_trade(&self, player_guid: ObjectGuid) -> Option<Arc<ActiveTrade>> {
        self.trades.get(&player_guid).map(|r| Arc::clone(&r))
    }

    /// Remove trade (cleans up both player entries)
    pub fn remove_trade(&self, player_guid: ObjectGuid) {
        if let Some((_, trade)) = self.trades.remove(&player_guid) {
            // Remove the other player's entry too
            if let Some(other_guid) = trade.get_partner_guid(player_guid) {
                self.trades.remove(&other_guid);
            }
        }
    }

    /// Check if a player is currently in a trade
    pub fn is_player_trading(&self, player_guid: ObjectGuid) -> bool {
        self.trades.contains_key(&player_guid)
    }

    /// Get count of active trades (actual count is half of entries since each trade has 2 entries)
    pub fn trade_count(&self) -> usize {
        self.trades.len() / 2
    }

    /// Clear all trades (for shutdown)
    pub fn clear(&self) {
        self.trades.clear();
    }

    /// Get all active trade session for iteration (returns unique trades)
    pub fn get_all_trades(&self) -> Vec<Arc<ActiveTrade>> {
        let mut seen = std::collections::HashSet::new();
        let mut trades = Vec::new();

        for entry in self.trades.iter() {
            let trade = entry.value();
            // Only add each trade once (check by initiator GUID)
            if seen.insert(trade.initiator_guid) {
                trades.push(Arc::clone(trade));
            }
        }

        trades
    }
}

impl Default for TradeCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::protocol::HighGuid;

    fn test_guid(low: u32) -> ObjectGuid {
        ObjectGuid::new_without_entry(HighGuid::Player, low)
    }

    #[test]
    fn test_active_trade_new() {
        let player1 = test_guid(1);
        let player2 = test_guid(2);
        let trade = ActiveTrade::new(player1, player2);

        assert_eq!(trade.initiator_guid, player1);
        assert_eq!(trade.target_guid, player2);
        assert_eq!(trade.get_state(), TradeState::Initiated);
    }

    #[test]
    fn test_active_trade_partner_guid() {
        let player1 = test_guid(1);
        let player2 = test_guid(2);
        let trade = ActiveTrade::new(player1, player2);

        assert_eq!(trade.get_partner_guid(player1), Some(player2));
        assert_eq!(trade.get_partner_guid(player2), Some(player1));
        assert_eq!(trade.get_partner_guid(test_guid(3)), None);
    }

    #[test]
    fn test_active_trade_involves_player() {
        let player1 = test_guid(1);
        let player2 = test_guid(2);
        let trade = ActiveTrade::new(player1, player2);

        assert!(trade.involves_player(player1));
        assert!(trade.involves_player(player2));
        assert!(!trade.involves_player(test_guid(3)));
    }

    #[test]
    fn test_active_trade_data_access() {
        let player1 = test_guid(1);
        let player2 = test_guid(2);
        let trade = ActiveTrade::new(player1, player2);

        // Player 1's data should be initiator_data
        assert!(trade.get_player_data(player1).is_some());
        assert!(trade.get_partner_data(player1).is_some());

        // Unknown player should get None
        assert!(trade.get_player_data(test_guid(3)).is_none());
    }

    #[test]
    fn test_active_trade_both_accepted() {
        let player1 = test_guid(1);
        let player2 = test_guid(2);
        let trade = ActiveTrade::new(player1, player2);

        assert!(!trade.both_accepted());

        trade.initiator_data.write().accepted = true;
        assert!(!trade.both_accepted());

        trade.target_data.write().accepted = true;
        assert!(trade.both_accepted());
    }

    #[test]
    fn test_trade_cache_create() {
        let cache = TradeCache::new();
        let player1 = test_guid(1);
        let player2 = test_guid(2);

        let trade = cache.create_trade(player1, player2).unwrap();
        assert_eq!(trade.initiator_guid, player1);
        assert_eq!(cache.trade_count(), 1);
    }

    #[test]
    fn test_trade_cache_already_trading() {
        let cache = TradeCache::new();
        let player1 = test_guid(1);
        let player2 = test_guid(2);
        let player3 = test_guid(3);

        cache.create_trade(player1, player2).unwrap();

        // Player 1 trying to start another trade
        let result = cache.create_trade(player1, player3);
        assert!(matches!(result, Err(TradeError::AlreadyTrading)));

        // Player 3 trying to trade with player 2 who is already trading
        let result = cache.create_trade(player3, player2);
        assert!(matches!(result, Err(TradeError::TargetAlreadyTrading)));
    }

    #[test]
    fn test_trade_cache_get_trade() {
        let cache = TradeCache::new();
        let player1 = test_guid(1);
        let player2 = test_guid(2);

        cache.create_trade(player1, player2).unwrap();

        // Both players should be able to get the trade
        let trade1 = cache.get_trade(player1);
        let trade2 = cache.get_trade(player2);

        assert!(trade1.is_some());
        assert!(trade2.is_some());

        // Should be the same trade
        assert!(Arc::ptr_eq(&trade1.unwrap(), &trade2.unwrap()));
    }

    #[test]
    fn test_trade_cache_remove() {
        let cache = TradeCache::new();
        let player1 = test_guid(1);
        let player2 = test_guid(2);

        cache.create_trade(player1, player2).unwrap();
        assert!(cache.is_player_trading(player1));
        assert!(cache.is_player_trading(player2));

        // Remove via player1 should also remove player2's entry
        cache.remove_trade(player1);
        assert!(!cache.is_player_trading(player1));
        assert!(!cache.is_player_trading(player2));
        assert_eq!(cache.trade_count(), 0);
    }

    #[test]
    fn test_trade_cache_clear() {
        let cache = TradeCache::new();
        cache.create_trade(test_guid(1), test_guid(2)).unwrap();
        cache.create_trade(test_guid(3), test_guid(4)).unwrap();

        assert_eq!(cache.trade_count(), 2);

        cache.clear();
        assert_eq!(cache.trade_count(), 0);
    }
}
