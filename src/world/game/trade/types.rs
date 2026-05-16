//! Trade system types for world

use crate::shared::protocol::ObjectGuid;
use std::time::Instant;

// Re-export constants from shared
pub use crate::shared::game::trade::{
    TRADE_DISTANCE_METERS, TRADE_DISTANCE_YARDS, TRADE_SCAM_PREVENTION_DELAY_MS, TRADE_SLOT_COUNT,
    TRADE_SLOT_INVALID, TRADE_SLOT_NONTRADED, TRADE_SLOT_TRADED_COUNT,
};

// Re-export TradeStatus from shared so all crate code uses the same type
pub use crate::shared::game::trade::TradeStatus;

// ========== ADDITIONAL CONSTANTS ==========

/// Maximum gold cap (2,147,483,647 copper = ~214k gold)
pub const MAX_MONEY: u32 = 0x7FFFFFFF;

/// Bank slot range start (slots 39-68 are bank slots)
pub const BANK_SLOT_START: u8 = 39;

/// Bank slot range end
pub const BANK_SLOT_END: u8 = 68;

// ========== TRADE STATE ==========

/// Trade session state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeState {
    /// Request sent, waiting for target to accept
    Initiated,
    /// Both parties accepted, trade window is open
    Open,
    /// Trade is being completed (both accepted)
    Processing,
    /// Trade finished (completed or cancelled)
    Closed,
}

impl Default for TradeState {
    fn default() -> Self {
        Self::Initiated
    }
}

// ========== TRADE ERROR ==========

/// Trade operation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeError {
    // Initiation errors
    PlayerNotFound,
    TargetNotFound,
    SelfTrade,
    AlreadyTrading,
    TargetAlreadyTrading,
    PlayerDead,
    TargetDead,
    PlayerStunned,
    TargetStunned,
    PlayerLoggingOut,
    TargetLoggingOut,
    PlayerInTaxi,
    TargetInTaxi,
    WrongFaction,
    TargetTooFar,
    TrialAccountRestricted,
    TargetIgnoringPlayer,

    // Trade operation errors
    NotInTrade,
    TradeNotOpen,
    InvalidTradeSlot,
    ItemNotFound,
    ItemNotTradeable,
    ItemSoulbound,
    ItemAlreadyInTrade,
    BankItemNotAllowed,
    NotEnoughGold,
    GoldCapExceeded,

    // Completion errors
    ScamPreventionDelay,
    TradeAlreadyProcessing,
    ItemDisappeared,
    GoldChangedDuringTrade,
    PlayerInventoryFull,
    TargetInventoryFull,
    SpellCastFailed,

    // Internal errors
    Internal(String),
}

impl std::fmt::Display for TradeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlayerNotFound => write!(f, "Player not found"),
            Self::TargetNotFound => write!(f, "Target not found"),
            Self::SelfTrade => write!(f, "Cannot trade with yourself"),
            Self::AlreadyTrading => write!(f, "Already in a trade"),
            Self::TargetAlreadyTrading => write!(f, "Target is already trading"),
            Self::PlayerDead => write!(f, "Cannot trade while dead"),
            Self::TargetDead => write!(f, "Target is dead"),
            Self::PlayerStunned => write!(f, "Cannot trade while stunned"),
            Self::TargetStunned => write!(f, "Target is stunned"),
            Self::PlayerLoggingOut => write!(f, "Cannot trade while logging out"),
            Self::TargetLoggingOut => write!(f, "Target is logging out"),
            Self::PlayerInTaxi => write!(f, "Cannot trade while on taxi"),
            Self::TargetInTaxi => write!(f, "Target is on taxi"),
            Self::WrongFaction => write!(f, "Cannot trade with enemy faction"),
            Self::TargetTooFar => write!(f, "Target is too far away"),
            Self::TrialAccountRestricted => write!(f, "Trial account restriction"),
            Self::TargetIgnoringPlayer => write!(f, "Target is ignoring you"),
            Self::NotInTrade => write!(f, "Not in a trade"),
            Self::TradeNotOpen => write!(f, "Trade window is not open"),
            Self::InvalidTradeSlot => write!(f, "Invalid trade slot"),
            Self::ItemNotFound => write!(f, "Item not found"),
            Self::ItemNotTradeable => write!(f, "Item cannot be traded"),
            Self::ItemSoulbound => write!(f, "Soulbound items cannot be traded"),
            Self::ItemAlreadyInTrade => write!(f, "Item is already in trade"),
            Self::BankItemNotAllowed => write!(f, "Cannot trade items from bank"),
            Self::NotEnoughGold => write!(f, "Not enough gold"),
            Self::GoldCapExceeded => write!(f, "Gold cap would be exceeded"),
            Self::ScamPreventionDelay => write!(f, "Please wait before accepting"),
            Self::TradeAlreadyProcessing => write!(f, "Trade is already being processed"),
            Self::ItemDisappeared => write!(f, "Item no longer exists"),
            Self::GoldChangedDuringTrade => write!(f, "Gold amount changed during trade"),
            Self::PlayerInventoryFull => write!(f, "Your inventory is full"),
            Self::TargetInventoryFull => write!(f, "Target's inventory is full"),
            Self::SpellCastFailed => write!(f, "Spell cast failed"),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for TradeError {}

impl TradeError {
    /// Convert error to appropriate TradeStatus for client notification
    pub fn to_trade_status(&self) -> TradeStatus {
        match self {
            Self::PlayerDead => TradeStatus::YouDead,
            Self::TargetDead => TradeStatus::TargetDead,
            Self::PlayerStunned => TradeStatus::YouStunned,
            Self::TargetStunned => TradeStatus::TargetStunned,
            Self::PlayerLoggingOut => TradeStatus::YouLogout,
            Self::TargetLoggingOut => TradeStatus::TargetLogout,
            Self::WrongFaction => TradeStatus::WrongFaction,
            Self::TargetTooFar => TradeStatus::TargetTooFar,
            Self::TrialAccountRestricted => TradeStatus::TrialAccount,
            Self::TargetIgnoringPlayer => TradeStatus::IgnoreYou,
            Self::TargetAlreadyTrading | Self::AlreadyTrading => TradeStatus::Busy,
            _ => TradeStatus::TradeCanceled,
        }
    }
}

// ========== TRADE DATA ==========

/// Per-player trade data within a trade session
#[derive(Debug, Clone)]
pub struct TradeData {
    /// Items in trade slots (7 slots: 0-5 traded, 6 non-traded for enchanting)
    pub items: [Option<ObjectGuid>; TRADE_SLOT_COUNT],
    /// Gold amount offered (in copper)
    pub gold: u32,
    /// Enchantment spell ID (for slot 6 enchanting)
    pub spell_id: u32,
    /// Item to cast enchantment on (partner's slot 6 item)
    pub spell_cast_item: Option<ObjectGuid>,
    /// Player has accepted trade
    pub accepted: bool,
    /// Trade is being processed (prevents double-accept)
    pub accept_process: bool,
    /// Last modification timestamp (for scam prevention)
    pub last_modification: Instant,
}

impl TradeData {
    pub fn new() -> Self {
        Self {
            items: [None; TRADE_SLOT_COUNT],
            gold: 0,
            spell_id: 0,
            spell_cast_item: None,
            accepted: false,
            accept_process: false,
            last_modification: Instant::now(),
        }
    }

    /// Check if the scam prevention delay has passed (200ms since last modification)
    pub fn can_accept(&self) -> bool {
        self.last_modification.elapsed().as_millis() >= TRADE_SCAM_PREVENTION_DELAY_MS as u128
    }

    /// Mark as modified - resets accept delay and accepted flag
    pub fn mark_modified(&mut self) {
        self.last_modification = Instant::now();
        self.accepted = false;
    }

    /// Get item in a specific trade slot
    pub fn get_item(&self, slot: usize) -> Option<ObjectGuid> {
        if slot < TRADE_SLOT_COUNT {
            self.items[slot]
        } else {
            None
        }
    }

    /// Set item in a trade slot
    pub fn set_item(&mut self, slot: usize, item_guid: Option<ObjectGuid>) {
        if slot < TRADE_SLOT_COUNT {
            self.items[slot] = item_guid;
            self.mark_modified();
        }
    }

    /// Clear item from a trade slot
    pub fn clear_item(&mut self, slot: usize) {
        if slot < TRADE_SLOT_COUNT {
            self.items[slot] = None;
            self.mark_modified();
        }
    }

    /// Count items in traded slots (excludes non-traded slot 6)
    pub fn traded_item_count(&self) -> usize {
        self.items
            .iter()
            .take(TRADE_SLOT_TRADED_COUNT)
            .filter(|i| i.is_some())
            .count()
    }

    /// Check if item is already in any trade slot
    pub fn has_item(&self, item_guid: ObjectGuid) -> bool {
        self.items.iter().any(|i| *i == Some(item_guid))
    }

    /// Reset trade data to initial state
    pub fn reset(&mut self) {
        self.items = [None; TRADE_SLOT_COUNT];
        self.gold = 0;
        self.spell_id = 0;
        self.spell_cast_item = None;
        self.accepted = false;
        self.accept_process = false;
        self.last_modification = Instant::now();
    }
}

impl Default for TradeData {
    fn default() -> Self {
        Self::new()
    }
}

// ========== ITEM INFO FOR PACKETS ==========

/// Item information for trade window display
#[derive(Debug, Clone, Default)]
pub struct TradeSlotInfo {
    pub slot_index: u8,
    pub item_entry: u32,
    pub display_id: u32,
    pub count: u32,
    pub wrapped: bool,
    pub gift_creator_guid: ObjectGuid,
    pub permanent_enchant: u32,
    pub creator_guid: ObjectGuid,
    pub charges: i32,
    pub suffix_factor: u32,
    pub random_property_id: i32,
    pub lock_id: u32,
    pub max_durability: u32,
    pub durability: u32,
}

impl TradeSlotInfo {
    pub fn empty(slot_index: u8) -> Self {
        Self {
            slot_index,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_state_default() {
        let state = TradeState::default();
        assert_eq!(state, TradeState::Initiated);
    }

    #[test]
    fn test_trade_data_new() {
        let data = TradeData::new();
        assert_eq!(data.gold, 0);
        assert!(!data.accepted);
        assert!(!data.accept_process);
        assert_eq!(data.traded_item_count(), 0);
    }

    #[test]
    fn test_trade_data_set_item() {
        let mut data = TradeData::new();
        let guid = ObjectGuid::from_raw(123);

        data.set_item(0, Some(guid));
        assert_eq!(data.get_item(0), Some(guid));
        assert!(data.has_item(guid));
        assert_eq!(data.traded_item_count(), 1);
    }

    #[test]
    fn test_trade_data_mark_modified() {
        let mut data = TradeData::new();
        data.accepted = true;

        data.mark_modified();
        assert!(!data.accepted); // Accepted should be reset
    }

    #[test]
    fn test_trade_error_to_status() {
        assert_eq!(
            TradeError::PlayerDead.to_trade_status(),
            TradeStatus::YouDead
        );
        assert_eq!(
            TradeError::TargetTooFar.to_trade_status(),
            TradeStatus::TargetTooFar
        );
        assert_eq!(
            TradeError::WrongFaction.to_trade_status(),
            TradeStatus::WrongFaction
        );
    }

    #[test]
    fn test_trade_slot_info_empty() {
        let slot = TradeSlotInfo::empty(3);
        assert_eq!(slot.slot_index, 3);
        assert_eq!(slot.item_entry, 0);
    }
}
