//! Inventory update packet types
//!
//! Provides packet structures for sending inventory slot updates to clients.
//! These messages implement `ToWorldPacket` for type-safe packet construction.
//!
//! # Usage
//! ```rust,no_run
//! use oxcore_shared::messages::inventory_update::SmsgInventorySlotUpdate;
//! use oxcore_shared::protocol::ObjectGuid;
//!
//! let msg = SmsgInventorySlotUpdate {
//!     player_guid: ObjectGuid::from_raw(0x1234),
//!     bag: 255,
//!     slot: 0,
//!     item_guid: Some(ObjectGuid::from_raw(0x5678)),
//! };
//! # Ok::<(), ()>(())
//! ```

use crate::messages::update::{ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock};
use crate::messages::{Recipient, ToWorldPacket};
use crate::protocol::update_fields::{
    PLAYER_FIELD_BANKBAG_SLOT_1, PLAYER_FIELD_BANK_SLOT_1, PLAYER_FIELD_BUYBACK_PRICE_1,
    PLAYER_FIELD_BUYBACK_TIMESTAMP_1, PLAYER_FIELD_INV_SLOT_HEAD, PLAYER_FIELD_PACK_SLOT_1,
    PLAYER_FIELD_VENDORBUYBACK_SLOT_1, PLAYER_VISIBLE_ITEM_1_0,
};
use crate::protocol::updates::update_mask::UpdateMask;
use crate::protocol::{ObjectGuid, Opcode, WorldPacket};

/// The vanilla player field holding the item GUID for one `(bag, slot)` pair, if there is one.
///
/// Shared by the modern encoders below. They do not hand-build a 1.14 field write: they restate the
/// update in vanilla field numbers and let the 1.14 slot map translate, which is the same path
/// every other object update takes and keeps the numbering in exactly one place.
///
/// Equipment, backpack, bank and bank-bag slots all survive that translation: the generator's
/// `ALIASES` table joins each of vanilla's four separately-named arrays to its section of 1.14's one
/// contiguous inventory array (`ACTIVE_PLAYER_FIELD_INV_SLOT_HEAD`), the same fix that restored
/// backpack/bank/bank-bag/buyback/keyring updates after they were found silently dropped (see
/// `docs/modern-opcode-plan.md`'s "inventory slot map" section). Buyback and keyring have no field
/// here because nothing calls this for those slots -- `SmsgBuybackSlotUpdate` is unported for an
/// unrelated reason (its price/timestamp fields have no 1.14 counterpart at all).
fn equipment_slot_field(bag: u8, slot: u8) -> Option<u32> {
    // A bag other than 255 addresses a slot inside a container object, which is a separate object's
    // fields entirely and never appears on the player.
    if bag != 255 {
        return None;
    }
    if slot < INVENTORY_SLOT_START {
        Some(PLAYER_FIELD_INV_SLOT_HEAD + (slot as u32 * 2))
    } else if slot < INVENTORY_SLOT_END {
        Some(PLAYER_FIELD_PACK_SLOT_1 + ((slot - INVENTORY_SLOT_START) as u32 * 2))
    } else if slot < BANK_SLOT_END {
        Some(PLAYER_FIELD_BANK_SLOT_1 + ((slot - BANK_SLOT_START) as u32 * 2))
    } else if slot < BANK_BAG_SLOT_END {
        Some(PLAYER_FIELD_BANKBAG_SLOT_1 + ((slot - BANK_BAG_SLOT_START) as u32 * 2))
    } else {
        None
    }
}

/// Restate a set of slot writes as a modern `SMSG_UPDATE_OBJECT` on the player's own object.
///
/// The player is always the recipient of their own inventory update, so the block is tagged with
/// their GUID: 1.14 splits the player field table in two and the inventory array only exists on the
/// self-only half. Without that tag every write below is discarded as an unknown field.
///
/// `None` when nothing survived the slot filter, so an update that is entirely backpack or bank
/// slots is dropped outright rather than sent as an update block with an empty field mask.
fn modern_slot_update(
    player_guid: ObjectGuid,
    updates: impl IntoIterator<Item = (u8, u8, Option<ObjectGuid>)>,
) -> Option<SmsgUpdateObject> {
    let mut block = ValuesUpdateBlock::new(player_guid, ObjectType::Player);
    let mut any = false;
    for (bag, slot, item_guid) in updates {
        let Some(field) = equipment_slot_field(bag, slot) else {
            continue;
        };
        // An empty slot is written as an all-zero GUID rather than omitted: the client keeps the
        // previous item in a field it is not told about, so clearing has to be an explicit write.
        block = block.set_guid_field(field, item_guid.unwrap_or_default());
        any = true;
    }
    any.then(|| SmsgUpdateObject::new().add_block(UpdateBlockData::Values(block)))
}

const MAX_VISIBLE_ITEM_OFFSET: u32 = 12;
const EQUIPMENT_SLOT_COUNT: u8 = 19;
const INVENTORY_SLOT_START: u8 = 23;
const INVENTORY_SLOT_END: u8 = 39;
const BANK_SLOT_START: u8 = 39;
const BANK_SLOT_END: u8 = 63;
const BANK_BAG_SLOT_START: u8 = 63;
const BANK_BAG_SLOT_END: u8 = 69;
const BUYBACK_SLOT_START: u8 = 69;
const BUYBACK_SLOT_END: u8 = 81;

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
    /// Both protocols express this as a field update on the player; only the slot numbering and the
    /// packet framing differ, and both of those are handled by the shared update encoder.
    ///
    /// See `equipment_slot_field` for which slots make it across.
    fn to_modern(&self) -> Option<WorldPacket> {
        modern_slot_update(self.player_guid, [(self.bag, self.slot, self.item_guid)])?
            .for_recipient(self.player_guid, 0)
            .to_modern()
    }

    /// Preferred over [`ToWorldPacket::to_modern`] when the send path knows the recipient: it
    /// carries the real map id and realm, which the modern header and every GUID in the body need.
    fn to_modern_for(&self, recipient: Recipient) -> Option<WorldPacket> {
        modern_slot_update(self.player_guid, [(self.bag, self.slot, self.item_guid)])?
            .to_modern_for(recipient)
    }

    fn to_vanilla(&self) -> WorldPacket {
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
            } else if self.slot >= BANK_SLOT_START && self.slot < BANK_SLOT_END {
                let field_low =
                    PLAYER_FIELD_BANK_SLOT_1 + ((self.slot - BANK_SLOT_START) as u32 * 2);
                let field_high = field_low + 1;

                if let Some(guid) = self.item_guid {
                    mask.set_guid(field_low, guid.low(), guid.high_u32());
                } else {
                    mask.set_field_required(field_low, 0);
                    mask.set_field_required(field_high, 0);
                }
            } else if self.slot >= BANK_BAG_SLOT_START && self.slot < BANK_BAG_SLOT_END {
                let field_low =
                    PLAYER_FIELD_BANKBAG_SLOT_1 + ((self.slot - BANK_BAG_SLOT_START) as u32 * 2);
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

/// Updates one vendor buyback slot, including the price and expiry timestamp
/// required for the client to display it in the vendor window.
#[derive(Debug, Clone)]
pub struct SmsgBuybackSlotUpdate {
    pub player_guid: ObjectGuid,
    pub slot: u8,
    pub item_guid: Option<ObjectGuid>,
    pub price: u32,
    pub timestamp: u32,
}

/// No `to_modern`: none of the three fields this writes has a 1.14 slot to write into.
///
/// The buyback item, price and expiry live in vanilla-only player fields; the generated 1.12 ->
/// 1.14 slot map has no counterpart for any of them, so encoding this would produce an update block
/// with an empty field mask -- a packet that costs bandwidth and changes nothing. Restoring the
/// buyback tab for a 1.14 client is a slot-map question, not a message-encoding one, so this is
/// left unported rather than shipped as a no-op.
impl ToWorldPacket for SmsgBuybackSlotUpdate {
    fn to_vanilla(&self) -> WorldPacket {
        debug_assert!(
            self.slot >= BUYBACK_SLOT_START && self.slot < BUYBACK_SLOT_END,
            "Buyback slot must be 69-80"
        );

        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_OBJECT);
        packet.write_u32(1);
        packet.write_u8(0);
        packet.write_u8(0);
        packet.write_packed_guid_raw(self.player_guid.raw());

        let offset = (self.slot - BUYBACK_SLOT_START) as u32;
        let field_low = PLAYER_FIELD_VENDORBUYBACK_SLOT_1 + (offset * 2);
        let field_high = field_low + 1;
        let mut mask = UpdateMask::new();
        if let Some(guid) = self.item_guid {
            mask.set_guid(field_low, guid.low(), guid.high_u32());
            mask.set_field(PLAYER_FIELD_BUYBACK_PRICE_1 + offset, self.price);
            mask.set_field(PLAYER_FIELD_BUYBACK_TIMESTAMP_1 + offset, self.timestamp);
        } else {
            mask.set_field_required(field_low, 0);
            mask.set_field_required(field_high, 0);
            mask.set_field_required(PLAYER_FIELD_BUYBACK_PRICE_1 + offset, 0);
            mask.set_field_required(PLAYER_FIELD_BUYBACK_TIMESTAMP_1 + offset, 0);
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
    /// Every slot in one block, exactly as vanilla does -- a swap must not arrive as two packets or
    /// the client briefly shows the item in both places, or in neither.
    ///
    /// See `equipment_slot_field` for which slots make it across.
    fn to_modern(&self) -> Option<WorldPacket> {
        modern_slot_update(self.player_guid, self.updates.iter().copied())?
            .for_recipient(self.player_guid, 0)
            .to_modern()
    }

    fn to_modern_for(&self, recipient: Recipient) -> Option<WorldPacket> {
        modern_slot_update(self.player_guid, self.updates.iter().copied())?.to_modern_for(recipient)
    }

    fn to_vanilla(&self) -> WorldPacket {
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
                } else if *slot >= BANK_SLOT_START && *slot < BANK_SLOT_END {
                    let field_low =
                        PLAYER_FIELD_BANK_SLOT_1 + ((*slot - BANK_SLOT_START) as u32 * 2);
                    let field_high = field_low + 1;

                    if let Some(guid) = item_guid {
                        mask.set_guid(field_low, guid.low(), guid.high_u32());
                    } else {
                        mask.set_field_required(field_low, 0);
                        mask.set_field_required(field_high, 0);
                    }
                } else if *slot >= BANK_BAG_SLOT_START && *slot < BANK_BAG_SLOT_END {
                    let field_low =
                        PLAYER_FIELD_BANKBAG_SLOT_1 + ((*slot - BANK_BAG_SLOT_START) as u32 * 2);
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

/// No `to_modern`: 1.14 restructured the visible-item fields, so they cannot be slot-translated.
///
/// Vanilla spends 12 field slots per equipment slot on the visible item -- entry, then eleven
/// enchantment slots; 1.14 replaces that with a much smaller per-slot record. The generated slot map
/// leaves the whole vanilla range unmapped because there is no field-for-field correspondence to
/// map, and writing the entry into whatever 1.14 slot happens to sit at the same offset would
/// scribble over an unrelated field. Porting this means a hand-written repack against the real 1.14
/// visible-item layout, not an index translation.
impl ToWorldPacket for SmsgVisibleItemUpdate {
    fn to_vanilla(&self) -> WorldPacket {
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

        mask.set_field_required(visible_base + 1, 0);
        mask.set_field_required(visible_base + 2, 0);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::ToWorldPacket;
    use crate::protocol::HighGuid;

    #[test]
    fn buyback_slot_update_includes_item_price_and_timestamp() {
        let player_guid = ObjectGuid::new_without_entry(HighGuid::Player, 1);
        let item_guid = ObjectGuid::new_without_entry(HighGuid::Item, 42);
        let price = 12_345;
        let timestamp = 67_890;

        let packet = SmsgBuybackSlotUpdate {
            player_guid,
            slot: 69,
            item_guid: Some(item_guid),
            price,
            timestamp,
        }
        .to_vanilla();

        assert_eq!(packet.opcode(), Opcode::SMSG_UPDATE_OBJECT);
        let data = packet.data();
        assert!(
            data.windows(4).any(|value| value == price.to_le_bytes()),
            "buyback update must contain the item's sell price"
        );
        assert!(
            data.windows(4)
                .any(|value| value == timestamp.to_le_bytes()),
            "buyback update must contain its expiry timestamp"
        );
    }

    #[test]
    fn cleared_buyback_slot_update_uses_vendor_buyback_fields() {
        let packet = SmsgBuybackSlotUpdate {
            player_guid: ObjectGuid::new_without_entry(HighGuid::Player, 1),
            slot: 80,
            item_guid: None,
            price: 0,
            timestamp: 0,
        }
        .to_vanilla();

        // Slot 80 is the last supported buyback slot. Serializing it exercises the
        // buyback field offset rather than the normal inventory field range.
        assert_eq!(packet.opcode(), Opcode::SMSG_UPDATE_OBJECT);
        assert!(!packet.data().is_empty());
    }

    /// The bug this fixes: `equipment_slot_field` used to return `None` for every slot past
    /// equipment (>= 23), so a backpack/bank/bank-bag move silently sent no modern update at all --
    /// the client's bag never changed, whether the item was picked up, moved, or sold to a vendor
    /// (selling clears the source slot through this same path).
    #[test]
    fn backpack_bank_and_bank_bag_slots_now_produce_a_modern_update() {
        let player_guid = ObjectGuid::new_without_entry(HighGuid::Player, 1);
        let item_guid = ObjectGuid::new_without_entry(HighGuid::Item, 42);

        for slot in [23u8, 38, 39, 62, 63, 68] {
            let update = SmsgInventorySlotUpdate {
                player_guid,
                bag: 255,
                slot,
                item_guid: Some(item_guid),
            };
            assert!(
                update.to_modern().is_some(),
                "slot {slot} should produce a modern update"
            );
        }
    }

    /// A bag other than 255 addresses a container object's own fields, not the player's -- there is
    /// no vanilla field for it here regardless of slot number.
    #[test]
    fn a_container_slot_has_no_player_field() {
        assert_eq!(equipment_slot_field(0, 0), None);
    }

    /// Buyback and keyring slots (69+) have no field here; `SmsgBuybackSlotUpdate` handles buyback
    /// separately, and keyring has no live caller yet, but both must stay `None` rather than alias
    /// onto the bank-bag range.
    #[test]
    fn slots_past_bank_bags_have_no_field() {
        assert_eq!(equipment_slot_field(255, BANK_BAG_SLOT_END), None);
    }
}
