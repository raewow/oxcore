use super::types::{Loot, LootItem, LootTableEntry};
use anyhow::Context;
use dashmap::DashMap;
use oxcore_shared::protocol::ObjectGuid;
use rand::Rng;
use std::sync::Arc;

/// The loot stores beyond creature, gameobject and reference loot. Same row layout, separate
/// tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LootStore {
    Pickpocketing,
    Skinning,
    Item,
    Disenchant,
    Fishing,
}

impl LootStore {
    const ALL: [LootStore; 5] = [
        LootStore::Pickpocketing,
        LootStore::Skinning,
        LootStore::Item,
        LootStore::Disenchant,
        LootStore::Fishing,
    ];

    fn table(self) -> &'static str {
        match self {
            LootStore::Pickpocketing => "pickpocketing_loot_template",
            LootStore::Skinning => "skinning_loot_template",
            LootStore::Item => "item_loot_template",
            LootStore::Disenchant => "disenchant_loot_template",
            LootStore::Fishing => "fishing_loot_template",
        }
    }
}

/// Manages loot tables and active loot instances
pub struct LootManager {
    /// Loot tables by creature entry
    creature_loot_tables: DashMap<u32, Vec<LootTableEntry>>,
    /// Loot tables by gameobject loot ID
    gameobject_loot_tables: DashMap<u32, Vec<LootTableEntry>>,
    /// Shared templates that creature and gameobject tables pull in by reference
    reference_loot_tables: DashMap<u32, Vec<LootTableEntry>>,
    /// Pickpocketing, skinning, item, disenchant and fishing tables
    store_tables: DashMap<(LootStore, u32), Vec<LootTableEntry>>,
    /// Active loot instances by source GUID
    active_loot: DashMap<ObjectGuid, Loot>,
}

/// Database row for creature_loot_template
#[derive(sqlx::FromRow, Debug, Clone)]
struct LootTableRow {
    pub entry: i64,
    pub item: i64,
    pub chance: f32,
    pub min_count: i32,
    pub max_count: i16,
    pub group_id: i16,
}

impl LootTableRow {
    fn into_entry(self) -> anyhow::Result<LootTableEntry> {
        let entry = u32::try_from(self.entry).context("loot entry is outside u32 range")?;
        let item = u32::try_from(self.item).context("loot item is outside u32 range")?;
        let max_count = u32::try_from(self.max_count)
            .context("loot max_count must not be negative")?
            .max(1);
        let group_id = u8::try_from(self.group_id).context("loot group_id is outside u8 range")?;

        Ok(LootTableEntry {
            entry,
            item,
            chance: self.chance.abs(),
            min_count: self.min_count.max(1) as u32,
            max_count,
            group_id,
            is_reference: self.min_count < 0,
            reference_id: if self.min_count < 0 {
                self.min_count.unsigned_abs()
            } else {
                0
            },
            is_quest_drop: self.chance < 0.0,
        })
    }
}

impl LootManager {
    pub fn new() -> Self {
        Self {
            creature_loot_tables: DashMap::new(),
            gameobject_loot_tables: DashMap::new(),
            reference_loot_tables: DashMap::new(),
            store_tables: DashMap::new(),
            active_loot: DashMap::new(),
        }
    }

    /// Load loot tables from database
    pub async fn load_loot_tables(&self, world_db: &sqlx::PgPool) -> anyhow::Result<()> {
        let rows = sqlx::query_as::<_, LootTableRow>(
            r#"SELECT entry, item, "ChanceOrQuestChance" AS chance,
                "mincountOrRef" AS min_count, maxcount AS max_count, groupid AS group_id
                FROM world.creature_loot_template
                WHERE patch_min <= 10 AND patch_max >= 10"#,
        )
        .fetch_all(world_db)
        .await?;

        for row in rows {
            let entry = row.into_entry()?;

            self.creature_loot_tables
                .entry(entry.entry)
                .or_insert_with(Vec::new)
                .push(entry);
        }

        tracing::info!(
            "Loaded loot tables for {} creature entries",
            self.creature_loot_tables.len()
        );

        // Rows are kept per patch range; only the ranges covering 1.12 (patch 10) apply.
        let rows = sqlx::query_as::<_, LootTableRow>(
            r#"SELECT entry, item, "ChanceOrQuestChance" AS chance,
                "mincountOrRef" AS min_count, maxcount AS max_count, groupid AS group_id
                FROM world.reference_loot_template
                WHERE patch_min <= 10 AND patch_max >= 10"#,
        )
        .fetch_all(world_db)
        .await?;
        for row in rows {
            let entry = row.into_entry()?;
            self.reference_loot_tables
                .entry(entry.entry)
                .or_insert_with(Vec::new)
                .push(entry);
        }
        tracing::info!(
            "Loaded {} reference loot templates",
            self.reference_loot_tables.len()
        );

        for store in LootStore::ALL {
            let query = format!(
                r#"SELECT entry, item, "ChanceOrQuestChance" AS chance,
                    "mincountOrRef" AS min_count, maxcount AS max_count, groupid AS group_id
                    FROM world.{}
                    WHERE patch_min <= 10 AND patch_max >= 10"#,
                store.table()
            );
            let rows = sqlx::query_as::<_, LootTableRow>(&query)
                .fetch_all(world_db)
                .await?;
            for row in rows {
                let entry = row.into_entry()?;
                self.store_tables
                    .entry((store, entry.entry))
                    .or_insert_with(Vec::new)
                    .push(entry);
            }
        }
        tracing::info!(
            "Loaded {} pickpocketing/skinning/item/disenchant/fishing loot templates",
            self.store_tables.len()
        );
        Ok(())
    }

    /// Load gameobject loot tables from the world database.
    pub async fn load_gameobject_loot_tables(&self, world_db: &sqlx::PgPool) -> anyhow::Result<()> {
        let rows = sqlx::query_as::<_, LootTableRow>(
            r#"SELECT entry, item, "ChanceOrQuestChance" AS chance,
                "mincountOrRef" AS min_count, maxcount AS max_count, groupid AS group_id
                FROM world.gameobject_loot_template
                WHERE patch_min <= 10 AND patch_max >= 10"#,
        )
        .fetch_all(world_db)
        .await?;

        for row in rows {
            let entry = row.into_entry()?;
            self.gameobject_loot_tables
                .entry(entry.entry)
                .or_insert_with(Vec::new)
                .push(entry);
        }

        tracing::info!(
            "Loaded loot tables for {} gameobject loot IDs",
            self.gameobject_loot_tables.len()
        );
        Ok(())
    }

    /// Generate loot for a creature
    pub fn generate_creature_loot(
        &self,
        source_guid: ObjectGuid,
        creature_entry: u32,
        gold_min: u32,
        gold_max: u32,
        allowed_looters: Vec<ObjectGuid>,
    ) -> bool {
        let drops = self
            .creature_loot_tables
            .get(&creature_entry)
            .map(|table| self.roll_template(&table))
            .unwrap_or_default();

        let mut loot = Self::loot_from_drops(drops);
        loot.gold = calculate_gold_drop(gold_min, gold_max);
        loot.allowed_looters = allowed_looters;
        loot.generated = true;

        self.active_loot.insert(source_guid, loot);
        true
    }

    /// Generate loot for a gameobject chest, with the chest template's own coin range.
    pub fn generate_gameobject_loot(
        &self,
        source_guid: ObjectGuid,
        loot_id: u32,
        gold_min: u32,
        gold_max: u32,
    ) -> bool {
        let drops = self
            .gameobject_loot_tables
            .get(&loot_id)
            .map(|table| self.roll_template(&table))
            .unwrap_or_default();

        let mut loot = Self::loot_from_drops(drops);
        loot.gold = calculate_gold_drop(gold_min, gold_max);
        loot.kind = crate::game::loot::LootKind::Gameobject;
        loot.generated = true;
        self.active_loot.insert(source_guid, loot);
        true
    }

    /// Whether a store has a template for `loot_id`.
    pub fn has_store_loot(&self, store: LootStore, loot_id: u32) -> bool {
        loot_id != 0 && self.store_tables.contains_key(&(store, loot_id))
    }

    /// Generate loot from one of the extra stores (pickpocketing, skinning, item, disenchant,
    /// fishing) into `source`, replacing whatever loot it held.
    pub fn generate_store_loot(
        &self,
        source_guid: ObjectGuid,
        store: LootStore,
        loot_id: u32,
        gold: u32,
        kind: crate::game::loot::LootKind,
        owner: ObjectGuid,
    ) {
        let drops = self
            .store_tables
            .get(&(store, loot_id))
            .map(|table| self.roll_template(&table))
            .unwrap_or_default();
        let mut loot = Self::loot_from_drops(drops);
        loot.gold = gold;
        loot.kind = kind;
        loot.recipient = Some(owner);
        loot.allowed_looters = vec![owner];
        loot.generated = true;
        self.active_loot.insert(source_guid, loot);
    }

    /// Fill loot from a reference template (a battleground's player loot) plus coins.
    pub fn generate_reference_loot(
        &self,
        source_guid: ObjectGuid,
        reference_id: u32,
        gold: u32,
        kind: crate::game::loot::LootKind,
        owner: ObjectGuid,
    ) {
        let drops = self
            .reference_loot_tables
            .get(&reference_id)
            .map(|table| self.roll_template(&table))
            .unwrap_or_default();
        let mut loot = Self::loot_from_drops(drops);
        loot.gold = gold;
        loot.kind = kind;
        loot.recipient = Some(owner);
        loot.allowed_looters = vec![owner];
        loot.generated = true;
        self.active_loot.insert(source_guid, loot);
    }

    /// Roll a whole template: every ungrouped entry on its own chance (references expand into
    /// their template, `max_count` times), then one pick from each group.
    fn roll_template(&self, table: &[LootTableEntry]) -> Vec<RolledDrop> {
        let mut drops = Vec::new();
        self.roll_template_into(table, 0, 0, &mut drops);
        drops
    }

    fn roll_template_into(
        &self,
        table: &[LootTableEntry],
        only_group: u8,
        depth: u8,
        drops: &mut Vec<RolledDrop>,
    ) {
        // Reference chains are shallow in practice; the cap only stops a cyclic table.
        const MAX_REFERENCE_DEPTH: u8 = 8;

        if only_group > 0 {
            if let Some(drop) = roll_group(table, only_group) {
                drops.push(drop);
            }
            return;
        }

        for entry in table
            .iter()
            .filter(|e| e.group_id == 0 || e.is_reference)
        {
            if !roll_chance(entry.chance) {
                continue;
            }
            if entry.is_reference {
                if depth >= MAX_REFERENCE_DEPTH {
                    continue;
                }
                // Cloned so no map guard is held across the recursion.
                let Some(referenced) = self
                    .reference_loot_tables
                    .get(&entry.reference_id)
                    .map(|table| table.clone())
                else {
                    continue;
                };
                for _ in 0..entry.max_count {
                    self.roll_template_into(&referenced, entry.group_id, depth + 1, drops);
                }
            } else {
                drops.push(RolledDrop::from_entry(entry));
            }
        }

        let mut groups: Vec<u8> = table
            .iter()
            .filter(|e| e.group_id > 0 && !e.is_reference)
            .map(|e| e.group_id)
            .collect();
        groups.sort_unstable();
        groups.dedup();
        for group_id in groups {
            if let Some(drop) = roll_group(table, group_id) {
                drops.push(drop);
            }
        }
    }

    /// Lay rolled drops out as a loot: normal items and quest items each numbered from 0.
    ///
    /// A loot holds at most 16 normal and 32 quest items; further drops are discarded, as the
    /// loot window has no room for them.
    fn loot_from_drops(drops: Vec<RolledDrop>) -> Loot {
        const MAX_LOOT_ITEMS: u8 = 16;
        const MAX_QUEST_ITEMS: u8 = 32;

        let mut loot = Loot::new();
        let mut slot: u8 = 0;
        let mut quest_slot: u8 = 0;
        for drop in drops {
            if (drop.is_quest && quest_slot >= MAX_QUEST_ITEMS)
                || (!drop.is_quest && slot >= MAX_LOOT_ITEMS)
            {
                continue;
            }
            let item = LootItem {
                slot: if drop.is_quest { quest_slot } else { slot },
                item_id: drop.item_id,
                count: drop.count,
                is_looted: false,
                is_blocked: false,
                is_counted: false,
                roll_winner: None,
                is_underthreshold: false,
                freeforall: false,
                loot_owner: None,
            };
            if drop.is_quest {
                loot.add_quest_item(item);
                quest_slot = quest_slot.saturating_add(1);
            } else {
                loot.add_item(item);
                slot = slot.saturating_add(1);
            }
        }
        loot
    }

    /// Whether a gameobject loot table has a quest-only item needed by a player.
    pub fn player_needs_gameobject_quest_loot<F>(&self, loot_id: u32, mut needs_item: F) -> bool
    where
        F: FnMut(u32) -> bool,
    {
        self.gameobject_loot_tables
            .get(&loot_id)
            .is_some_and(|table| {
                table
                    .iter()
                    .any(|entry| entry.is_quest_drop && needs_item(entry.item))
            })
    }

    /// Whether a gameobject loot table has any quest-only item at all.
    ///
    /// Player-independent, unlike [`Self::player_needs_gameobject_quest_loot`]: this answers
    /// "could this object ever matter to a quest", which is what decides whether an object is
    /// eligible for quest activation before any particular player is considered.
    pub fn gameobject_loot_has_quest_drop(&self, loot_id: u32) -> bool {
        self.gameobject_loot_tables
            .get(&loot_id)
            .is_some_and(|table| table.iter().any(|entry| entry.is_quest_drop))
    }

    /// Check if loot exists for a source
    pub fn has_loot(&self, source: ObjectGuid) -> bool {
        self.active_loot.contains_key(&source)
    }

    /// Get loot for a source
    pub fn get_loot(
        &self,
        source: ObjectGuid,
    ) -> Option<dashmap::mapref::one::Ref<'_, ObjectGuid, Loot>> {
        self.active_loot.get(&source)
    }

    /// Get mutable loot
    pub fn get_loot_mut(
        &self,
        source: ObjectGuid,
    ) -> Option<dashmap::mapref::one::RefMut<'_, ObjectGuid, Loot>> {
        self.active_loot.get_mut(&source)
    }

    /// Remove loot when corpse despawns
    pub fn remove_loot(&self, source: ObjectGuid) {
        self.active_loot.remove(&source);
    }

    /// Set the loot's allowed-looter set (e.g. the whole tap group).
    pub fn set_allowed_looters(&self, source: ObjectGuid, looters: Vec<ObjectGuid>) {
        if let Some(mut loot) = self.active_loot.get_mut(&source) {
            loot.allowed_looters = looters;
        }
    }

    /// Insert loot directly into the active store (test-only; production populates it via
    /// `generate_creature_loot` and the spawn paths).
    #[cfg(test)]
    pub fn insert_loot(&self, source: ObjectGuid, loot: Loot) {
        self.active_loot.insert(source, loot);
    }

    /// Whether a player is on this loot's allowed-looter set.
    pub fn is_allowed_looter(&self, source: ObjectGuid, player: ObjectGuid) -> Option<bool> {
        self.active_loot
            .get(&source)
            .map(|loot| loot.allowed_looters.contains(&player))
    }

    /// Add the group's active-loot checks for rolls.
    /// Mark an item as blocked (a roll is in progress / finished) in the shared loot.
    pub fn set_item_blocked(&self, source: ObjectGuid, slot: u8, blocked: bool) {
        if let Some(mut loot) = self.active_loot.get_mut(&source) {
            if let Some(item) = loot.items.iter_mut().find(|i| i.slot == slot) {
                item.is_blocked = blocked;
            }
        }
    }

    /// Register `player` as having this loot window open.
    pub fn add_viewer(&self, source: ObjectGuid, player: ObjectGuid) {
        if let Some(mut loot) = self.active_loot.get_mut(&source) {
            loot.viewers.insert(player);
        }
    }

    /// `player` closed the loot window.
    pub fn remove_viewer(&self, source: ObjectGuid, player: ObjectGuid) {
        if let Some(mut loot) = self.active_loot.get_mut(&source) {
            loot.viewers.remove(&player);
        }
    }

    /// Players currently looking at this loot.
    pub fn viewers(&self, source: ObjectGuid) -> Vec<ObjectGuid> {
        self.active_loot
            .get(&source)
            .map(|loot| loot.viewers.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Record who may loot a corpse: the tapper, their group, the looters within reach at kill
    /// time, and this corpse's round-robin owner.
    pub fn set_group_rights(
        &self,
        source: ObjectGuid,
        recipient: ObjectGuid,
        group_id: Option<u32>,
        round_robin_player: Option<ObjectGuid>,
        allowed_looters: Vec<ObjectGuid>,
    ) {
        if let Some(mut loot) = self.active_loot.get_mut(&source) {
            loot.recipient = Some(recipient);
            loot.group_id = group_id;
            loot.round_robin_player = round_robin_player;
            loot.allowed_looters = allowed_looters;
        }
    }

    /// Take gold from loot
    pub fn take_gold(&self, source: ObjectGuid) -> u32 {
        if let Some(mut loot) = self.active_loot.get_mut(&source) {
            loot.take_gold()
        } else {
            0
        }
    }

    /// Resolve which quest items `player` may see, counting them once.
    ///
    /// A quest drop only becomes "unlooted loot" once a player who needs it opens the corpse.
    pub fn fill_quest_loot<F>(&self, source: ObjectGuid, player: ObjectGuid, allowed: F) -> Vec<u8>
    where
        F: FnMut(u32) -> bool,
    {
        self.active_loot
            .get_mut(&source)
            .map(|mut loot| loot.fill_quest_loot(player, allowed))
            .unwrap_or_default()
    }

    /// Look up an item by the wire slot `player` sent, without marking it looted.
    /// Returns `(is_quest, item)`.
    pub fn peek_item(
        &self,
        source: ObjectGuid,
        slot: u8,
        player: ObjectGuid,
    ) -> Option<(bool, LootItem)> {
        let loot = self.active_loot.get(&source)?;
        let (is_quest, real_slot) = loot.resolve_wire_slot(slot, player)?;
        let item = if is_quest {
            loot.get_quest_item(real_slot).cloned()
        } else {
            loot.get_item(real_slot).cloned()
        }?;
        Some((is_quest, item))
    }

    /// Mark the item at `player`'s wire slot as looted, using the same mapping as
    /// [`Self::peek_item`].
    pub fn loot_item(&self, source: ObjectGuid, slot: u8, player: ObjectGuid) -> Option<LootItem> {
        let mut loot = self.active_loot.get_mut(&source)?;
        let (is_quest, real_slot) = loot.resolve_wire_slot(slot, player)?;
        if is_quest {
            loot.loot_quest_item(real_slot)
        } else {
            loot.loot_item(real_slot)
        }
    }

    /// Check if loot is empty
    pub fn is_loot_empty(&self, source: ObjectGuid) -> bool {
        if let Some(loot) = self.active_loot.get(&source) {
            loot.is_empty()
        } else {
            true
        }
    }

    /// Execute closure with mutable loot access
    pub fn with_loot_mut<F, R>(&self, source: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&mut Loot) -> R,
    {
        self.active_loot
            .get_mut(&source)
            .map(|mut loot| f(&mut *loot))
    }
}

/// One item a template roll produced.
#[derive(Debug, Clone)]
struct RolledDrop {
    item_id: u32,
    count: u32,
    is_quest: bool,
}

impl RolledDrop {
    fn from_entry(entry: &LootTableEntry) -> Self {
        let count = if entry.min_count >= entry.max_count {
            entry.min_count
        } else {
            rand::thread_rng().gen_range(entry.min_count..=entry.max_count)
        };
        Self {
            item_id: entry.item,
            count,
            is_quest: entry.is_quest_drop,
        }
    }
}

/// Whether an entry with this percent chance drops.
fn roll_chance(chance: f32) -> bool {
    chance >= 100.0 || rand::random::<f32>() * 100.0 < chance
}

/// Pick at most one item from a loot group.
///
/// Entries with an explicit chance are tried first, each claiming its share of a single
/// 0-100 roll. If none hits, one of the zero-chance ("equal chanced") entries is picked
/// uniformly. A group of only explicit entries whose chances sum below 100 can drop nothing.
fn roll_group(table: &[LootTableEntry], group_id: u8) -> Option<RolledDrop> {
    let members: Vec<&LootTableEntry> = table
        .iter()
        .filter(|e| e.group_id == group_id && !e.is_reference)
        .collect();

    let mut roll = rand::random::<f32>() * 100.0;
    for entry in members.iter().filter(|e| e.chance != 0.0) {
        if entry.chance >= 100.0 {
            return Some(RolledDrop::from_entry(entry));
        }
        roll -= entry.chance;
        if roll < 0.0 {
            return Some(RolledDrop::from_entry(entry));
        }
    }

    let equal: Vec<&&LootTableEntry> = members.iter().filter(|e| e.chance == 0.0).collect();
    if equal.is_empty() {
        return None;
    }
    let pick = rand::thread_rng().gen_range(0..equal.len());
    Some(RolledDrop::from_entry(equal[pick]))
}

/// Roll a coin amount in `[gold_min, gold_max]` with the same rounding as corpse money.
pub fn roll_money(gold_min: u32, gold_max: u32) -> u32 {
    calculate_gold_drop(gold_min, gold_max)
}

/// Roll a gold drop from the creature's own `gold_min`/`gold_max` (creature_template).
/// Many creatures (critters, most beasts) have both at 0 and drop no money at all —
/// this used to be a flat level-based formula that ignored the template entirely, so
/// every kill dropped copper regardless of what the source data said.
fn calculate_gold_drop(gold_min: u32, gold_max: u32) -> u32 {
    if gold_max == 0 {
        return 0;
    }
    if gold_max <= gold_min {
        return gold_max;
    }
    // Wide ranges roll in steps of 256 copper, so a boss's purse is not a uniform copper count.
    if gold_max - gold_min < 32_700 {
        rand::thread_rng().gen_range(gold_min..=gold_max)
    } else {
        rand::thread_rng().gen_range((gold_min >> 8)..=(gold_max >> 8)) << 8
    }
}

impl Default for LootManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(item: u32, chance: f32, group_id: u8) -> LootTableEntry {
        LootTableEntry {
            entry: 1,
            item,
            chance,
            min_count: 1,
            max_count: 1,
            group_id,
            is_reference: false,
            reference_id: 0,
            is_quest_drop: false,
        }
    }

    #[test]
    fn a_group_drops_exactly_one_of_its_items() {
        let manager = LootManager::new();
        // Three equal-chanced (0%) entries in group 1, plus a guaranteed ungrouped drop.
        manager.creature_loot_tables.insert(
            7,
            vec![entry(100, 100.0, 0), entry(1, 0.0, 1), entry(2, 0.0, 1), entry(3, 0.0, 1)],
        );
        for counter in 0..50 {
            let source = ObjectGuid::new_creature(7, counter);
            manager.generate_creature_loot(source, 7, 0, 0, vec![]);
            let loot = manager.get_loot(source).unwrap();
            let grouped = loot.items.iter().filter(|i| i.item_id != 100).count();
            assert_eq!(grouped, 1, "one pick per group, and equal-chanced entries do drop");
            assert!(loot.items.iter().any(|i| i.item_id == 100));
        }
    }

    #[test]
    fn references_expand_their_template_max_count_times() {
        let manager = LootManager::new();
        manager
            .reference_loot_tables
            .insert(9000, vec![entry(555, 100.0, 0)]);
        let mut reference = entry(0, 100.0, 0);
        reference.is_reference = true;
        reference.reference_id = 9000;
        reference.max_count = 2;
        manager.creature_loot_tables.insert(8, vec![reference]);

        let source = ObjectGuid::new_creature(8, 1);
        manager.generate_creature_loot(source, 8, 0, 0, vec![]);
        let loot = manager.get_loot(source).unwrap();
        assert_eq!(loot.items.iter().filter(|i| i.item_id == 555).count(), 2);
        let slots: Vec<u8> = loot.items.iter().map(|i| i.slot).collect();
        assert_eq!(slots, vec![0, 1], "slots are the storage indices");
    }

    #[test]
    fn a_loot_holds_at_most_sixteen_items() {
        let manager = LootManager::new();
        let table: Vec<LootTableEntry> = (1..=20).map(|item| entry(item, 100.0, 0)).collect();
        manager.creature_loot_tables.insert(9, table);
        let source = ObjectGuid::new_creature(9, 1);
        manager.generate_creature_loot(source, 9, 0, 0, vec![]);
        assert_eq!(manager.get_loot(source).unwrap().items.len(), 16);
    }

    #[test]
    fn store_loot_belongs_to_its_owner() {
        let manager = LootManager::new();
        manager
            .store_tables
            .insert((LootStore::Skinning, 77), vec![entry(4304, 100.0, 0)]);
        let corpse = ObjectGuid::new_creature(500, 3);
        let skinner = ObjectGuid::new_player(5);
        assert!(manager.has_store_loot(LootStore::Skinning, 77));
        assert!(!manager.has_store_loot(LootStore::Skinning, 0));

        manager.generate_store_loot(
            corpse,
            LootStore::Skinning,
            77,
            0,
            crate::game::loot::LootKind::Skinning,
            skinner,
        );
        let loot = manager.get_loot(corpse).unwrap();
        assert_eq!(loot.kind, crate::game::loot::LootKind::Skinning);
        assert_eq!(loot.recipient, Some(skinner));
        assert_eq!(loot.allowed_looters, vec![skinner]);
        assert_eq!(loot.items[0].item_id, 4304);
    }

    #[test]
    fn wide_money_ranges_round_to_256_copper() {
        for _ in 0..50 {
            let gold = roll_money(100_000, 200_000);
            assert_eq!(gold % 256, 0);
            assert!((99_840..=200_000).contains(&gold));
        }
        assert_eq!(roll_money(50, 50), 50);
        assert_eq!(roll_money(0, 0), 0);
    }

    #[test]
    fn gameobject_quest_loot_is_generated_as_quest_only() {
        let manager = LootManager::new();
        let loot_id = 10119;
        let source = ObjectGuid::new_gameobject(161557, 1);
        manager.gameobject_loot_tables.insert(
            loot_id,
            vec![LootTableEntry {
                entry: loot_id,
                item: 11119,
                chance: 100.0,
                min_count: 1,
                max_count: 1,
                group_id: 0,
                is_reference: false,
                reference_id: 0,
                is_quest_drop: true,
            }],
        );

        manager.generate_gameobject_loot(source, loot_id, 0, 0);

        let loot = manager.get_loot(source).expect("chest loot should exist");
        assert!(loot.items.is_empty());
        assert_eq!(loot.quest_items.len(), 1);
        assert_eq!(loot.quest_items[0].item_id, 11119);
        assert!(manager.player_needs_gameobject_quest_loot(loot_id, |item_id| item_id == 11119));
        assert!(!manager.player_needs_gameobject_quest_loot(loot_id, |_| false));
    }
}
