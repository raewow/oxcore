//! Inventory update packet types
//!
//! Provides packet structures for sending inventory slot updates to clients.
//! These messages implement `ToWorldPacket` for type-safe packet construction.
//!
//! # Usage
//! ```rust,no_run
//! use wow_server::shared::messages::inventory_update::SmsgInventorySlotUpdate;
//! use wow_server::shared::protocol::ObjectGuid;
//!
//! let msg = SmsgInventorySlotUpdate {
//!     player_guid: ObjectGuid::from_raw(0x1234),
//!     bag: 255,
//!     slot: 0,
//!     item_guid: Some(ObjectGuid::from_raw(0x5678)),
//! };
//! # Ok::<(), ()>(())
//! ```

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::update_fields::{
    PLAYER_FIELD_INV_SLOT_HEAD, PLAYER_FIELD_PACK_SLOT_1, PLAYER_VISIBLE_ITEM_1_0,
};
use crate::shared::protocol::updates::update_mask::UpdateMask;
use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};

const MAX_VISIBLE_ITEM_OFFSET: u32 = 12;
const EQUIPMENT_SLOT_COUNT: u8 = 19;
const INVENTORY_SLOT_START: u8 = 23;
const INVENTORY_SLOT_END: u8 = 39;

#[derive(Debug, Clone)]
pub struct SmsgInventorySlotUpdate {
    pub player_guid: ObjectGuid,
    pub bag: u8,
    pub slot: u8,
    pub item_guid: Option<ObjectGuid>,
}

impl SmsgInventorySlotUpdate {
    pub fn equipment_slot(
        player_guid: ObjectGuid,
        slot: u8,
        item_guid: Option<ObjectGuid>,
    ) -> Self {
        assert!(slot < 23, "Equipment/bag slot must be 0-22");
        Self {
            player_guid,
            bag: 255,
            slot,
            item_guid,
        }
    }

    pub fn inventory_slot(
        player_guid: ObjectGuid,
        slot: u8,
        item_guid: Option<ObjectGuid>,
    ) -> Self {
        assert!(slot >= 23 && slot < 39, "Inventory slot must be 23-38");
        Self {
            player_guid,
            bag: 255,
            slot,
            item_guid,
        }
    }
}

impl ToWorldPacket for SmsgInventorySlotUpdate {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_OBJECT);
        packet.write_u32(1);
        packet.write_u8(0);

        packet.write_u8(0);
        packet.write_packed_guid_raw(self.player_guid.raw());

        let mut mask = UpdateMask::new();

        if self.bag == 255 {
            if self.slot < 23 {
                let field_low = PLAYER_FIELD_INV_SLOT_HEAD + (self.slot as u32 * 2);
                let field_high = field_low + 1;

                if let Some(guid) = self.item_guid {
                    mask.set_guid(field_low, guid.low(), guid.high_u32());
                } else {
                    mask.set_field_required(field_low, 0);
                    mask.set_field_required(field_high, 0);
                }
            } else if self.slot >= INVENTORY_SLOT_START && self.slot < INVENTORY_SLOT_END {
                let field_low =
                    PLAYER_FIELD_PACK_SLOT_1 + ((self.slot - INVENTORY_SLOT_START) as u32 * 2);
                let field_high = field_low + 1;

                if let Some(guid) = self.item_guid {
                    mask.set_guid(field_low, guid.low(), guid.high_u32());
                } else {
                    mask.set_field_required(field_low, 0);
                    mask.set_field_required(field_high, 0);
                }
            }
        }

        mask.write_to_packet(&mut packet);
        packet
    }
}

#[derive(Debug, Clone)]
pub struct SmsgInventorySlotsUpdate {
    pub player_guid: ObjectGuid,
    pub updates: Vec<(u8, u8, Option<ObjectGuid>)>,
}

impl SmsgInventorySlotsUpdate {
    pub fn swap(
        player_guid: ObjectGuid,
        src_bag: u8,
        src_slot: u8,
        src_item: Option<ObjectGuid>,
        dst_bag: u8,
        dst_slot: u8,
        dst_item: Option<ObjectGuid>,
    ) -> Self {
        Self {
            player_guid,
            updates: vec![(src_bag, src_slot, src_item), (dst_bag, dst_slot, dst_item)],
        }
    }
}

impl ToWorldPacket for SmsgInventorySlotsUpdate {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_OBJECT);
        packet.write_u32(1);
        packet.write_u8(0);

        packet.write_u8(0);
        packet.write_packed_guid_raw(self.player_guid.raw());

        let mut mask = UpdateMask::new();

        for (bag, slot, item_guid) in &self.updates {
            if *bag == 255 {
                if *slot < 23 {
                    let field_low = PLAYER_FIELD_INV_SLOT_HEAD + (*slot as u32 * 2);
                    let field_high = field_low + 1;

                    if let Some(guid) = item_guid {
                        mask.set_guid(field_low, guid.low(), guid.high_u32());
                    } else {
                        mask.set_field_required(field_low, 0);
                        mask.set_field_required(field_high, 0);
                    }
                } else if *slot >= INVENTORY_SLOT_START && *slot < INVENTORY_SLOT_END {
                    let field_low =
                        PLAYER_FIELD_PACK_SLOT_1 + ((*slot - INVENTORY_SLOT_START) as u32 * 2);
                    let field_high = field_low + 1;

                    if let Some(guid) = item_guid {
                        mask.set_guid(field_low, guid.low(), guid.high_u32());
                    } else {
                        mask.set_field_required(field_low, 0);
                        mask.set_field_required(field_high, 0);
                    }
                }
            }
        }

        mask.write_to_packet(&mut packet);
        packet
    }
}

#[derive(Debug, Clone)]
pub struct SmsgVisibleItemUpdate {
    pub player_guid: ObjectGuid,
    pub slot: u8,
    pub item_entry: u32,
}

impl SmsgVisibleItemUpdate {
    pub fn new(player_guid: ObjectGuid, slot: u8, item_entry: u32) -> Self {
        assert!(slot < EQUIPMENT_SLOT_COUNT, "Equipment slot must be 0-18");
        Self {
            player_guid,
            slot,
            item_entry,
        }
    }

    pub fn cleared(player_guid: ObjectGuid, slot: u8) -> Self {
        Self::new(player_guid, slot, 0)
    }
}

impl ToWorldPacket for SmsgVisibleItemUpdate {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_OBJECT);
        packet.write_u32(1);
        packet.write_u8(0);

        packet.write_u8(0);
        packet.write_packed_guid_raw(self.player_guid.raw());

        let mut mask = UpdateMask::new();

        let visible_base = PLAYER_VISIBLE_ITEM_1_0 + (self.slot as u32 * MAX_VISIBLE_ITEM_OFFSET);

        if self.item_entry != 0 {
            mask.set_field(visible_base, self.item_entry);
        } else {
            mask.set_field_required(visible_base, 0);
        }

        mask.set_field_required(visible_base + 1, 0x40000000);
        mask.set_field_required(visible_base + 2, 0x40000000);
        mask.set_field_required(visible_base + 3, 0);
        mask.set_field_required(visible_base + 4, 0);
        mask.set_field_required(visible_base + 5, 0);
        mask.set_field_required(visible_base + 6, 0);
        mask.set_field_required(visible_base + 7, 0);
        mask.set_field_required(visible_base + 8, 0);
        mask.set_field_required(visible_base + 9, 0);
        mask.set_field_required(visible_base + 10, 0);
        mask.set_field_required(visible_base + 11, 0);

        mask.write_to_packet(&mut packet);
        packet
    }
}
