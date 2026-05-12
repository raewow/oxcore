use crate::shared::protocol::ObjectGuid;
use std::collections::HashMap;

/// Loot instance for a creature corpse
#[derive(Debug, Clone)]
pub struct Loot {
    /// Items in the loot (normal drops)
    pub items: Vec<LootItem>,
    /// Quest items (shown only to eligible players)
    pub quest_items: Vec<LootItem>,
    /// Gold in copper
    pub gold: u32,
    /// Players allowed to loot
    pub allowed_looters: Vec<ObjectGuid>,
    /// Has loot been generated?
    pub generated: bool,
    /// Is this loot currently being viewed?
    pub being_looted: bool,
    /// Player currently looting
    pub looting_player: Option<ObjectGuid>,

    // Player-dependent loot (different for each player)
    /// Quest items per player
    pub player_quest_items: HashMap<ObjectGuid, Vec<LootItem>>,
    /// Free-for-all items per player
    pub player_ffa_items: HashMap<ObjectGuid, Vec<LootItem>>,
}

/// A single item in loot
#[derive(Debug, Clone)]
pub struct LootItem {
    /// Slot index in loot window
    pub slot: u8,
    /// Item entry ID
    pub item_id: u32,
    /// Stack count
    pub count: u32,
    /// Item already looted?
    pub is_looted: bool,
    /// Blocked for this looter (quest item they don't need)
    pub is_blocked: bool,
    /// Roll in progress
    pub roll_winner: Option<ObjectGuid>,
}

/// Loot table entry from database
#[derive(Debug, Clone)]
pub struct LootTableEntry {
    pub entry: u32,
    pub item: u32,
    pub chance: f32,
    pub min_count: u32,
    pub max_count: u32,
    pub group_id: u8,
    pub is_reference: bool,   // If true, item ID points to another loot template
    pub is_quest_drop: bool,  // If true, only shown to players who have the quest
}

/// Loot group - one item selected from group
#[derive(Debug, Clone)]
pub struct LootGroup {
    pub group_id: u8,
    pub entries: Vec<LootTableEntry>,
    pub equal_chanced: bool, // If true, random selection; if false, explicit chances
}

impl Loot {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            quest_items: Vec::new(),
            gold: 0,
            allowed_looters: Vec::new(),
            generated: false,
            being_looted: false,
            looting_player: None,
            player_quest_items: HashMap::new(),
            player_ffa_items: HashMap::new(),
        }
    }

    /// Check if loot is empty
    pub fn is_empty(&self) -> bool {
        self.gold == 0
            && self.items.iter().all(|i| i.is_looted)
            && self.quest_items.iter().all(|i| i.is_looted)
    }

    /// Check if player can loot
    pub fn can_loot(&self, player: ObjectGuid) -> bool {
        self.allowed_looters.is_empty() || self.allowed_looters.contains(&player)
    }

    /// Get item by slot
    pub fn get_item(&self, slot: u8) -> Option<&LootItem> {
        self.items.iter().find(|i| i.slot == slot && !i.is_looted)
    }

    /// Mark item as looted
    pub fn loot_item(&mut self, slot: u8) -> Option<LootItem> {
        if let Some(item) = self
            .items
            .iter_mut()
            .find(|i| i.slot == slot && !i.is_looted)
        {
            item.is_looted = true;
            Some(item.clone())
        } else {
            None
        }
    }

    /// Mark a quest item as looted by its quest-item-list slot index
    pub fn loot_quest_item(&mut self, slot: u8) -> Option<LootItem> {
        if let Some(item) = self
            .quest_items
            .iter_mut()
            .find(|i| i.slot == slot && !i.is_looted)
        {
            item.is_looted = true;
            Some(item.clone())
        } else {
            None
        }
    }

    /// Take all gold
    pub fn take_gold(&mut self) -> u32 {
        let gold = self.gold;
        self.gold = 0;
        gold
    }
}

impl Default for Loot {
    fn default() -> Self {
        Self::new()
    }
}
