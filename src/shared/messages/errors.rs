use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;
use crate::shared::protocol::ObjectGuid;

// Inventory Result Error Codes - matching MaNGOS InventoryResult enum
pub const EQUIP_ERR_OK: u8 = 0;
pub const EQUIP_ERR_CANT_EQUIP_LEVEL_I: u8 = 1;
pub const EQUIP_ERR_CANT_EQUIP_SKILL: u8 = 2;
pub const EQUIP_ERR_ITEM_DOESNT_GO_TO_SLOT: u8 = 3;
pub const EQUIP_ERR_BAG_FULL: u8 = 4;
pub const EQUIP_ERR_NONEMPTY_BAG_OVER_OTHER_BAG: u8 = 5;
pub const EQUIP_ERR_CANT_TRADE_EQUIP_BAGS: u8 = 6;
pub const EQUIP_ERR_ONLY_AMMO_CAN_GO_HERE: u8 = 7;
pub const EQUIP_ERR_NO_REQUIRED_PROFICIENCY: u8 = 8;
pub const EQUIP_ERR_NO_EQUIPMENT_SLOT_AVAILABLE: u8 = 9;
pub const EQUIP_ERR_YOU_CAN_NEVER_USE_THAT_ITEM: u8 = 10;
pub const EQUIP_ERR_YOU_CAN_NEVER_USE_THAT_ITEM2: u8 = 11;
pub const EQUIP_ERR_NO_EQUIPMENT_SLOT_AVAILABLE2: u8 = 12;
pub const EQUIP_ERR_CANT_EQUIP_WITH_TWOHANDED: u8 = 13;
pub const EQUIP_ERR_CANT_DUAL_WIELD: u8 = 14;
pub const EQUIP_ERR_ITEM_DOESNT_GO_INTO_BAG: u8 = 15;
pub const EQUIP_ERR_ITEM_DOESNT_GO_INTO_BAG2: u8 = 16;
pub const EQUIP_ERR_CANT_CARRY_MORE_OF_THIS: u8 = 17;
pub const EQUIP_ERR_NO_EQUIPMENT_SLOT_AVAILABLE3: u8 = 18;
pub const EQUIP_ERR_ITEM_CANT_STACK: u8 = 19;
pub const EQUIP_ERR_ITEM_CANT_BE_EQUIPPED: u8 = 20;
pub const EQUIP_ERR_ITEMS_CANT_BE_SWAPPED: u8 = 21;
pub const EQUIP_ERR_SLOT_IS_EMPTY: u8 = 22;
pub const EQUIP_ERR_ITEM_NOT_FOUND: u8 = 23;
pub const EQUIP_ERR_CANT_DROP_SOULBOUND: u8 = 24;
pub const EQUIP_ERR_OUT_OF_RANGE: u8 = 25;
pub const EQUIP_ERR_TRIED_TO_SPLIT_MORE_THAN_COUNT: u8 = 26;
pub const EQUIP_ERR_COULDNT_SPLIT_ITEMS: u8 = 27;
pub const EQUIP_ERR_MISSING_REAGENT: u8 = 28;
pub const EQUIP_ERR_NOT_ENOUGH_MONEY: u8 = 29;
pub const EQUIP_ERR_NOT_A_BAG: u8 = 30;
pub const EQUIP_ERR_CAN_ONLY_DO_WITH_EMPTY_BAGS: u8 = 31;
pub const EQUIP_ERR_DONT_OWN_THAT_ITEM: u8 = 32;
pub const EQUIP_ERR_CAN_EQUIP_ONLY1_QUIVER: u8 = 33;
pub const EQUIP_ERR_MUST_PURCHASE_THAT_BAG_SLOT: u8 = 34;
pub const EQUIP_ERR_TOO_FAR_AWAY_FROM_BANK: u8 = 35;
pub const EQUIP_ERR_ITEM_LOCKED: u8 = 36;
pub const EQUIP_ERR_YOU_ARE_STUNNED: u8 = 37;
pub const EQUIP_ERR_YOU_ARE_DEAD: u8 = 38;
pub const EQUIP_ERR_CANT_DO_RIGHT_NOW: u8 = 39;
pub const EQUIP_ERR_INT_BAG_ERROR: u8 = 40;
pub const EQUIP_ERR_CAN_EQUIP_ONLY1_BOLT: u8 = 41;
pub const EQUIP_ERR_CAN_EQUIP_ONLY1_AMMOPOUCH: u8 = 42;
pub const EQUIP_ERR_STACKABLE_CANT_BE_WRAPPED: u8 = 43;
pub const EQUIP_ERR_EQUIPPED_CANT_BE_WRAPPED: u8 = 44;
pub const EQUIP_ERR_WRAPPED_CANT_BE_WRAPPED: u8 = 45;
pub const EQUIP_ERR_BOUND_CANT_BE_WRAPPED: u8 = 46;
pub const EQUIP_ERR_UNIQUE_CANT_BE_WRAPPED: u8 = 47;
pub const EQUIP_ERR_BAGS_CANT_BE_WRAPPED: u8 = 48;
pub const EQUIP_ERR_ALREADY_LOOTED: u8 = 49;
pub const EQUIP_ERR_INVENTORY_FULL: u8 = 50;
pub const EQUIP_ERR_BANK_FULL: u8 = 51;
pub const EQUIP_ERR_ITEM_IS_CURRENTLY_SOLD_OUT: u8 = 52;
pub const EQUIP_ERR_BAG_FULL3: u8 = 53;
pub const EQUIP_ERR_ITEM_NOT_FOUND2: u8 = 54;
pub const EQUIP_ERR_ITEM_CANT_STACK2: u8 = 55;
pub const EQUIP_ERR_BAG_FULL4: u8 = 56;
pub const EQUIP_ERR_ITEM_SOLD_OUT: u8 = 57;
pub const EQUIP_ERR_OBJECT_IS_BUSY: u8 = 58;
pub const EQUIP_ERR_NONE: u8 = 59;
pub const EQUIP_ERR_NOT_IN_COMBAT: u8 = 60;
pub const EQUIP_ERR_NOT_WHILE_DISARMED: u8 = 61;
pub const EQUIP_ERR_BAG_FULL6: u8 = 62;
pub const EQUIP_ERR_CANT_EQUIP_RANK: u8 = 63;
pub const EQUIP_ERR_CANT_EQUIP_REPUTATION: u8 = 64;
pub const EQUIP_ERR_TOO_MANY_SPECIAL_BAGS: u8 = 65;
pub const EQUIP_ERR_LOOT_CANT_LOOT_THAT_NOW: u8 = 66;

// Legacy error constants for backward compatibility
pub const ERR_INV_FULL: u8 = EQUIP_ERR_INVENTORY_FULL;
pub const ERR_ITEM_NOT_FOUND: u8 = EQUIP_ERR_ITEM_NOT_FOUND;
pub const ERR_ITEM_LOCKED: u8 = EQUIP_ERR_ITEM_LOCKED;
pub const ERR_NOT_EQUIPPABLE: u8 = EQUIP_ERR_ITEM_CANT_BE_EQUIPPED;
pub const ERR_CANT_EQUIP_LEVEL_I: u8 = EQUIP_ERR_CANT_EQUIP_LEVEL_I;
pub const ERR_CANT_EQUIP_SKILL: u8 = EQUIP_ERR_CANT_EQUIP_SKILL;
pub const ERR_BAG_FULL: u8 = EQUIP_ERR_BAG_FULL;
pub const ERR_BAG_IN_BAG: u8 = EQUIP_ERR_NONEMPTY_BAG_OVER_OTHER_BAG;
pub const ERR_NOT_ENOUGH_MONEY: u8 = EQUIP_ERR_NOT_ENOUGH_MONEY;
pub const ERR_ITEM_AT_COOLDOWN: u8 = EQUIP_ERR_CANT_DO_RIGHT_NOW;
pub const ERR_PLAYER_SILENCED: u8 = EQUIP_ERR_CANT_DO_RIGHT_NOW;
pub const ERR_UNKNOWN: u8 = EQUIP_ERR_INT_BAG_ERROR;

/// SMSG_INVENTORY_CHANGE_FAILURE packet
///
/// Structure (matching MaNGOS):
/// - uint8: error code (InventoryResult)
/// - If error != EQUIP_ERR_OK:
///   - If error == EQUIP_ERR_CANT_EQUIP_LEVEL_I: uint32 required_level
///   - uint64: src_item_guid (or 0)
///   - uint64: dst_item_guid (or 0)
///   - uint8: bag_type_subclass (usually 0)
#[derive(Debug, Clone)]
pub struct SmsgInventoryChangeFailure {
    pub error: u8,
    pub src_item_guid: Option<ObjectGuid>,
    pub dst_item_guid: Option<ObjectGuid>,
    pub required_level: Option<u32>, // Only used for EQUIP_ERR_CANT_EQUIP_LEVEL_I
}

impl SmsgInventoryChangeFailure {
    pub fn new(error: u8) -> Self {
        Self {
            error,
            src_item_guid: None,
            dst_item_guid: None,
            required_level: None,
        }
    }

    pub fn with_items(error: u8, src: Option<ObjectGuid>, dst: Option<ObjectGuid>) -> Self {
        Self {
            error,
            src_item_guid: src,
            dst_item_guid: dst,
            required_level: None,
        }
    }

    pub fn with_level_requirement(error: u8, required_level: u32) -> Self {
        Self {
            error,
            src_item_guid: None,
            dst_item_guid: None,
            required_level: Some(required_level),
        }
    }
}

impl ToWorldPacket for SmsgInventoryChangeFailure {
    fn to_world_packet(&self) -> WorldPacket {
        // Calculate packet size
        // Base: 1 byte for error
        // If error != OK: +16 bytes for 2 GUIDs + 1 byte for bag_type
        // If error == CANT_EQUIP_LEVEL_I: +4 bytes for required_level
        let size = if self.error == EQUIP_ERR_OK {
            1
        } else if self.error == EQUIP_ERR_CANT_EQUIP_LEVEL_I {
            1 + 4 + 16 + 1
        } else {
            1 + 16 + 1
        };

        let mut packet = WorldPacket::new(Opcode::SMSG_INVENTORY_CHANGE_FAILURE);
        packet.write_u8(self.error);

        if self.error != EQUIP_ERR_OK {
            // Write required level if applicable
            if self.error == EQUIP_ERR_CANT_EQUIP_LEVEL_I {
                packet.write_u32(self.required_level.unwrap_or(0));
            }

            // Write src item GUID (or 0)
            if let Some(guid) = self.src_item_guid {
                packet.write_guid_raw(guid.raw());
            } else {
                packet.write_u64(0);
            }

            // Write dst item GUID (or 0)
            if let Some(guid) = self.dst_item_guid {
                packet.write_guid_raw(guid.raw());
            } else {
                packet.write_u64(0);
            }

            // Write bag type subclass (usually 0)
            packet.write_u8(0);
        }

        packet
    }
}

#[derive(Debug, Clone)]
pub struct SmsgItemCooldown {
    pub item_guid: ObjectGuid,
    pub spell_id: u32,
}

impl ToWorldPacket for SmsgItemCooldown {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ITEM_COOLDOWN);
        packet.write_guid_raw(self.item_guid.raw());
        packet.write_u32(self.spell_id);
        packet
    }
}

#[derive(Debug, Clone)]
pub struct SmsgSetProficiency {
    pub item_class: u8,
    pub proficiency_mask: u32,
}

impl ToWorldPacket for SmsgSetProficiency {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SET_PROFICIENCY);
        packet.write_u8(self.item_class);
        packet.write_u32(self.proficiency_mask);
        packet
    }
}
