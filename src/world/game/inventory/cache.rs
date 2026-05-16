//! Inventory cache for fast in-memory lookups
//!
//! This module provides thread-safe caching of player inventory data
//! using DashMap for concurrent access.

use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;

use crate::shared::protocol::{HighGuid, ObjectGuid};
use crate::world::game::items::{Bag, Item};

pub const EQUIPMENT_SLOT_COUNT: usize = 19;
pub const BAG_SLOT_COUNT: usize = 4;
pub const INVENTORY_SLOT_COUNT: usize = 16;
pub const BANK_ITEM_COUNT: usize = 24;
pub const BANK_BAG_COUNT: usize = 6;
pub const BUYBACK_SLOT_COUNT: usize = 12;
pub const KEYRING_SLOT_COUNT: usize = 32;

pub const INVENTORY_SLOT_BAG_0: u8 = 255;

#[derive(Debug, Clone, PartialEq)]
pub struct CachedItemInfo {
    pub guid: ObjectGuid,
    pub entry_id: u32,
    pub count: u32,
    pub bag: u8,
    pub slot: u8,
}

#[derive(Debug, Default, Clone)]
pub struct PlayerInventoryData {
    pub player_guid: ObjectGuid,
    pub equipment: [Option<ObjectGuid>; EQUIPMENT_SLOT_COUNT],
    pub bag_guids: [Option<ObjectGuid>; BAG_SLOT_COUNT],
    pub inventory: [Option<ObjectGuid>; INVENTORY_SLOT_COUNT],
    pub bank_items: [Option<ObjectGuid>; BANK_ITEM_COUNT],
    pub bank_bag_guids: [Option<ObjectGuid>; BANK_BAG_COUNT],
    pub buyback: [Option<ObjectGuid>; BUYBACK_SLOT_COUNT],
    pub keyring: [Option<ObjectGuid>; KEYRING_SLOT_COUNT],
    pub money: u32,
    pub items: DashMap<ObjectGuid, Arc<RwLock<Item>>>,
    pub bags: DashMap<ObjectGuid, Bag>,
}

impl PlayerInventoryData {
    pub fn new(player_guid: ObjectGuid) -> Self {
        Self {
            player_guid,
            equipment: [None; EQUIPMENT_SLOT_COUNT],
            bag_guids: [None; BAG_SLOT_COUNT],
            inventory: [None; INVENTORY_SLOT_COUNT],
            bank_items: [None; BANK_ITEM_COUNT],
            bank_bag_guids: [None; BANK_BAG_COUNT],
            buyback: [None; BUYBACK_SLOT_COUNT],
            keyring: [None; KEYRING_SLOT_COUNT],
            money: 0,
            items: DashMap::new(),
            bags: DashMap::new(),
        }
    }

    pub fn get_item_at(&self, bag: u8, slot: u8) -> Option<ObjectGuid> {
        if bag == INVENTORY_SLOT_BAG_0 {
            if slot < EQUIPMENT_SLOT_COUNT as u8 {
                self.equipment[slot as usize]
            } else if slot >= 19 && slot < 23 {
                self.bag_guids[(slot - 19) as usize]
            } else if slot >= 23 && slot < 39 {
                self.inventory[(slot - 23) as usize]
            } else if slot >= 39 && slot < 63 {
                self.bank_items[(slot - 39) as usize]
            } else if slot >= 63 && slot < 69 {
                self.bank_bag_guids[(slot - 63) as usize]
            } else if slot >= 69 && slot < 81 {
                self.buyback[(slot - 69) as usize]
            } else if slot >= 81 && slot < 113 {
                self.keyring[(slot - 81) as usize]
            } else {
                None
            }
        } else {
            let bag_guid = ObjectGuid::new_without_entry(HighGuid::Item, bag as u32);
            self.bags.get(&bag_guid).and_then(|bag| bag.get_slot(slot))
        }
    }

    pub fn set_item_at(&mut self, bag: u8, slot: u8, guid: Option<ObjectGuid>) -> bool {
        if bag == INVENTORY_SLOT_BAG_0 {
            if slot < EQUIPMENT_SLOT_COUNT as u8 {
                self.equipment[slot as usize] = guid;
                true
            } else if slot >= 19 && slot < 23 {
                self.bag_guids[(slot - 19) as usize] = guid;
                true
            } else if slot >= 23 && slot < 39 {
                self.inventory[(slot - 23) as usize] = guid;
                true
            } else if slot >= 39 && slot < 63 {
                self.bank_items[(slot - 39) as usize] = guid;
                true
            } else if slot >= 63 && slot < 69 {
                self.bank_bag_guids[(slot - 63) as usize] = guid;
                true
            } else if slot >= 69 && slot < 81 {
                self.buyback[(slot - 69) as usize] = guid;
                true
            } else if slot >= 81 && slot < 113 {
                self.keyring[(slot - 81) as usize] = guid;
                true
            } else {
                false
            }
        } else {
            let bag_guid = ObjectGuid::new_without_entry(HighGuid::Item, bag as u32);
            if let Some(mut bag) = self.bags.get_mut(&bag_guid) {
                bag.set_slot(slot, guid)
            } else {
                false
            }
        }
    }

    pub fn find_free_inventory_slot(&self) -> Option<(u8, u8)> {
        for i in 0..INVENTORY_SLOT_COUNT {
            if self.inventory[i].is_none() {
                return Some((255, (23 + i) as u8));
            }
        }
        None
    }

    pub fn count_free_inventory_slots(&self) -> u32 {
        self.inventory.iter().filter(|x| x.is_none()).count() as u32
    }

    pub fn find_items_by_entry(&self, entry_id: u32) -> Vec<ObjectGuid> {
        self.items
            .iter()
            .filter(|pair| pair.value().read().entry == entry_id)
            .map(|pair| *pair.key())
            .collect()
    }

    pub fn count_items_by_entry(&self, entry_id: u32) -> u32 {
        self.items
            .iter()
            .filter(|pair| pair.value().read().entry == entry_id)
            .map(|pair| pair.value().read().count)
            .sum()
    }

    pub fn get_item_entry(&self, item_guid: ObjectGuid) -> u32 {
        self.items
            .get(&item_guid)
            .map(|item| item.read().entry)
            .unwrap_or(0)
    }

    pub fn is_equipment_slot(&self, slot: u8) -> bool {
        slot < EQUIPMENT_SLOT_COUNT as u8
    }

    pub fn find_free_slot_anywhere(&self) -> Option<(u8, u8)> {
        for i in 0..INVENTORY_SLOT_COUNT {
            if self.inventory[i].is_none() {
                return Some((INVENTORY_SLOT_BAG_0, (23 + i) as u8));
            }
        }

        for (idx, bag_guid) in self.bag_guids.iter().enumerate() {
            if let Some(bag_guid) = *bag_guid {
                if let Some(bag) = self.bags.get(&bag_guid) {
                    if let Some(slot) = bag.find_free_slot() {
                        return Some((bag_guid.low() as u8, slot));
                    }
                }
            }
        }

        None
    }

    pub fn count_total_free_slots(&self) -> u32 {
        let mut count = 0;
        count += self.inventory.iter().filter(|x| x.is_none()).count() as u32;

        for bag_guid in &self.bag_guids {
            if let Some(bag_guid) = *bag_guid {
                if let Some(bag) = self.bags.get(&bag_guid) {
                    count += bag.free_slots();
                }
            }
        }

        count
    }

    pub fn get_or_add_bag(&mut self, bag_guid: ObjectGuid, entry_id: u32, size: u8) {
        if !self.bags.contains_key(&bag_guid) {
            let bag = Bag::with_size(bag_guid, entry_id, size);
            self.bags.insert(bag_guid, bag);
        }
    }
}

/// A deferred DB operation queued during gameplay, flushed periodically.
#[derive(Debug, Clone)]
pub enum PendingInventoryOp {
    MoveItem {
        player_guid: u32,
        item_guid: u32,
        bag: u8,
        slot: u8,
    },
    SwapItems {
        player_guid: u32,
        item1_guid: u32,
        bag1: u8,
        slot1: u8,
        item2_guid: Option<u32>,
        bag2: u8,
        slot2: u8,
    },
    UpdateCount {
        item_guid: u32,
        count: u32,
    },
    DeleteItem {
        item_guid: u32,
    },
    CreateItem {
        item: crate::shared::database::characters::models::item::ItemInstanceRow,
        slot: crate::shared::database::characters::repositories::inventory_repository_trait::InventorySlotRow,
    },
    UpdateMoney {
        player_guid: u32,
        amount: u32,
    },
}

pub struct InventoryCache {
    player_inventories: DashMap<ObjectGuid, PlayerInventoryData>,
    pending_ops: DashMap<ObjectGuid, Vec<PendingInventoryOp>>,
}

impl InventoryCache {
    pub fn new() -> Self {
        Self {
            player_inventories: DashMap::new(),
            pending_ops: DashMap::new(),
        }
    }

    pub fn push_pending_op(&self, player_guid: ObjectGuid, op: PendingInventoryOp) {
        self.pending_ops.entry(player_guid).or_default().push(op);
    }

    pub fn take_pending_ops(&self, player_guid: ObjectGuid) -> Vec<PendingInventoryOp> {
        self.pending_ops
            .remove(&player_guid)
            .map(|(_, ops)| ops)
            .unwrap_or_default()
    }

    pub fn take_all_pending_ops(&self) -> Vec<(ObjectGuid, Vec<PendingInventoryOp>)> {
        let keys: Vec<ObjectGuid> = self.pending_ops.iter().map(|r| *r.key()).collect();
        let mut result = Vec::with_capacity(keys.len());
        for key in keys {
            if let Some((guid, ops)) = self.pending_ops.remove(&key) {
                if !ops.is_empty() {
                    result.push((guid, ops));
                }
            }
        }
        result
    }

    pub fn add_player_inventory(&self, data: PlayerInventoryData) {
        self.player_inventories.insert(data.player_guid, data);
    }

    pub fn remove_player_inventory(&self, player_guid: ObjectGuid) {
        self.player_inventories.remove(&player_guid);
    }

    pub fn get_player_inventory(&self, player_guid: ObjectGuid) -> Option<PlayerInventoryData> {
        self.player_inventories.get(&player_guid).map(|r| r.clone())
    }

    pub fn has_player_inventory(&self, player_guid: ObjectGuid) -> bool {
        self.player_inventories.contains_key(&player_guid)
    }

    pub fn get_item_at(&self, player_guid: ObjectGuid, bag: u8, slot: u8) -> Option<ObjectGuid> {
        self.player_inventories
            .get(&player_guid)
            .map(|inv| inv.get_item_at(bag, slot))
            .flatten()
    }

    pub fn set_item_at(
        &self,
        player_guid: ObjectGuid,
        bag: u8,
        slot: u8,
        guid: Option<ObjectGuid>,
    ) -> bool {
        if let Some(mut inv) = self.player_inventories.get_mut(&player_guid) {
            inv.set_item_at(bag, slot, guid)
        } else {
            false
        }
    }

    pub fn find_free_inventory_slot(&self, player_guid: ObjectGuid) -> Option<(u8, u8)> {
        self.player_inventories
            .get(&player_guid)
            .and_then(|inv| inv.find_free_inventory_slot())
    }

    pub fn count_free_inventory_slots(&self, player_guid: ObjectGuid) -> u32 {
        self.player_inventories
            .get(&player_guid)
            .map(|inv| inv.count_free_inventory_slots())
            .unwrap_or(0)
    }

    pub fn get_money(&self, player_guid: ObjectGuid) -> Option<u32> {
        self.player_inventories
            .get(&player_guid)
            .map(|inv| inv.money)
    }

    pub fn set_money(&self, player_guid: ObjectGuid, money: u32) {
        if let Some(mut inv) = self.player_inventories.get_mut(&player_guid) {
            inv.money = money;
        }
    }

    pub fn add_money(&self, player_guid: ObjectGuid, amount: u32) -> Option<u32> {
        let mut result = None;
        if let Some(mut inv) = self.player_inventories.get_mut(&player_guid) {
            let new_amount = inv.money.saturating_add(amount);
            inv.money = new_amount;
            result = Some(new_amount);
        }
        result
    }

    pub fn remove_money(&self, player_guid: ObjectGuid, amount: u32) -> Option<u32> {
        let mut result = None;
        if let Some(mut inv) = self.player_inventories.get_mut(&player_guid) {
            if inv.money >= amount {
                inv.money -= amount;
                result = Some(inv.money);
            }
        }
        result
    }

    pub fn add_item(&self, player_guid: ObjectGuid, item: Arc<RwLock<Item>>) {
        let item_guid = item.read().guid;
        if let Some(mut inv) = self.player_inventories.get_mut(&player_guid) {
            inv.items.insert(item_guid, item);
        }
    }

    pub fn get_item(
        &self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
    ) -> Option<Arc<RwLock<Item>>> {
        self.player_inventories
            .get(&player_guid)
            .and_then(|inv| inv.items.get(&item_guid).map(|r| Arc::clone(&*r)))
    }

    pub fn remove_item(&self, player_guid: ObjectGuid, item_guid: ObjectGuid) {
        if let Some(mut inv) = self.player_inventories.get_mut(&player_guid) {
            inv.items.remove(&item_guid);
        }
    }

    pub fn get_all_items(&self, player_guid: ObjectGuid) -> Vec<Arc<RwLock<Item>>> {
        self.player_inventories
            .get(&player_guid)
            .map(|inv| {
                inv.items
                    .iter()
                    .map(|pair| Arc::clone(pair.value()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_equipment_slots(&self, player_guid: ObjectGuid) -> Vec<(u8, ObjectGuid)> {
        self.player_inventories
            .get(&player_guid)
            .map(|inv| {
                inv.equipment
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, guid)| guid.map(|g| (idx as u8, g)))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn update_item_count(
        &self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
        new_count: u32,
    ) {
        if let Some(mut inv) = self.player_inventories.get_mut(&player_guid) {
            if let Some(item) = inv.items.get_mut(&item_guid) {
                item.write().count = new_count;
            }
        }
    }

    pub fn find_items_by_entry(&self, player_guid: ObjectGuid, entry_id: u32) -> Vec<ObjectGuid> {
        self.player_inventories
            .get(&player_guid)
            .map(|inv| inv.find_items_by_entry(entry_id))
            .unwrap_or_default()
    }

    pub fn count_items_by_entry(&self, player_guid: ObjectGuid, entry_id: u32) -> u32 {
        self.player_inventories
            .get(&player_guid)
            .map(|inv| inv.count_items_by_entry(entry_id))
            .unwrap_or(0)
    }

    pub fn get_item_info(
        &self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
    ) -> Option<CachedItemInfo> {
        self.player_inventories.get(&player_guid).and_then(|inv| {
            inv.items.get(&item_guid).map(|item| {
                let item = item.read();
                CachedItemInfo {
                    guid: item.guid,
                    entry_id: item.entry,
                    count: item.count,
                    bag: item.bag,
                    slot: item.slot,
                }
            })
        })
    }

    pub fn get_item_entry(&self, player_guid: ObjectGuid, item_guid: ObjectGuid) -> u32 {
        self.player_inventories
            .get(&player_guid)
            .map(|inv| inv.get_item_entry(item_guid))
            .unwrap_or(0)
    }

    pub fn is_equipment_slot(&self, player_guid: ObjectGuid, slot: u8) -> bool {
        self.player_inventories
            .get(&player_guid)
            .map(|inv| inv.is_equipment_slot(slot))
            .unwrap_or(false)
    }

    pub fn set_item_in_bag(
        &self,
        player_guid: ObjectGuid,
        bag_guid: ObjectGuid,
        slot: u8,
        item_guid: Option<ObjectGuid>,
    ) -> bool {
        if let Some(inv) = self.player_inventories.get(&player_guid) {
            if let Some(mut bag) = inv.bags.get_mut(&bag_guid) {
                return bag.set_slot(slot, item_guid);
            }
        }
        false
    }

    pub fn find_free_slot_anywhere(&self, player_guid: ObjectGuid) -> Option<(u8, u8)> {
        self.player_inventories
            .get(&player_guid)
            .and_then(|inv| inv.find_free_slot_anywhere())
    }

    pub fn count_total_free_slots(&self, player_guid: ObjectGuid) -> u32 {
        self.player_inventories
            .get(&player_guid)
            .map(|inv| inv.count_total_free_slots())
            .unwrap_or(0)
    }

    pub fn get_or_add_bag(
        &self,
        player_guid: ObjectGuid,
        bag_guid: ObjectGuid,
        entry_id: u32,
        size: u8,
    ) {
        if let Some(mut inv) = self.player_inventories.get_mut(&player_guid) {
            inv.get_or_add_bag(bag_guid, entry_id, size);
        }
    }
}

impl Default for InventoryCache {
    fn default() -> Self {
        Self::new()
    }
}
