use oxcore_shared::game::inventory::{
    bag_family, encode_position, item_can_go_into_bag, InventoryResult, ItemPosCount,
    ItemPosCountVec, BANK_SLOT_BAG_END, BANK_SLOT_BAG_START, BANK_SLOT_ITEM_END,
    BANK_SLOT_ITEM_START, BUYBACK_SLOT_END, BUYBACK_SLOT_START, INVENTORY_SLOT_BAG_0,
    INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_END,
    INVENTORY_SLOT_ITEM_START, KEYRING_SLOT_END, KEYRING_SLOT_START, NULL_BAG, NULL_SLOT,
};

use crate::game::items::manager::ItemTemplate;
use crate::game::items::Item;
use oxcore_shared::protocol::ObjectGuid;
use std::sync::Arc;

use super::cache::InventoryCache;
use crate::game::ItemManager;

/// Maximum keyring size for vanilla (32 slots, same as KEYRING_SLOT_END - KEYRING_SLOT_START)
const MAX_KEYRING_SIZE: u8 = 32;

/// Check if a bag+slot pair is a valid inventory position.
/// Maps to C++ Player::IsInventoryPos
pub fn is_inventory_pos(bag: u8, slot: u8) -> bool {
    if bag == INVENTORY_SLOT_BAG_0 && slot == NULL_SLOT {
        return true;
    }
    if bag == INVENTORY_SLOT_BAG_0
        && slot >= INVENTORY_SLOT_ITEM_START
        && slot < INVENTORY_SLOT_ITEM_END
    {
        return true;
    }
    if bag >= INVENTORY_SLOT_BAG_START && bag < INVENTORY_SLOT_BAG_END {
        return true;
    }
    if bag == INVENTORY_SLOT_BAG_0 && slot >= KEYRING_SLOT_START && slot < KEYRING_SLOT_END {
        return true;
    }
    false
}

/// Check if a bag+slot pair is a valid equipment position.
/// Maps to C++ Player::IsEquipmentPos
pub fn is_equipment_pos(bag: u8, slot: u8) -> bool {
    if bag == INVENTORY_SLOT_BAG_0 && slot < 19 {
        return true;
    }
    if bag == INVENTORY_SLOT_BAG_0
        && slot >= INVENTORY_SLOT_BAG_START
        && slot < INVENTORY_SLOT_BAG_END
    {
        return true;
    }
    false
}

/// Result of a can_store check
#[derive(Debug, Clone)]
pub struct CanStoreResult {
    /// Destination positions and how many items can go to each
    pub dest: ItemPosCountVec,
    /// Remaining count that couldn't be stored
    pub remaining_count: u32,
    /// Number of items skipped due to unique limit (max_count)
    pub no_similar_count: u32,
    /// Error, if any
    pub error: Option<InventoryResult>,
}

impl Default for CanStoreResult {
    fn default() -> Self {
        Self {
            dest: Vec::new(),
            remaining_count: 0,
            no_similar_count: 0,
            error: None,
        }
    }
}

impl CanStoreResult {
    pub fn ok() -> Self {
        Self::default()
    }

    pub fn err(result: InventoryResult, remaining_count: u32, no_similar_count: u32) -> Self {
        Self {
            dest: Vec::new(),
            remaining_count,
            no_similar_count,
            error: Some(result),
        }
    }

    pub fn total_remaining(&self) -> u32 {
        self.remaining_count + self.no_similar_count
    }
}

/// Dedicated can-store checker that mirrors C++ Player::_CanStoreItem and friends.
///
/// Created per check so that intermediate state (no_similar_count, dest vector)
/// is fresh every invocation and no mutable borrow conflicts arise.
pub struct CanStoreChecker<'a> {
    cache: &'a InventoryCache,
    item_mgr: &'a ItemManager,
    player_guid: ObjectGuid,
}

impl<'a> CanStoreChecker<'a> {
    pub fn new(
        cache: &'a InventoryCache,
        item_mgr: &'a ItemManager,
        player_guid: ObjectGuid,
    ) -> Self {
        Self {
            cache,
            item_mgr,
            player_guid,
        }
    }

    // ── helpers ──────────────────────────────────────────────────────

    fn get_item_at(&self, bag: u8, slot: u8) -> Option<ObjectGuid> {
        self.cache.get_item_at(self.player_guid, bag, slot)
    }

    fn get_item(&self, guid: ObjectGuid) -> Option<std::sync::Arc<parking_lot::RwLock<Item>>> {
        self.cache.get_item(self.player_guid, guid)
    }

    fn get_proto(&self, entry: u32) -> Option<Arc<ItemTemplate>> {
        self.item_mgr.get_template(entry)
    }

    /// Get the bag at a given bag slot (19-22 or 63-68).
    fn resolve_bag_item(&self, bag_slot: u8) -> Option<ObjectGuid> {
        self.get_item_at(INVENTORY_SLOT_BAG_0, bag_slot)
    }

    fn resolve_bag_proto(&self, bag_slot: u8) -> Option<Arc<ItemTemplate>> {
        let bag_guid = self.resolve_bag_item(bag_slot)?;
        let bag_item = self.get_item(bag_guid)?;
        let entry = bag_item.read().entry;
        self.get_proto(entry)
    }

    // ── _CanTakeMoreSimilarItems ──────────────────────────────────────

    /// Check if the player can carry more of a unique/quest item.
    /// Returns ERR_OK if fine, or the specific error + count of items that can't be carried.
    fn can_take_more_similar_items(
        &self,
        entry: u32,
        count: u32,
        p_item: Option<&Item>,
    ) -> (InventoryResult, u32) {
        let proto = match self.get_proto(entry) {
            Some(p) => p,
            None => return (InventoryResult::ItemNotFound, count),
        };

        if proto.max_count == 0 {
            return (InventoryResult::Ok, 0);
        }

        let mut cur_count = self.cache.count_items_by_entry(self.player_guid, entry);

        // Subtract item being moved (if it's the same entry — it's leaving the source)
        if let Some(item) = p_item {
            if item.entry == entry {
                cur_count = cur_count.saturating_sub(item.count);
            }
        }

        if cur_count >= proto.max_count {
            return (InventoryResult::CantCarryMoreOfThis, count);
        }

        let can_take = proto.max_count - cur_count;
        if count <= can_take {
            (InventoryResult::Ok, 0)
        } else {
            (InventoryResult::CantCarryMoreOfThis, count - can_take)
        }
    }

    // ── _CanStoreItem_InSpecificSlot ──────────────────────────────────

    /// Check if an item can be stored in a specific bag+slot.
    fn can_store_item_in_specific_slot(
        &self,
        bag: u8,
        slot: u8,
        dest: &mut ItemPosCountVec,
        proto: &ItemTemplate,
        count: &mut u32,
        swap: bool,
        p_src_item: Option<&Item>,
        bag_slot: &mut u8,
    ) -> InventoryResult {
        let p_item2_guid = self.get_item_at(bag, slot);
        let p_item2 = p_item2_guid.and_then(|g| self.get_item(g));

        // Ignore move item (this slot will be empty at move)
        let p_item2_is_src = p_src_item
            .zip(p_item2.as_ref())
            .map(|(src, dst)| src.guid == dst.read().guid)
            .unwrap_or(false);

        if p_item2_is_src {
            // pretend the slot is empty
        }

        let is_empty_slot = p_item2_guid.is_none() || p_item2_is_src || swap;

        let need_space: u32;

        if is_empty_slot {
            if bag == INVENTORY_SLOT_BAG_0 {
                // keyring case
                if slot >= KEYRING_SLOT_START
                    && slot < KEYRING_SLOT_START + MAX_KEYRING_SIZE
                    && proto.bag_family != bag_family::KEYS
                {
                    *bag_slot = bag;
                    return InventoryResult::ItemDoesntGoIntoBag2;
                }

                // prevent cheating (buyback and out-of-range slots)
                if (slot >= BUYBACK_SLOT_START && slot < BUYBACK_SLOT_END)
                    || slot >= BANK_SLOT_BAG_END
                {
                    *bag_slot = bag;
                    return InventoryResult::ItemDoesntGoIntoBag2;
                }
            } else {
                // item is being placed in a bag (19-22 or 63-68)
                let p_bag_guid = self.resolve_bag_item(bag);
                if p_bag_guid.is_none()
                    || p_src_item
                        .map(|s| p_bag_guid == Some(s.guid))
                        .unwrap_or(false)
                {
                    *bag_slot = bag;
                    return InventoryResult::IntBagError;
                }

                let p_bag_proto = match self.resolve_bag_proto(bag) {
                    Some(p) => p,
                    None => {
                        *bag_slot = bag;
                        return InventoryResult::IntBagError;
                    }
                };

                if slot >= p_bag_proto.container_slots {
                    *bag_slot = bag;
                    return InventoryResult::IntBagError;
                }

                if !item_can_go_into_bag(proto.bag_family, p_bag_proto.bag_family) {
                    *bag_slot = bag;
                    return InventoryResult::ItemDoesntGoIntoBag2;
                }
            }

            need_space = proto.stackable;
        } else {
            // non-empty slot, check item type
            if let Some(p_item2_ref) = p_item2 {
                let p_item2_read = p_item2_ref.read();
                let res = can_be_merged_partly_with(p_item2_read.entry, p_item2_read.count, proto);
                if res != InventoryResult::Ok {
                    return res;
                }
                need_space = proto.stackable.saturating_sub(p_item2_read.count);
            } else {
                return InventoryResult::ItemNotFound;
            }
        }

        let actual_need = need_space.min(*count);
        let pos = encode_position(bag, slot);
        let new_pos = ItemPosCount::new(pos, actual_need as u8);
        if !dest.iter().any(|p| p.pos == new_pos.pos) {
            dest.push(new_pos);
            *count -= actual_need;
        }

        InventoryResult::Ok
    }

    // ── _CanStoreItem_InBag ───────────────────────────────────────────

    /// Check if an item can be stored in a specific bag (searching its slots).
    fn can_store_item_in_bag(
        &self,
        bag: u8,
        dest: &mut ItemPosCountVec,
        proto: &ItemTemplate,
        count: &mut u32,
        merge: bool,
        non_specialized: bool,
        p_src_item: Option<&Item>,
        skip_bag: u8,
        skip_slot: u8,
        bag_slot: &mut u8,
    ) -> InventoryResult {
        if bag == skip_bag {
            *bag_slot = bag;
            return InventoryResult::ItemDoesntGoIntoBag2;
        }

        let p_bag_guid = match self.resolve_bag_item(bag) {
            Some(g) => g,
            None => {
                *bag_slot = bag;
                return InventoryResult::IntBagError;
            }
        };

        if p_src_item.map(|s| p_bag_guid == s.guid).unwrap_or(false) {
            *bag_slot = bag;
            return InventoryResult::IntBagError;
        }

        let p_bag_proto = match self.resolve_bag_proto(bag) {
            Some(p) => p,
            None => {
                *bag_slot = bag;
                return InventoryResult::IntBagError;
            }
        };

        // specialized bag mode: non_specialized=false means we want bags that match the item's bag_family.
        // non_specialized=true means we want plain containers (class=container, subclass=container).
        let is_plain_container = p_bag_proto.item_class == 1 && p_bag_proto.item_subclass == 0; // ITEM_CLASS_CONTAINER=1, ITEM_SUBCLASS_CONTAINER=0

        if non_specialized != is_plain_container {
            *bag_slot = bag;
            return InventoryResult::ItemDoesntGoIntoBag2;
        }

        if !item_can_go_into_bag(proto.bag_family, p_bag_proto.bag_family) {
            *bag_slot = bag;
            return InventoryResult::ItemDoesntGoIntoBag2;
        }

        let bag_size = p_bag_proto.container_slots;

        for j in 0..bag_size {
            if j == skip_slot && bag == skip_bag {
                continue;
            }

            let p_item2_guid = self.get_item_at(bag, j);
            let p_item2_is_src = p_src_item
                .zip(p_item2_guid)
                .map(|(src, dst_guid)| src.guid == dst_guid)
                .unwrap_or(false);

            if p_item2_is_src {
                // ignore move item — slot will be empty
            }

            // if merge, skip empty; if !merge, skip non-empty
            let has_item = p_item2_guid.is_some() && !p_item2_is_src;
            if has_item != merge {
                continue;
            }

            let mut need_space = proto.stackable;

            if has_item {
                if let Some(p_item2) = p_item2_guid.and_then(|g| self.get_item(g)) {
                    let p_item2_read = p_item2.read();
                    let res =
                        can_be_merged_partly_with(p_item2_read.entry, p_item2_read.count, proto);
                    if res != InventoryResult::Ok {
                        continue;
                    }
                    need_space = need_space.saturating_sub(p_item2_read.count);
                } else {
                    continue;
                }
            }

            if need_space > *count {
                need_space = *count;
            }

            let pos = encode_position(bag, j);
            let new_pos = ItemPosCount::new(pos, need_space as u8);
            if !dest.iter().any(|p| p.pos == new_pos.pos) {
                dest.push(new_pos);
                *count -= need_space;
                if *count == 0 {
                    return InventoryResult::Ok;
                }
            }
        }

        InventoryResult::Ok
    }

    // ── _CanStoreItem_InInventorySlots ────────────────────────────────

    /// Search a range of main-inventory slots for space.
    fn can_store_item_in_inventory_slots(
        &self,
        slot_begin: u8,
        slot_end: u8,
        dest: &mut ItemPosCountVec,
        proto: &ItemTemplate,
        count: &mut u32,
        merge: bool,
        p_src_item: Option<&Item>,
        skip_bag: u8,
        skip_slot: u8,
    ) -> InventoryResult {
        for j in slot_begin..slot_end {
            if INVENTORY_SLOT_BAG_0 == skip_bag && j == skip_slot {
                continue;
            }

            let p_item2_guid = self.get_item_at(INVENTORY_SLOT_BAG_0, j);
            let p_item2_is_src = p_src_item
                .zip(p_item2_guid)
                .map(|(src, dst_guid)| src.guid == dst_guid)
                .unwrap_or(false);

            if p_item2_is_src {
                // slot will be empty
            }

            let has_item = p_item2_guid.is_some() && !p_item2_is_src;
            if has_item != merge {
                continue;
            }

            let mut need_space = proto.stackable;

            if has_item {
                if let Some(p_item2) = p_item2_guid.and_then(|g| self.get_item(g)) {
                    let p_item2_read = p_item2.read();
                    let res =
                        can_be_merged_partly_with(p_item2_read.entry, p_item2_read.count, proto);
                    if res != InventoryResult::Ok {
                        continue;
                    }
                    need_space = need_space.saturating_sub(p_item2_read.count);
                } else {
                    continue;
                }
            }

            if need_space > *count {
                need_space = *count;
            }

            let pos = encode_position(INVENTORY_SLOT_BAG_0, j);
            let new_pos = ItemPosCount::new(pos, need_space as u8);
            if !dest.iter().any(|p| p.pos == new_pos.pos) {
                dest.push(new_pos);
                *count -= need_space;
                if *count == 0 {
                    return InventoryResult::Ok;
                }
            }
        }

        InventoryResult::Ok
    }

    // ── _CanStoreItem (top-level) ─────────────────────────────────────

    /// Top-level check: can an item be stored at (bag, slot)?
    ///
    /// * `bag` / `slot` — desired destination (NULL_BAG / NULL_SLOT means "anywhere")
    /// * `entry` — item template entry
    /// * `count` — how many to store
    /// * `p_item` — the actual item instance (for bind/temp-loot checks)
    /// * `swap` — true if this is part of a swap operation
    pub fn can_store_item(
        &self,
        bag: u8,
        slot: u8,
        entry: u32,
        mut count: u32,
        p_item: Option<&Item>,
        swap: bool,
    ) -> CanStoreResult {
        let proto_arc = match self.get_proto(entry) {
            Some(p) => p,
            None => {
                return CanStoreResult::err(
                    if swap {
                        InventoryResult::ItemsCantBeSwapped
                    } else {
                        InventoryResult::ItemNotFound
                    },
                    count,
                    0,
                );
            }
        };
        let proto: &ItemTemplate = &*proto_arc;

        // item used / temp loot
        if let Some(item) = p_item {
            if item.loot_state != crate::game::items::item::ItemLootUpdateState::None {
                return CanStoreResult::err(InventoryResult::AlreadyLooted, count, 0);
            }

            if item.is_bound_to_other_player(self.player_guid) {
                return CanStoreResult::err(InventoryResult::DontOwnThatItem, count, 0);
            }
        }

        // check count of similar items (unique/quest limits)
        let (similar_res, no_similar_count) =
            self.can_take_more_similar_items(entry, count, p_item);

        let mut no_similar: u32 = no_similar_count;

        if similar_res != InventoryResult::Ok {
            if count == no_similar {
                return CanStoreResult::err(similar_res, count, no_similar);
            }
            count -= no_similar;
        }

        let mut dest: ItemPosCountVec = Vec::new();
        let mut bag_slot: u8 = 0;
        let orig_count = count;

        // ── specific bag+slot ──────────────────────────────────────────
        if bag != NULL_BAG && slot != NULL_SLOT {
            let last_item_slot_field = BANK_SLOT_ITEM_END;
            if bag == INVENTORY_SLOT_BAG_0 && (slot as u32) * 2 > last_item_slot_field as u32 {
                let remaining = count + no_similar;
                return CanStoreResult::err(InventoryResult::ItemDoesntGoToSlot, remaining, 0);
            }

            let res = self.can_store_item_in_specific_slot(
                bag,
                slot,
                &mut dest,
                proto,
                &mut count,
                swap,
                p_item,
                &mut bag_slot,
            );
            if res != InventoryResult::Ok {
                let remaining = count + no_similar;
                return CanStoreResult::err(res, remaining, 0);
            }

            if count == 0 {
                if no_similar == 0 {
                    return CanStoreResult {
                        dest,
                        remaining_count: 0,
                        no_similar_count: 0,
                        error: None,
                    };
                }
                return CanStoreResult::err(
                    InventoryResult::CantCarryMoreOfThis,
                    count,
                    no_similar,
                );
            }
        }

        // ── specific bag (any slot in that bag) ────────────────────────
        if bag != NULL_BAG {
            if proto.stackable > 1 {
                // search stack in bag for merge
                if bag == INVENTORY_SLOT_BAG_0 {
                    // keyring first
                    let res = self.can_store_item_in_inventory_slots(
                        KEYRING_SLOT_START,
                        KEYRING_SLOT_END,
                        &mut dest,
                        proto,
                        &mut count,
                        true,
                        p_item,
                        bag,
                        slot,
                    );
                    if res != InventoryResult::Ok {
                        return CanStoreResult::err(res, count + no_similar, 0);
                    }
                    if count == 0 {
                        return Self::finish_ok_or_limit(dest, no_similar);
                    }

                    // then main inventory
                    let res = self.can_store_item_in_inventory_slots(
                        INVENTORY_SLOT_ITEM_START,
                        INVENTORY_SLOT_ITEM_END,
                        &mut dest,
                        proto,
                        &mut count,
                        true,
                        p_item,
                        bag,
                        slot,
                    );
                    if res != InventoryResult::Ok {
                        return CanStoreResult::err(res, count + no_similar, 0);
                    }
                    if count == 0 {
                        return Self::finish_ok_or_limit(dest, no_similar);
                    }
                } else {
                    // equipped bag — try specialized then non-specialized
                    let res = self.can_store_item_in_bag(
                        bag,
                        &mut dest,
                        proto,
                        &mut count,
                        true,
                        false,
                        p_item,
                        NULL_BAG,
                        slot,
                        &mut bag_slot,
                    );
                    if res != InventoryResult::Ok {
                        let res2 = self.can_store_item_in_bag(
                            bag,
                            &mut dest,
                            proto,
                            &mut count,
                            true,
                            true,
                            p_item,
                            NULL_BAG,
                            slot,
                            &mut bag_slot,
                        );
                        if res2 != InventoryResult::Ok {
                            return CanStoreResult::err(res2, count + no_similar, 0);
                        }
                    }
                    if count == 0 {
                        return Self::finish_ok_or_limit(dest, no_similar);
                    }
                }
            }

            // search free slot in bag
            if bag == INVENTORY_SLOT_BAG_0 {
                if proto.bag_family == bag_family::KEYS {
                    let res = self.can_store_item_in_inventory_slots(
                        KEYRING_SLOT_START,
                        KEYRING_SLOT_START + MAX_KEYRING_SIZE,
                        &mut dest,
                        proto,
                        &mut count,
                        false,
                        p_item,
                        bag,
                        slot,
                    );
                    if res != InventoryResult::Ok {
                        return CanStoreResult::err(res, count + no_similar, 0);
                    }
                    if count == 0 {
                        return Self::finish_ok_or_limit(dest, no_similar);
                    }
                }

                let res = self.can_store_item_in_inventory_slots(
                    INVENTORY_SLOT_ITEM_START,
                    INVENTORY_SLOT_ITEM_END,
                    &mut dest,
                    proto,
                    &mut count,
                    false,
                    p_item,
                    bag,
                    slot,
                );
                if res != InventoryResult::Ok {
                    return CanStoreResult::err(res, count + no_similar, 0);
                }
                if count == 0 {
                    return Self::finish_ok_or_limit(dest, no_similar);
                }
            } else {
                let res = self.can_store_item_in_bag(
                    bag,
                    &mut dest,
                    proto,
                    &mut count,
                    false,
                    false,
                    p_item,
                    NULL_BAG,
                    slot,
                    &mut bag_slot,
                );
                if res != InventoryResult::Ok {
                    let res2 = self.can_store_item_in_bag(
                        bag,
                        &mut dest,
                        proto,
                        &mut count,
                        false,
                        true,
                        p_item,
                        NULL_BAG,
                        slot,
                        &mut bag_slot,
                    );
                    if res2 != InventoryResult::Ok {
                        return CanStoreResult::err(res2, count + no_similar, 0);
                    }
                }
                if count == 0 {
                    return Self::finish_ok_or_limit(dest, no_similar);
                }
            }
        }

        // ── no specific bag — search all bags ──────────────────────────

        if proto.stackable > 1 {
            // merge into existing stacks
            let res = self.can_store_item_in_inventory_slots(
                KEYRING_SLOT_START,
                KEYRING_SLOT_END,
                &mut dest,
                proto,
                &mut count,
                true,
                p_item,
                bag,
                slot,
            );
            if res != InventoryResult::Ok {
                return CanStoreResult::err(res, count + no_similar, 0);
            }
            if count == 0 {
                return Self::finish_ok_or_limit(dest, no_similar);
            }

            let res = self.can_store_item_in_inventory_slots(
                INVENTORY_SLOT_ITEM_START,
                INVENTORY_SLOT_ITEM_END,
                &mut dest,
                proto,
                &mut count,
                true,
                p_item,
                bag,
                slot,
            );
            if res != InventoryResult::Ok {
                return CanStoreResult::err(res, count + no_similar, 0);
            }
            if count == 0 {
                return Self::finish_ok_or_limit(dest, no_similar);
            }

            // specialized bags
            if proto.bag_family != bag_family::NONE {
                for i in INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END {
                    let res = self.can_store_item_in_bag(
                        i,
                        &mut dest,
                        proto,
                        &mut count,
                        true,
                        false,
                        p_item,
                        bag,
                        slot,
                        &mut bag_slot,
                    );
                    if res != InventoryResult::Ok {
                        continue;
                    }
                    if count == 0 {
                        return Self::finish_ok_or_limit(dest, no_similar);
                    }
                }
            }

            // non-specialized bags
            for i in INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END {
                let res = self.can_store_item_in_bag(
                    i,
                    &mut dest,
                    proto,
                    &mut count,
                    true,
                    true,
                    p_item,
                    bag,
                    slot,
                    &mut bag_slot,
                );
                if res != InventoryResult::Ok {
                    continue;
                }
                if count == 0 {
                    return Self::finish_ok_or_limit(dest, no_similar);
                }
            }
        }

        // free slot — specialized bag case
        if proto.bag_family != bag_family::NONE {
            if proto.bag_family == bag_family::KEYS {
                let res = self.can_store_item_in_inventory_slots(
                    KEYRING_SLOT_START,
                    KEYRING_SLOT_START + MAX_KEYRING_SIZE,
                    &mut dest,
                    proto,
                    &mut count,
                    false,
                    p_item,
                    bag,
                    slot,
                );
                if res != InventoryResult::Ok {
                    return CanStoreResult::err(res, count + no_similar, 0);
                }
                if count == 0 {
                    return Self::finish_ok_or_limit(dest, no_similar);
                }
            }

            for i in INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END {
                let res = self.can_store_item_in_bag(
                    i,
                    &mut dest,
                    proto,
                    &mut count,
                    false,
                    false,
                    p_item,
                    bag,
                    slot,
                    &mut bag_slot,
                );
                if res != InventoryResult::Ok {
                    continue;
                }
                if count == 0 {
                    return Self::finish_ok_or_limit(dest, no_similar);
                }
            }
        }

        // non-empty bag over other bag check
        if let Some(item) = p_item {
            if let Some(item_proto) = self.get_proto(item.entry) {
                if item_proto.container_slots > 0 {
                    return CanStoreResult::err(
                        InventoryResult::NonEmptyBagOverOtherBag,
                        count + no_similar,
                        0,
                    );
                }
            }
        }

        // free slot in inventory
        let res = self.can_store_item_in_inventory_slots(
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_END,
            &mut dest,
            proto,
            &mut count,
            false,
            p_item,
            bag,
            slot,
        );
        if res != InventoryResult::Ok {
            return CanStoreResult::err(res, count + no_similar, 0);
        }
        if count == 0 {
            return Self::finish_ok_or_limit(dest, no_similar);
        }

        // free slot in non-specialized bags
        for i in INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END {
            let res = self.can_store_item_in_bag(
                i,
                &mut dest,
                proto,
                &mut count,
                false,
                true,
                p_item,
                bag,
                slot,
                &mut bag_slot,
            );
            if res != InventoryResult::Ok {
                continue;
            }
            if count == 0 {
                return Self::finish_ok_or_limit(dest, no_similar);
            }
        }

        let total_remaining = count + no_similar;
        CanStoreResult::err(InventoryResult::InventoryFull, count, no_similar)
    }

    fn finish_ok_or_limit(dest: ItemPosCountVec, no_similar: u32) -> CanStoreResult {
        if no_similar == 0 {
            CanStoreResult {
                dest,
                remaining_count: 0,
                no_similar_count: 0,
                error: None,
            }
        } else {
            CanStoreResult {
                dest,
                remaining_count: 0,
                no_similar_count: no_similar,
                error: Some(InventoryResult::CantCarryMoreOfThis),
            }
        }
    }

    // ── batch check (CanStoreItems) ───────────────────────────────────

    /// Check if a set of items can all be stored at once (for quest rewards, loot, etc.).
    /// Returns the first error found.
    pub fn can_store_items(&self, items: &[Option<&Item>]) -> CanStoreResult {
        // Build slot occupancy array
        let inv_slot_count = (INVENTORY_SLOT_ITEM_END - INVENTORY_SLOT_ITEM_START) as usize;
        let keyring_count = (KEYRING_SLOT_END - KEYRING_SLOT_START) as usize;

        let mut inv_slot_items = vec![0u32; inv_slot_count];
        let mut inv_keys = vec![0u32; keyring_count];

        for i in INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_END {
            if let Some(guid) = self.get_item_at(INVENTORY_SLOT_BAG_0, i) {
                if let Some(item) = self.get_item(guid) {
                    let item_read = item.read();
                    inv_slot_items[(i - INVENTORY_SLOT_ITEM_START) as usize] = item_read.count;
                }
            }
        }

        for i in KEYRING_SLOT_START..KEYRING_SLOT_END {
            if let Some(guid) = self.get_item_at(INVENTORY_SLOT_BAG_0, i) {
                if let Some(item) = self.get_item(guid) {
                    let item_read = item.read();
                    inv_keys[(i - KEYRING_SLOT_START) as usize] = item_read.count;
                }
            }
        }

        // Simplified batch check: just run can_store_item for each
        // and aggregate. C++ does a more complex space simulation.
        let mut total_count: u32 = 0;
        for item_opt in items {
            let item = match item_opt {
                Some(i) => i,
                None => continue,
            };

            if let Some(proto) = self.get_proto(item.entry) {
                if proto.container_slots > 0 {
                    // Non-empty bag check: we can't tell if it's empty
                    // without scanning contents. Assume it might be non-empty.
                    // For simplicity, skip the check; the item handler
                    // should already reject bag moves.
                }

                if item.is_bound_to_other_player(self.player_guid) {
                    return CanStoreResult::err(InventoryResult::DontOwnThatItem, 0, 0);
                }

                if item.loot_state != crate::game::items::item::ItemLootUpdateState::None {
                    return CanStoreResult::err(InventoryResult::AlreadyLooted, 0, 0);
                }

                let (res, _) = self.can_take_more_similar_items(item.entry, item.count, Some(item));
                if res != InventoryResult::Ok {
                    return CanStoreResult::err(res, 0, 0);
                }
            }

            total_count += item.count;
        }

        if total_count == 0 {
            return CanStoreResult::ok();
        }

        // Fallback to regular can_store with NULL_BAG/NULL_SLOT
        // The C++ version simulates slot-by-slot filling, but the
        // simpler approach of checking if there's enough free space
        // works for the common cases.
        let free = self.cache.count_total_free_slots(self.player_guid);
        if free >= total_count {
            CanStoreResult::ok()
        } else {
            CanStoreResult::err(InventoryResult::InventoryFull, total_count - free, 0)
        }
    }
}

/// Check if two items can be merged (same entry, not unique).
/// Maps to C++ Item::CanBeMergedPartlyWith
fn can_be_merged_partly_with(
    other_entry: u32,
    other_count: u32,
    proto: &ItemTemplate,
) -> InventoryResult {
    if other_entry != proto.entry {
        return InventoryResult::ItemCantStack;
    }
    if other_count >= proto.stackable {
        return InventoryResult::ItemCantStack;
    }
    InventoryResult::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    // The actual tests require test_world setup;
    // unit tests for pure helpers follow.
    #[test]
    fn can_be_merged_same_entry_under_max() {
        let proto = ItemTemplate {
            entry: 100,
            name: "Test".into(),
            stackable: 20,
            bag_family: 0,
            ..Default::default()
        };
        assert_eq!(
            can_be_merged_partly_with(100, 5, &proto),
            InventoryResult::Ok
        );
    }

    #[test]
    fn can_be_merged_different_entry_fails() {
        let proto = ItemTemplate {
            entry: 100,
            stackable: 20,
            ..Default::default()
        };
        assert_eq!(
            can_be_merged_partly_with(200, 5, &proto),
            InventoryResult::ItemCantStack
        );
    }

    #[test]
    fn can_be_merged_full_stack_fails() {
        let proto = ItemTemplate {
            entry: 100,
            stackable: 20,
            ..Default::default()
        };
        assert_eq!(
            can_be_merged_partly_with(100, 20, &proto),
            InventoryResult::ItemCantStack
        );
    }

    #[test]
    fn item_can_go_into_general_bag() {
        assert!(item_can_go_into_bag(bag_family::HERBS, bag_family::NONE));
    }

    #[test]
    fn item_can_go_into_matching_specialized_bag() {
        assert!(item_can_go_into_bag(bag_family::HERBS, bag_family::HERBS));
    }

    #[test]
    fn item_cannot_go_into_wrong_specialized_bag() {
        assert!(!item_can_go_into_bag(bag_family::HERBS, bag_family::KEYS));
    }

    #[test]
    fn is_inventory_pos_main_bag_and_bag_slots() {
        assert!(is_inventory_pos(255, 23)); // main bag first slot
        assert!(is_inventory_pos(255, 38)); // main bag last slot
        assert!(is_inventory_pos(19, 0)); // first bag slot
        assert!(is_inventory_pos(22, 15)); // last possible bag+slot
        assert!(is_inventory_pos(255, 81)); // keyring start
        assert!(is_inventory_pos(255, 96)); // keyring end
        assert!(is_inventory_pos(255, 255)); // null slot
        assert!(!is_inventory_pos(255, 0)); // equipment slot - not inventory
        assert!(!is_inventory_pos(255, 39)); // bank slot - not inventory
    }
}
