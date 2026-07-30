//! Bag object - represents a container (bag, quiver, ammo pouch)
//! Max 36 slots per bag, follows composition pattern with Item

use oxcore_shared::protocol::ObjectGuid;

/// Maximum bag size (36 slots)
pub const MAX_BAG_SIZE: usize = 36;

/// Bag - represents a container item that holds other items
///
/// Composition pattern: Bag wraps Item + array of slot pointers
#[derive(Debug, Clone)]
pub struct Bag {
    /// Container item GUID
    pub guid: ObjectGuid,
    /// Item entry ID (the bag item template)
    pub entry_id: u32,
    /// Bag slots (slot 0 = first slot inside bag)
    /// Array of (slot -> item GUID), None = empty
    pub slots: [Option<ObjectGuid>; MAX_BAG_SIZE],
    /// Number of slots this bag actually has (from item template)
    /// Typically 16, 20, 24, 28, 32, or 36
    pub actual_size: u8,
}

impl Bag {
    /// Create a new empty bag with default size based on entry
    ///
    /// Bag entry IDs and their default sizes:
    /// - 0, 23, 24, 25, 26: 16 slots (Smallest bag)
    /// - 27, 28, 29: 20 slots
    /// - 30, 31, 32: 24 slots
    /// - 33, 34: 28 slots
    /// - Other: 16 slots (default)
    pub fn new(guid: ObjectGuid, entry_id: u32) -> Self {
        let actual_size = match entry_id {
            0 | 23 | 24 | 25 | 26 => 16,
            27 | 28 | 29 => 20,
            30 | 31 | 32 => 24,
            33 | 34 => 28,
            _ => 16,
        };

        Self {
            guid,
            entry_id,
            slots: [None; MAX_BAG_SIZE],
            actual_size,
        }
    }

    /// Create a bag with explicit size (used when loading from DB)
    pub fn with_size(guid: ObjectGuid, entry_id: u32, size: u8) -> Self {
        Self {
            guid,
            entry_id,
            slots: [None; MAX_BAG_SIZE],
            actual_size: size.min(MAX_BAG_SIZE as u8),
        }
    }

    /// Get item GUID at a specific slot (0-indexed inside bag)
    ///
    /// Returns None if slot is beyond actual bag size or empty
    pub fn get_slot(&self, slot: u8) -> Option<ObjectGuid> {
        if slot < self.actual_size as u8 {
            self.slots[slot as usize]
        } else {
            None
        }
    }

    /// Set item GUID at a specific slot
    ///
    /// Returns true if slot was valid and was set, false otherwise
    pub fn set_slot(&mut self, slot: u8, item_guid: Option<ObjectGuid>) -> bool {
        if slot < self.actual_size as u8 {
            self.slots[slot as usize] = item_guid;
            true
        } else {
            false
        }
    }

    /// Get bag capacity (maximum possible slots)
    pub fn capacity(&self) -> u8 {
        MAX_BAG_SIZE as u8
    }

    /// Get actual number of usable slots (from item template)
    pub fn actual_size(&self) -> u8 {
        self.actual_size
    }

    /// Check if bag is completely empty
    pub fn is_empty(&self) -> bool {
        self.slots[..self.actual_size as usize]
            .iter()
            .all(|slot| slot.is_none())
    }

    /// Find the slot containing a specific item GUID
    pub fn get_slot_by_guid(&self, guid: ObjectGuid) -> Option<u8> {
        self.slots[..self.actual_size as usize]
            .iter()
            .position(|slot| *slot == Some(guid))
            .map(|pos| pos as u8)
    }

    /// Get item GUID at position (alias for get_slot)
    pub fn get_item_by_pos(&self, slot: u8) -> Option<ObjectGuid> {
        self.get_slot(slot)
    }

    /// Count free (empty) slots in bag
    pub fn free_slots(&self) -> u32 {
        self.slots[..self.actual_size as usize]
            .iter()
            .filter(|slot| slot.is_none())
            .count() as u32
    }

    /// Find the first free slot in bag
    ///
    /// Returns Some(slot) if free slot found, None if bag is full
    pub fn find_free_slot(&self) -> Option<u8> {
        self.slots[..self.actual_size as usize]
            .iter()
            .position(|slot| slot.is_none())
            .map(|pos| pos as u8)
    }

    /// Check if a slot number is valid for this bag
    pub fn is_valid_slot(&self, slot: u8) -> bool {
        slot < self.actual_size as u8
    }

    /// Check if bag is completely full
    pub fn is_full(&self) -> bool {
        self.slots[..self.actual_size as usize]
            .iter()
            .all(|slot| slot.is_some())
    }

    /// Get count of items currently in bag
    pub fn item_count(&self) -> u32 {
        self.slots[..self.actual_size as usize]
            .iter()
            .filter(|slot| slot.is_some())
            .count() as u32
    }

    /// Clear all slots (used when deleting bag)
    pub fn clear(&mut self) {
        for slot in self.slots.iter_mut() {
            *slot = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guid(raw: u64) -> ObjectGuid {
        ObjectGuid::from_raw(raw)
    }

    #[test]
    fn new_initializes_all_storage_slots_empty() {
        let bag = Bag::new(guid(1), 23);

        assert_eq!(bag.guid, guid(1));
        assert_eq!(bag.entry_id, 23);
        assert_eq!(bag.capacity(), MAX_BAG_SIZE as u8);
        assert_eq!(bag.actual_size(), 16);
        assert!(bag.slots.iter().all(Option::is_none));
        assert!(bag.is_empty());
        assert_eq!(bag.free_slots(), 16);
        assert_eq!(bag.item_count(), 0);
    }

    #[test]
    fn with_size_clamps_to_max_bag_size() {
        let bag = Bag::with_size(guid(1), 123, 100);

        assert_eq!(bag.actual_size(), MAX_BAG_SIZE as u8);
        assert_eq!(bag.free_slots(), MAX_BAG_SIZE as u32);
    }

    #[test]
    fn slot_access_is_limited_to_actual_bag_size() {
        let mut bag = Bag::with_size(guid(1), 123, 2);

        assert!(bag.set_slot(0, Some(guid(10))));
        assert!(bag.set_slot(1, Some(guid(11))));
        assert!(!bag.set_slot(2, Some(guid(12))));

        assert_eq!(bag.get_slot(0), Some(guid(10)));
        assert_eq!(bag.get_item_by_pos(1), Some(guid(11)));
        assert_eq!(bag.get_slot(2), None);
        assert_eq!(bag.get_item_by_pos(2), None);
        assert_eq!(bag.get_slot_by_guid(guid(11)), Some(1));
        assert_eq!(bag.get_slot_by_guid(guid(12)), None);
        assert!(!bag.is_empty());
        assert_eq!(bag.free_slots(), 0);
        assert_eq!(bag.item_count(), 2);
    }

    #[test]
    fn clear_empties_all_storage_slots() {
        let mut bag = Bag::with_size(guid(1), 123, 2);
        assert!(bag.set_slot(0, Some(guid(10))));
        assert!(bag.set_slot(1, Some(guid(11))));

        bag.clear();

        assert!(bag.slots.iter().all(Option::is_none));
        assert!(bag.is_empty());
        assert_eq!(bag.free_slots(), 2);
        assert_eq!(bag.item_count(), 0);
    }
}
