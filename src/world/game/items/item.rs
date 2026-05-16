//! Item object - full item representation for inventory system
//!
//! Contains all fields needed for inventory operations and update packets.

use crate::shared::messages::update::{CreateObjectBlock, ObjectType};
use crate::shared::protocol::ObjectGuid;
use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
use crate::world::game::common::object_type::update_flags;
use crate::world::game::common::object_type::ObjectTypeId;
use crate::world::game::common::object_type::{TYPEMASK_ITEM, TYPEMASK_OBJECT};
use crate::world::game::common::update_fields::{
    ITEM_FIELD_CONTAINED, ITEM_FIELD_CREATOR, ITEM_FIELD_DURABILITY, ITEM_FIELD_DURATION,
    ITEM_FIELD_ENCHANTMENT, ITEM_FIELD_FLAGS, ITEM_FIELD_GIFTCREATOR, ITEM_FIELD_ITEM_TEXT_ID,
    ITEM_FIELD_MAXDURABILITY, ITEM_FIELD_OWNER, ITEM_FIELD_PROPERTY_SEED,
    ITEM_FIELD_RANDOM_PROPERTIES_ID, ITEM_FIELD_SPELL_CHARGES, ITEM_FIELD_STACK_COUNT,
    OBJECT_FIELD_ENTRY, OBJECT_FIELD_GUID, OBJECT_FIELD_SCALE_X, OBJECT_FIELD_TYPE,
};

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
}
