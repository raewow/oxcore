//! Player-related message structs
//!
//! Contains message types for player-specific packets like money updates.

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::guid::ObjectGuid;
use crate::shared::protocol::packet::WorldPacketGuidExt;
use crate::shared::protocol::update_fields::PLAYER_FIELD_COINAGE;
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;

/// SMSG_PLAYER_MONEY_UPDATE - Send player money update
///
/// This packet format matches the old core's send_money_update():
/// - SMSG_UPDATE_OBJECT with VALUES_UPDATE block
/// - Contains PLAYER_FIELD_COINAGE field update
///
/// Packet structure:
/// - u32: block count (1)
/// - u8: hasTransport (0)
/// - u8: updateType (0 = VALUES_UPDATE)
/// - u64: player guid (packed)
/// - u32: update mask block count
/// - u32[]: update mask blocks (bitmask of updated fields)
/// - u32[]: field values (only for set bits in mask)
#[derive(Debug, Clone)]
pub struct SmsgPlayerMoneyUpdate {
    pub guid: ObjectGuid,
    pub money: u32,
}

impl ToWorldPacket for SmsgPlayerMoneyUpdate {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_OBJECT);
        packet.write_u32(1); // block count
        packet.write_u8(0); // hasTransport

        // VALUES_UPDATE block (update_type = 0)
        packet.write_u8(0);
        packet.write_packed_guid_raw(self.guid.raw());

        // UpdateMask for PLAYER_FIELD_COINAGE
        // PLAYER_FIELD_COINAGE = UNIT_END + 0x3DC = 0x3B6 in hex (950)
        // We need enough blocks to cover this field
        let field_index = PLAYER_FIELD_COINAGE;
        let block_count = (field_index / 32) + 1;

        packet.write_u8(block_count as u8);

        // Remember where the mask blocks start
        let mask_start = packet.size();

        // Initialize all blocks to 0
        for _ in 0..block_count {
            packet.write_u32(0);
        }

        // Set the bit for PLAYER_FIELD_COINAGE in the update mask
        let offset = field_index % 32;
        let block_idx = (field_index / 32) as usize;

        // CRITICAL: Actually modify the packet to set the bit
        let byte_offset = mask_start + (block_idx * 4);

        // Read the current block value (currently 0)
        let packet_data = packet.data_mut();
        let mut current_block = u32::from_le_bytes([
            packet_data[byte_offset],
            packet_data[byte_offset + 1],
            packet_data[byte_offset + 2],
            packet_data[byte_offset + 3],
        ]);

        // Set the bit for PLAYER_FIELD_COINAGE
        current_block |= 1u32 << offset;

        // Write it back to the packet
        packet_data[byte_offset..byte_offset + 4].copy_from_slice(&current_block.to_le_bytes());

        // Write the money value
        packet.write_u32(self.money);

        packet
    }
}

/// SMSG_PLAYER_INVENTORY_UPDATE - Send full inventory to player
///
/// This is a convenience wrapper around SmsgUpdateObject for sending
/// all player inventory items. Uses CREATE_OBJECT2 blocks for each item.
#[derive(Debug, Clone)]
pub struct SmsgPlayerInventoryUpdate<'a> {
    pub items: &'a [(ObjectGuid, u32, u32)], // (guid, entry, count)
}

impl<'a> ToWorldPacket for SmsgPlayerInventoryUpdate<'a> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_OBJECT);
        packet.write_u32(self.items.len() as u32);
        packet.write_u8(0); // hasTransport

        for (guid, entry, count) in self.items {
            // Write CREATE_OBJECT2 block
            packet.write_u8(3); // updateType = CREATE_OBJECT2

            // GUID
            packet.write_packed_guid_raw(guid.raw());

            // Object type (item = 3)
            packet.write_u8(3);

            // Update flags (0 for items)
            packet.write_u8(0);

            // Movement data (none for items - 0 movement flags, 0 has transport)
            packet.write_u32(0); // movement flags
            packet.write_u8(0); // has transport

            // Position (all zeros for items - they don't have a position)
            packet.write_f32(0.0); // x
            packet.write_f32(0.0); // y
            packet.write_f32(0.0); // z
            packet.write_f32(0.0); // orientation

            // Update mask - items need at least OBJECT_FIELD_GUID, OBJECT_FIELD_TYPE, OBJECT_FIELD_ENTRY
            // That's up to field 3, so 1 block (32 bits) is enough
            packet.write_u32(1); // mask block count
            packet.write_u32(0x7); // bits 0, 1, 2 set (GUID, TYPE, ENTRY)

            // Object fields
            packet.write_u64(guid.raw()); // OBJECT_FIELD_GUID
            packet.write_u32(3); // OBJECT_FIELD_TYPE (ITEM = 3)
            packet.write_u32(*entry); // OBJECT_FIELD_ENTRY

            // Item fields
            packet.write_u32(0); // ITEM_FIELD_OWNER (high)
            packet.write_u32(guid.low()); // ITEM_FIELD_OWNER (low)
            packet.write_u32(0); // ITEM_FIELD_CONTAINED (high)
            packet.write_u32(guid.low()); // ITEM_FIELD_CONTAINED (low)
            packet.write_u32(*count); // ITEM_FIELD_STACK_COUNT
        }

        packet
    }
}
