//! Item object - full item representation for inventory system
//!
//! Contains all fields needed for inventory operations and update packets.

use crate::core::common::guid::ObjectGuid as WorldObjectGuid;
use crate::game::common::object_type::update_flags;
use crate::game::common::object_type::ObjectTypeId;
use crate::game::common::object_type::{TYPEMASK_ITEM, TYPEMASK_OBJECT};
use crate::game::common::update_fields::{
    ITEM_FIELD_CONTAINED, ITEM_FIELD_CREATOR, ITEM_FIELD_DURABILITY, ITEM_FIELD_DURATION,
    ITEM_FIELD_ENCHANTMENT, ITEM_FIELD_FLAGS, ITEM_FIELD_GIFTCREATOR, ITEM_FIELD_ITEM_TEXT_ID,
    ITEM_FIELD_MAXDURABILITY, ITEM_FIELD_OWNER, ITEM_FIELD_PROPERTY_SEED,
    ITEM_FIELD_RANDOM_PROPERTIES_ID, ITEM_FIELD_SPELL_CHARGES, ITEM_FIELD_STACK_COUNT,
    OBJECT_FIELD_ENTRY, OBJECT_FIELD_GUID, OBJECT_FIELD_SCALE_X, OBJECT_FIELD_TYPE,
};
use oxcore_shared::messages::update::{CreateObjectBlock, ObjectType};
use oxcore_shared::protocol::ObjectGuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemUpdateState {
    Unchanged,
    Changed,
    New,
    Removed,
}

/// Kind of unit an item requires as its use target.
/// Mirrors the `type` column of the `item_required_target` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemTargetType {
    /// ITEM_TARGET_TYPE_CREATURE: target must be a living creature.
    Creature,
    /// ITEM_TARGET_TYPE_DEAD: target must be a dead creature.
    Dead,
}

impl ItemTargetType {
    pub fn from_db(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(ItemTargetType::Creature),
            2 => Some(ItemTargetType::Dead),
            _ => None,
        }
    }
}

/// A single required-target rule for an item.
/// Maps to C++ `ItemRequiredTarget`.
#[derive(Debug, Clone, Copy)]
pub struct ItemRequiredTarget {
    pub target_type: ItemTargetType,
    pub target_entry: u32,
}

impl ItemRequiredTarget {
    pub fn new(target_type: ItemTargetType, target_entry: u32) -> Self {
        Self {
            target_type,
            target_entry,
        }
    }

    /// Check whether a candidate target satisfies this rule.
    /// Maps to C++ ItemRequiredTarget::IsFitToRequirements.
    ///
    /// `target_is_unit` is true only for creatures (C++ TYPEID_UNIT); players
    /// and other object types never satisfy a required-target rule.
    pub fn is_fit_to_requirements(
        &self,
        target_is_unit: bool,
        target_entry: u32,
        target_alive: bool,
    ) -> bool {
        if !target_is_unit {
            return false;
        }

        if target_entry != self.target_entry {
            return false;
        }

        match self.target_type {
            ItemTargetType::Creature => target_alive,
            ItemTargetType::Dead => !target_alive,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemLootUpdateState {
    None,
    Temporary,
    Unchanged,
    Changed,
    New,
    Removed,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub guid: ObjectGuid,
    pub entry: u32,
    pub count: u32,
    pub owner_guid: ObjectGuid,
    pub slot: u8,
    pub bag: u8,
    pub flags: u32,
    pub durability: u32,
    pub max_durability: u32,
    pub enchantments: Vec<(u32, u32, u32)>,
    pub random_property_id: i32,
    pub creator_guid: Option<ObjectGuid>,
    pub gift_creator_guid: Option<ObjectGuid>,
    pub duration: u32,
    pub spell_charges: [i32; 5],
    pub update_state: ItemUpdateState,
    pub loot_state: ItemLootUpdateState,
}

impl Item {
    pub fn new(
        guid: ObjectGuid,
        entry: u32,
        count: u32,
        owner_guid: ObjectGuid,
        slot: u8,
        bag: u8,
        flags: u32,
        durability: u32,
        max_durability: u32,
        enchantments: Vec<(u32, u32, u32)>,
        random_property_id: i32,
        creator_guid: Option<ObjectGuid>,
        gift_creator_guid: Option<ObjectGuid>,
        duration: u32,
        spell_charges: [i32; 5],
    ) -> Self {
        Self {
            guid,
            entry,
            count,
            owner_guid,
            slot,
            bag,
            flags,
            durability,
            max_durability,
            enchantments,
            random_property_id,
            creator_guid,
            gift_creator_guid,
            duration,
            spell_charges,
            update_state: ItemUpdateState::Unchanged,
            loot_state: ItemLootUpdateState::None,
        }
    }

    fn mark_changed(&mut self) {
        if self.update_state != ItemUpdateState::New {
            self.update_state = ItemUpdateState::Changed;
        }
    }

    /// Update loot persistence state.
    /// Maps to C++ Item::SetLootState.
    pub fn set_loot_state(&mut self, state: ItemLootUpdateState) {
        match state {
            ItemLootUpdateState::None | ItemLootUpdateState::New => {
                debug_assert!(false, "invalid direct item loot state transition");
                return;
            }
            ItemLootUpdateState::Temporary => {
                debug_assert_eq!(self.loot_state, ItemLootUpdateState::None);
                self.loot_state = ItemLootUpdateState::Temporary;
            }
            ItemLootUpdateState::Changed => {
                if self.loot_state != ItemLootUpdateState::New
                    && self.loot_state != ItemLootUpdateState::Temporary
                {
                    self.loot_state = if self.loot_state == ItemLootUpdateState::None {
                        ItemLootUpdateState::New
                    } else {
                        ItemLootUpdateState::Changed
                    };
                }
            }
            ItemLootUpdateState::Unchanged => {
                if self.loot_state == ItemLootUpdateState::Removed {
                    self.loot_state = ItemLootUpdateState::None;
                } else if self.loot_state != ItemLootUpdateState::Temporary {
                    self.loot_state = ItemLootUpdateState::Unchanged;
                }
            }
            ItemLootUpdateState::Removed => {
                if self.loot_state == ItemLootUpdateState::New
                    || self.loot_state == ItemLootUpdateState::Temporary
                {
                    self.loot_state = ItemLootUpdateState::None;
                    return;
                }

                self.loot_state = ItemLootUpdateState::Removed;
            }
        }

        if self.loot_state != ItemLootUpdateState::None
            && self.loot_state != ItemLootUpdateState::Unchanged
            && self.loot_state != ItemLootUpdateState::Temporary
        {
            self.mark_changed();
        }
    }

    pub fn from_db_row(
        guid: ObjectGuid,
        entry: u32,
        count: u32,
        owner_guid: ObjectGuid,
        slot: u8,
        bag: u8,
        flags: u32,
        durability: u32,
        max_durability: u32,
        enchantments: Vec<(u32, u32, u32)>,
        random_property_id: i32,
        creator_guid: Option<ObjectGuid>,
        gift_creator_guid: Option<ObjectGuid>,
        duration: u32,
        spell_charges: [i32; 5],
    ) -> Self {
        Self::new(
            guid,
            entry,
            count,
            owner_guid,
            slot,
            bag,
            flags,
            durability,
            max_durability,
            enchantments,
            random_property_id,
            creator_guid,
            gift_creator_guid,
            duration,
            spell_charges,
        )
    }

    pub fn to_create_block(&self) -> CreateObjectBlock {
        let world_guid = WorldObjectGuid::from_raw(self.guid.raw());
        let mut block = CreateObjectBlock::new(world_guid, ObjectTypeId::Item, ObjectType::Item);

        block = block.with_flags(update_flags::UPDATEFLAG_ALL);

        tracing::debug!(
            "[ITEM_CREATE] Creating block for item: guid={:?}, world_guid={:?}, entry={}, owner={:?}",
            self.guid, world_guid, self.entry, self.owner_guid
        );

        // OBJECT_FIELD_GUID is a 64-bit GUID - need both low and high parts
        let guid_raw = self.guid.raw();
        // Type mask: TYPEMASK_OBJECT | TYPEMASK_ITEM = 0x0001 | 0x0002 = 0x0003
        let type_mask = (TYPEMASK_OBJECT | TYPEMASK_ITEM) as u32;
        block = block
            .set_guid_field(OBJECT_FIELD_GUID, world_guid)
            // OBJECT_FIELD_TYPE must be set (TYPEMASK_OBJECT | TYPEMASK_ITEM)
            .set_required(OBJECT_FIELD_TYPE, type_mask)
            // OBJECT_FIELD_ENTRY must always be sent (required field for client to look up item template)
            .set_required(OBJECT_FIELD_ENTRY, self.entry)
            .set_float_field(OBJECT_FIELD_SCALE_X, 1.0);

        let owner_raw = self.owner_guid.raw();
        // Item fields - vanilla 1.12.1 uses 32-bit GUIDs for item owner/container
        // These must be set as required fields to ensure they're always sent (even if 0)
        block = block.set_required(ITEM_FIELD_OWNER, owner_raw as u32);

        // ITEM_FIELD_CONTAINED: container GUID if in bag, owner GUID otherwise (32-bit)
        // Must be set as required field to ensure it's always sent
        let contained_raw = self.owner_guid.raw();
        block = block.set_required(ITEM_FIELD_CONTAINED, contained_raw as u32);

        // Creator and gift creator - only set if GUID is non-zero (matching working implementation)
        if let Some(creator) = self.creator_guid {
            if creator.raw() != 0 {
                let creator_raw = creator.raw();
                block = block.set_field(ITEM_FIELD_CREATOR, creator_raw as u32);
            }
        }

        if let Some(gift_creator) = self.gift_creator_guid {
            if gift_creator.raw() != 0 {
                let gift_raw = gift_creator.raw();
                block = block.set_field(ITEM_FIELD_GIFTCREATOR, gift_raw as u32);
            }
        }

        // STACK_COUNT must be set as required - items should always have a count
        block = block.set_required(ITEM_FIELD_STACK_COUNT, self.count);

        // Duration - only set if > 0 (matching working implementation)
        if self.duration > 0 {
            block = block.set_field(ITEM_FIELD_DURATION, self.duration);
        }

        // Spell charges - only set if non-zero (matching working implementation)
        for i in 0..5 {
            if self.spell_charges[i] != 0 {
                block = block.set_field(
                    ITEM_FIELD_SPELL_CHARGES + i as u32,
                    self.spell_charges[i] as u32,
                );
            }
        }

        // Flags - always set as required (matching working implementation)
        // The old implementation uses set_field_required for flags
        block = block.set_required(ITEM_FIELD_FLAGS, self.flags);

        // Enchantments - only set if enchant_id is non-zero (matching working implementation)
        for i in 0..7 {
            if i < self.enchantments.len() {
                let (enchant_id, duration, charges) = self.enchantments[i];
                if enchant_id != 0 {
                    block = block
                        .set_field(ITEM_FIELD_ENCHANTMENT + (i as u32 * 3), enchant_id)
                        .set_field(ITEM_FIELD_ENCHANTMENT + (i as u32 * 3) + 1, duration)
                        .set_field(ITEM_FIELD_ENCHANTMENT + (i as u32 * 3) + 2, charges);
                }
            }
        }

        // Property seed is always 0
        block = block.set_field(ITEM_FIELD_PROPERTY_SEED, 0);

        // Random properties ID - only set if non-zero (matching working implementation)
        if self.random_property_id != 0 {
            block = block.set_field(
                ITEM_FIELD_RANDOM_PROPERTIES_ID,
                self.random_property_id as u32,
            );
        }

        // Item text ID - only set if > 0 (matching working implementation)
        // Note: We don't have item_text_id field in our Item struct, so skip for now

        // Durability fields - ALWAYS set as required (matching working implementation)
        // CRITICAL: The client needs to know the durability value even when it's 0 (broken items)
        // The working implementation always sends these fields regardless of value
        block = block
            .set_required(ITEM_FIELD_DURABILITY, self.durability)
            .set_required(ITEM_FIELD_MAXDURABILITY, self.max_durability);

        block
    }

    /// Set enchantment duration for a specific slot
    /// Maps to C++ Item::SetEnchantmentDuration
    pub fn set_enchantment_duration(&mut self, slot: usize, duration: u32) {
        if slot < self.enchantments.len() {
            let (id, _, charges) = self.enchantments[slot];
            if id != 0 {
                self.enchantments[slot] = (id, duration, charges);
            }
        }
    }

    /// Set enchantment charges for a specific slot
    /// Maps to C++ Item::SetEnchantmentCharges
    pub fn set_enchantment_charges(&mut self, slot: usize, charges: u32) {
        if slot < self.enchantments.len() {
            let (id, duration, _) = self.enchantments[slot];
            if id != 0 {
                self.enchantments[slot] = (id, duration, charges);
            }
        }
    }

    /// Clear enchantment at a specific slot
    /// Maps to C++ Item::ClearEnchantment
    pub fn clear_enchantment(&mut self, slot: usize) {
        if slot < self.enchantments.len() {
            self.enchantments[slot] = (0, 0, 0);
        }
    }

    /// Check if item is soulbound
    /// Maps to C++ Item::IsSoulBound
    pub fn is_soulbound(&self) -> bool {
        const ITEM_DYNFLAG_BOUND: u32 = 0x0001;
        (self.flags & ITEM_DYNFLAG_BOUND) != 0
    }

    /// Check if item can be traded
    /// Maps to C++ Item::CanBeTraded
    pub fn can_be_traded(&self) -> bool {
        // Cannot trade if soulbound
        if self.is_soulbound() {
            return false;
        }

        // Cannot trade if quest item
        const ITEM_FLAGS_QUEST_ITEM: u32 = 0x00000080;
        if (self.flags & ITEM_FLAGS_QUEST_ITEM) != 0 {
            return false;
        }

        // Cannot trade if conjured
        const ITEM_FLAGS_CONJURED: u32 = 0x00000002;
        if (self.flags & ITEM_FLAGS_CONJURED) != 0 {
            return false;
        }

        true
    }

    /// Check if item is bound to a different player
    /// Maps to C++ Item::IsBindedNotWith
    pub fn is_bound_to_other_player(&self, player_guid: ObjectGuid) -> bool {
        self.is_soulbound() && self.owner_guid != player_guid
    }

    /// Check if item is limited to another map or zone
    /// Maps to C++ Item::IsLimitedToAnotherMapOrZone
    /// Note: ItemTemplate currently doesn't have Map/Area fields, so this always returns false
    pub fn is_limited_to_another_map_or_zone(&self, _cur_map_id: u32, _cur_zone_id: u32) -> bool {
        // TODO: Add Map/Area fields to ItemTemplate when needed
        false
    }

    /// Change item entry ID (used for item transformations)
    /// Maps to C++ Item::ChangeEntry
    pub fn change_entry(&mut self, new_entry: u32) {
        self.entry = new_entry;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guid(raw: u64) -> ObjectGuid {
        ObjectGuid::from_raw(raw)
    }

    fn test_item() -> Item {
        Item::new(
            guid(1),
            100,
            1,
            guid(2),
            0,
            0,
            0,
            10,
            10,
            Vec::new(),
            0,
            None,
            None,
            0,
            [0; 5],
        )
    }

    #[test]
    fn item_starts_with_no_loot_and_unchanged_update_state() {
        let item = test_item();

        assert_eq!(item.loot_state, ItemLootUpdateState::None);
        assert_eq!(item.update_state, ItemUpdateState::Unchanged);
    }

    #[test]
    fn temporary_loot_stays_temporary_until_removed() {
        let mut item = test_item();

        item.set_loot_state(ItemLootUpdateState::Temporary);
        assert_eq!(item.loot_state, ItemLootUpdateState::Temporary);
        assert_eq!(item.update_state, ItemUpdateState::Unchanged);

        item.set_loot_state(ItemLootUpdateState::Changed);
        assert_eq!(item.loot_state, ItemLootUpdateState::Temporary);
        assert_eq!(item.update_state, ItemUpdateState::Unchanged);

        item.set_loot_state(ItemLootUpdateState::Removed);
        assert_eq!(item.loot_state, ItemLootUpdateState::None);
        assert_eq!(item.update_state, ItemUpdateState::Unchanged);
    }

    #[test]
    fn changed_from_none_becomes_new_and_marks_item_changed() {
        let mut item = test_item();

        item.set_loot_state(ItemLootUpdateState::Changed);

        assert_eq!(item.loot_state, ItemLootUpdateState::New);
        assert_eq!(item.update_state, ItemUpdateState::Changed);
    }

    #[test]
    fn new_loot_stays_new_until_removed_or_saved() {
        let mut item = test_item();
        item.set_loot_state(ItemLootUpdateState::Changed);

        item.update_state = ItemUpdateState::Unchanged;
        item.set_loot_state(ItemLootUpdateState::Changed);
        assert_eq!(item.loot_state, ItemLootUpdateState::New);
        assert_eq!(item.update_state, ItemUpdateState::Changed);

        item.set_loot_state(ItemLootUpdateState::Removed);
        assert_eq!(item.loot_state, ItemLootUpdateState::None);
        assert_eq!(item.update_state, ItemUpdateState::Changed);
    }

    #[test]
    fn saved_loot_can_change_remove_and_clear_after_save() {
        let mut item = test_item();
        item.set_loot_state(ItemLootUpdateState::Unchanged);
        assert_eq!(item.loot_state, ItemLootUpdateState::Unchanged);

        item.set_loot_state(ItemLootUpdateState::Changed);
        assert_eq!(item.loot_state, ItemLootUpdateState::Changed);
        assert_eq!(item.update_state, ItemUpdateState::Changed);

        item.update_state = ItemUpdateState::Unchanged;
        item.set_loot_state(ItemLootUpdateState::Removed);
        assert_eq!(item.loot_state, ItemLootUpdateState::Removed);
        assert_eq!(item.update_state, ItemUpdateState::Changed);

        item.update_state = ItemUpdateState::Unchanged;
        item.set_loot_state(ItemLootUpdateState::Unchanged);
        assert_eq!(item.loot_state, ItemLootUpdateState::None);
        assert_eq!(item.update_state, ItemUpdateState::Unchanged);
    }

    #[test]
    fn required_target_creature_must_match_entry_and_be_alive() {
        let req = ItemRequiredTarget::new(ItemTargetType::Creature, 2530);

        // Right entry, alive -> fits
        assert!(req.is_fit_to_requirements(true, 2530, true));
        // Right entry, dead -> fails (creature rule needs alive)
        assert!(!req.is_fit_to_requirements(true, 2530, false));
        // Wrong entry -> fails
        assert!(!req.is_fit_to_requirements(true, 9999, true));
        // Not a creature (e.g. a player) -> fails
        assert!(!req.is_fit_to_requirements(false, 2530, true));
    }

    #[test]
    fn required_target_dead_must_match_entry_and_be_dead() {
        let req = ItemRequiredTarget::new(ItemTargetType::Dead, 7318);

        assert!(req.is_fit_to_requirements(true, 7318, false));
        assert!(!req.is_fit_to_requirements(true, 7318, true));
        assert!(!req.is_fit_to_requirements(true, 1, false));
        assert!(!req.is_fit_to_requirements(false, 7318, false));
    }

    #[test]
    fn item_target_type_parses_known_db_values_only() {
        assert_eq!(ItemTargetType::from_db(1), Some(ItemTargetType::Creature));
        assert_eq!(ItemTargetType::from_db(2), Some(ItemTargetType::Dead));
        assert_eq!(ItemTargetType::from_db(0), None);
        assert_eq!(ItemTargetType::from_db(3), None);
    }

    #[test]
    fn changed_loot_preserves_new_update_state() {
        let mut item = test_item();
        item.update_state = ItemUpdateState::New;

        item.set_loot_state(ItemLootUpdateState::Changed);

        assert_eq!(item.loot_state, ItemLootUpdateState::New);
        assert_eq!(item.update_state, ItemUpdateState::New);
    }
}
