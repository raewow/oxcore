use oxcore_shared::protocol::ObjectGuid;
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
    /// Items still lootable by someone.
    ///
    /// Normal items count as soon as they are added. Quest items count only once an
    /// eligible player has actually been shown them (see [`Loot::fill_quest_loot`]),
    /// so a quest drop nobody can use never keeps the corpse lootable.
    pub unlooted_count: u32,

    // Player-dependent loot (different for each player)
    /// Quest item slots already shown to a given player
    pub player_quest_items: HashMap<ObjectGuid, Vec<u8>>,
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
    /// Already counted in `Loot::unlooted_count` (quest items only)
    pub is_counted: bool,
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
    pub is_reference: bool, // If true, item ID points to another loot template
    pub is_quest_drop: bool, // If true, only shown to players who have the quest
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
            unlooted_count: 0,
            player_quest_items: HashMap::new(),
            player_ffa_items: HashMap::new(),
        }
    }

    /// Add a normal drop (non-quest branch)
    pub fn add_item(&mut self, item: LootItem) {
        self.items.push(item);
        self.unlooted_count += 1;
    }

    /// Add a quest drop. Quest items are not counted here — only when an eligible
    /// player is shown them (quest branch).
    pub fn add_quest_item(&mut self, item: LootItem) {
        self.quest_items.push(item);
    }

    /// Resolve which quest items this player may see, counting them once
    /// Mark quest slot as visible for a player and count it once.
    ///
    /// `allowed` decides eligibility for an item id (i.e. the player has the quest).
    /// Returns the quest-item slots visible to this player; the result is cached so
    /// repeated loot windows do not double-count.
    pub fn fill_quest_loot<F>(&mut self, player: ObjectGuid, mut allowed: F) -> Vec<u8>
    where
        F: FnMut(u32) -> bool,
    {
        if let Some(existing) = self.player_quest_items.get(&player) {
            return existing.clone();
        }

        let mut visible = Vec::new();
        for item in self.quest_items.iter_mut() {
            if item.is_looted || !allowed(item.item_id) {
                continue;
            }

            // Count each quest item once, no matter how many eligible players see it
            if !item.is_counted {
                self.unlooted_count += 1;
                item.is_counted = true;
            }
            visible.push(item.slot);
        }

        self.player_quest_items.insert(player, visible.clone());
        visible
    }

    /// Quest item slots already made visible to a player, if any
    pub fn visible_quest_slots(&self, player: ObjectGuid) -> Option<&Vec<u8>> {
        self.player_quest_items.get(&player)
    }

    /// Check if loot is empty
    pub fn is_empty(&self) -> bool {
        self.gold == 0 && self.unlooted_count == 0
    }

    /// Check if player can loot
    pub fn can_loot(&self, player: ObjectGuid) -> bool {
        self.allowed_looters.is_empty() || self.allowed_looters.contains(&player)
    }

    /// Get item by slot
    pub fn get_item(&self, slot: u8) -> Option<&LootItem> {
        self.items.iter().find(|i| i.slot == slot && !i.is_looted)
    }

    /// Get a quest item by its quest-item-list slot index
    pub fn get_quest_item(&self, slot: u8) -> Option<&LootItem> {
        self.quest_items
            .iter()
            .find(|i| i.slot == slot && !i.is_looted)
    }

    /// Mark item as looted
    pub fn loot_item(&mut self, slot: u8) -> Option<LootItem> {
        if let Some(item) = self
            .items
            .iter_mut()
            .find(|i| i.slot == slot && !i.is_looted)
        {
            item.is_looted = true;
            self.unlooted_count = self.unlooted_count.saturating_sub(1);
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
            // Only counted quest items were ever added to unlooted_count
            if item.is_counted {
                self.unlooted_count = self.unlooted_count.saturating_sub(1);
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn item(slot: u8, item_id: u32) -> LootItem {
        LootItem {
            slot,
            item_id,
            count: 1,
            is_looted: false,
            is_blocked: false,
            is_counted: false,
            roll_winner: None,
        }
    }

    fn looted_corpse() -> (Loot, ObjectGuid) {
        let mut loot = Loot::new();
        loot.gold = 42;
        loot.add_item(item(0, 1000));
        loot.add_quest_item(item(0, 2000));
        (loot, ObjectGuid::new_player(1))
    }

    #[test]
    fn quest_drop_nobody_needs_does_not_keep_corpse_lootable() {
        let (mut loot, player) = looted_corpse();

        // Player has no quest for the drop, so it is never shown or counted
        assert!(loot.fill_quest_loot(player, |_| false).is_empty());

        assert_eq!(loot.take_gold(), 42);
        assert!(loot.loot_item(0).is_some());
        assert!(loot.is_empty());
    }

    #[test]
    fn quest_drop_the_looter_needs_keeps_corpse_lootable_until_taken() {
        let (mut loot, player) = looted_corpse();

        assert_eq!(loot.fill_quest_loot(player, |_| true), vec![0]);

        loot.take_gold();
        loot.loot_item(0);
        assert!(!loot.is_empty(), "quest item is still there");

        assert!(loot.loot_quest_item(0).is_some());
        assert!(loot.is_empty());
    }

    #[test]
    fn repeated_loot_windows_do_not_double_count_quest_items() {
        let (mut loot, player) = looted_corpse();

        loot.fill_quest_loot(player, |_| true);
        loot.fill_quest_loot(player, |_| true);

        loot.take_gold();
        loot.loot_item(0);
        loot.loot_quest_item(0);
        assert!(loot.is_empty());
    }

    #[test]
    fn gold_left_behind_keeps_corpse_lootable() {
        let (mut loot, _) = looted_corpse();

        loot.loot_item(0);
        assert!(!loot.is_empty());
    }
}

impl Default for Loot {
    fn default() -> Self {
        Self::new()
    }
}
