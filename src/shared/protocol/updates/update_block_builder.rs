//! Builder pattern for SMSG_UPDATE_OBJECT update blocks.
//!
//! Provides a fluent, type-safe API for constructing update blocks that can be
//! used standalone or with `UpdateData` for batching multiple objects.
//!
//! # Examples
//!
//! Simple VALUES update (field changes):
//! ```ignore
//! let packet = UpdatePacketBuilder::new(
//!     UpdateBlockBuilder::values(unit_guid)
//!         .set_u32_required(UNIT_FIELD_HEALTH, 100)
//! )
//! .with_min_mask_blocks(6)
//! .build()?;
//! ```
//!
//! CREATE_OBJECT for spawning:
//! ```ignore
//! let packet = UpdatePacketBuilder::new(
//!     UpdateBlockBuilder::create_object(player_guid, ObjectTypeId::Player)
//!         .with_flags(UPDATEFLAG_LIVING)
//!         .with_movement(position, 0)
//!         .for_self()
//!         .set_guid_field(OBJECT_FIELD_GUID, player_guid)
//!         .set_u32(OBJECT_FIELD_TYPE, OBJECT_TYPE_PLAYER)
//!         .set_u32(UNIT_FIELD_HEALTH, health)
//! )
//! .with_min_mask_blocks(41)
//! .build()?;
//! ```
//!
//! Batching multiple objects with UpdateData:
//! ```ignore
//! let mut update_data = UpdateData::new();
//! for creature in creatures {
//!     let block = UpdateBlockBuilder::create_object(creature.guid(), ObjectTypeId::Unit)
//!         .with_flags(UPDATEFLAG_LIVING)
//!         .with_movement(creature.position(), 0);
//!     update_data.add_block(&block, 6);
//! }
//! let packet = update_data.build_packet(false)?;
//! ```

use anyhow::Result;
use bytes::{BufMut, BytesMut};
use tracing::{info, warn};

use super::movement_block::MovementSpeeds;
use super::packet_compression::compress_update_packet_if_needed;
use super::update_mask::UpdateMask;
use super::update_types::{ObjectTypeId, ObjectUpdateType};
use crate::shared::protocol::guid::ObjectGuid;
use crate::shared::protocol::opcodes::Opcode;
use crate::shared::protocol::position::Position;
use crate::shared::protocol::{packet::WorldPacketGuidExt, WorldPacket};

/// Update flags for object updates (from movement_block.rs)
pub mod update_flags {
    pub const UPDATEFLAG_NONE: u8 = 0x00;
    pub const UPDATEFLAG_SELF: u8 = 0x01;
    pub const UPDATEFLAG_TRANSPORT: u8 = 0x02;
    pub const UPDATEFLAG_MELEE_ATTACKING: u8 = 0x04;
    pub const UPDATEFLAG_HIGHGUID: u8 = 0x08;
    pub const UPDATEFLAG_ALL: u8 = 0x10;
    pub const UPDATEFLAG_LIVING: u8 = 0x20;
    pub const UPDATEFLAG_HAS_POSITION: u8 = 0x40;
}

/// Movement data for living units
#[derive(Debug, Clone)]
pub struct MovementData {
    pub position: Position,
    pub movement_flags: u32,
    pub speeds: Option<MovementSpeeds>,
}

/// Builder for individual update blocks within SMSG_UPDATE_OBJECT.
///
/// This builder creates a single update block that can be:
/// - Written directly to a WorldPacket via `UpdatePacketBuilder`
/// - Added to `UpdateData` for batching with other blocks
///
/// The builder uses owned self for fluent method chaining.
#[derive(Debug, Clone)]
pub struct UpdateBlockBuilder {
    update_type: ObjectUpdateType,
    guid: ObjectGuid,
    type_id: Option<ObjectTypeId>,
    update_flags: u8,
    movement: Option<MovementData>,
    melee_attacking_victim: Option<ObjectGuid>,
    mask: UpdateMask,
}

impl UpdateBlockBuilder {
    // =========================================================================
    // Static Constructors
    // =========================================================================

    /// Create a VALUES update block (UPDATETYPE_VALUES = 0).
    ///
    /// Used for partial field updates on existing objects (health, mana, etc).
    pub fn values(guid: ObjectGuid) -> Self {
        Self {
            update_type: ObjectUpdateType::Values,
            guid,
            type_id: None,
            update_flags: update_flags::UPDATEFLAG_NONE,
            movement: None,
            melee_attacking_victim: None,
            mask: UpdateMask::new(),
        }
    }

    /// Create a CREATE_OBJECT block (UPDATETYPE_CREATE_OBJECT = 2).
    ///
    /// Used for spawning new objects visible to the client.
    pub fn create_object(guid: ObjectGuid, type_id: ObjectTypeId) -> Self {
        Self {
            update_type: ObjectUpdateType::CreateObject,
            guid,
            type_id: Some(type_id),
            update_flags: update_flags::UPDATEFLAG_NONE,
            movement: None,
            melee_attacking_victim: None,
            mask: UpdateMask::new(),
        }
    }

    /// Create a CREATE_OBJECT2 block (UPDATETYPE_CREATE_OBJECT2 = 3).
    ///
    /// Extended version used for certain object types.
    pub fn create_object2(guid: ObjectGuid, type_id: ObjectTypeId) -> Self {
        Self {
            update_type: ObjectUpdateType::CreateObject2,
            guid,
            type_id: Some(type_id),
            update_flags: update_flags::UPDATEFLAG_NONE,
            movement: None,
            melee_attacking_victim: None,
            mask: UpdateMask::new(),
        }
    }

    /// Create a MOVEMENT update block (UPDATETYPE_MOVEMENT = 1).
    ///
    /// Used for movement-only updates.
    pub fn movement(guid: ObjectGuid) -> Self {
        Self {
            update_type: ObjectUpdateType::Movement,
            guid,
            type_id: None,
            update_flags: update_flags::UPDATEFLAG_NONE,
            movement: None,
            melee_attacking_victim: None,
            mask: UpdateMask::new(),
        }
    }

    // =========================================================================
    // Fluent Configuration
    // =========================================================================

    /// Set update flags directly.
    pub fn with_flags(mut self, flags: u8) -> Self {
        self.update_flags = flags;
        self
    }

    /// Add update flags (OR with existing).
    pub fn add_flags(mut self, flags: u8) -> Self {
        self.update_flags |= flags;
        self
    }

    /// Mark this update as being for the player themselves (UPDATEFLAG_SELF).
    pub fn for_self(mut self) -> Self {
        self.update_flags |= update_flags::UPDATEFLAG_SELF;
        self
    }

    /// Conditionally mark this update as being for the player themselves.
    /// Allows chaining: `.for_self_if(target_guid == player_guid)`
    pub fn for_self_if(self, condition: bool) -> Self {
        if condition {
            self.for_self()
        } else {
            self
        }
    }

    /// Add a movement block for living units (UPDATEFLAG_LIVING).
    ///
    /// Automatically sets UPDATEFLAG_LIVING.
    pub fn with_movement(mut self, position: Position, movement_flags: u32) -> Self {
        self.update_flags |= update_flags::UPDATEFLAG_LIVING;
        self.movement = Some(MovementData {
            position,
            movement_flags,
            speeds: None,
        });
        self
    }

    /// Add a movement block with custom speeds.
    ///
    /// Automatically sets UPDATEFLAG_LIVING.
    pub fn with_movement_speeds(
        mut self,
        position: Position,
        movement_flags: u32,
        speeds: MovementSpeeds,
    ) -> Self {
        self.update_flags |= update_flags::UPDATEFLAG_LIVING;
        self.movement = Some(MovementData {
            position,
            movement_flags,
            speeds: Some(speeds),
        });
        self
    }

    /// Add position-only data (UPDATEFLAG_HAS_POSITION).
    ///
    /// Used for game objects that don't have full movement.
    pub fn with_position(mut self, position: Position) -> Self {
        self.update_flags |= update_flags::UPDATEFLAG_HAS_POSITION;
        self.movement = Some(MovementData {
            position,
            movement_flags: 0,
            speeds: None,
        });
        self
    }

    /// Set UPDATEFLAG_MELEE_ATTACKING with the victim's GUID.
    ///
    /// MaNGOS adds this flag when a unit has a victim and is in melee attacking state.
    /// The client expects a packed GUID of the victim after the ALL u32 in the update block.
    pub fn with_melee_attacking(mut self, victim: ObjectGuid) -> Self {
        self.update_flags |= update_flags::UPDATEFLAG_MELEE_ATTACKING;
        self.melee_attacking_victim = Some(victim);
        self
    }

    // =========================================================================
    // Field Setting (delegates to UpdateMask)
    // =========================================================================

    /// Set a GUID field (takes 2 consecutive u32 slots).
    pub fn set_guid_field(mut self, index: u32, guid: ObjectGuid) -> Self {
        let raw = guid.raw();
        self.mask.set_guid(index, raw as u32, (raw >> 32) as u32);
        self
    }

    /// Set a u32 field. Skips if value is 0 (use set_u32_required for 0 values).
    pub fn set_u32(mut self, index: u32, value: u32) -> Self {
        self.mask.set_field(index, value);
        self
    }

    /// Set a u32 field, including 0 values (required fields).
    pub fn set_u32_required(mut self, index: u32, value: u32) -> Self {
        self.mask.set_field_required(index, value);
        self
    }

    /// Set a float field.
    pub fn set_f32(mut self, index: u32, value: f32) -> Self {
        self.mask.set_float_field(index, value);
        self
    }

    /// Set a bytes field (4 packed bytes).
    pub fn set_bytes(mut self, index: u32, bytes: [u8; 4]) -> Self {
        self.mask.set_bytes_field(index, bytes);
        self
    }

    /// Set multiple u32 fields at once.
    pub fn set_u32_fields(mut self, fields: &[(u32, u32)]) -> Self {
        for &(index, value) in fields {
            self.mask.set_field(index, value);
        }
        self
    }

    /// Get mutable access to the internal UpdateMask for advanced usage.
    pub fn mask_mut(&mut self) -> &mut UpdateMask {
        &mut self.mask
    }

    /// Get read access to the internal UpdateMask.
    pub fn mask(&self) -> &UpdateMask {
        &self.mask
    }

    /// Replace the internal UpdateMask with a pre-populated one.
    /// Useful when fields are populated by external logic (e.g., Player::populate_create_fields).
    pub fn with_mask(mut self, mask: UpdateMask) -> Self {
        self.mask = mask;
        self
    }

    /// Populate the mask using a closure.
    /// Allows external code to populate the mask while maintaining the builder chain.
    pub fn populate_mask<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut UpdateMask),
    {
        f(&mut self.mask);
        self
    }

    // =========================================================================
    // Build / Write
    // =========================================================================

    /// Write this update block to a BytesMut buffer.
    ///
    /// Used by `UpdateData::add_block()` for batching.
    ///
    /// `min_mask_blocks`: Minimum mask blocks (6 for units, 41 for players).
    pub fn write_to_buffer(&self, buf: &mut BytesMut, min_mask_blocks: u8) {
        // Update type
        buf.put_u8(self.update_type.as_u8());

        // Packed GUID
        write_packed_guid_to_buf(buf, self.guid);

        // Object type (only for CREATE_OBJECT / CREATE_OBJECT2)
        if let Some(type_id) = self.type_id {
            buf.put_u8(type_id.as_u8());
        }

        // Update flags (for CREATE_OBJECT / CREATE_OBJECT2 and MOVEMENT)
        if self.type_id.is_some() || self.update_type == ObjectUpdateType::Movement {
            buf.put_u8(self.update_flags);
        }

        // Movement block
        if let Some(ref movement) = self.movement {
            self.write_movement_block(buf, movement);
        }

        // Post-movement flags (MaNGOS order: HIGHGUID, ALL, MELEE_ATTACKING, TRANSPORT)
        if self.update_flags & update_flags::UPDATEFLAG_HIGHGUID != 0 {
            buf.put_u32_le(0);
        }

        if self.update_flags & update_flags::UPDATEFLAG_ALL != 0 {
            buf.put_u32_le(1);
        }

        if self.update_flags & update_flags::UPDATEFLAG_MELEE_ATTACKING != 0 {
            if let Some(victim) = self.melee_attacking_victim {
                write_packed_guid_to_buf(buf, victim);
            } else {
                buf.put_u8(0); // empty packed GUID mask
            }
        }

        // Update mask and values
        self.write_mask_to_buffer(buf, min_mask_blocks);
    }

    /// Write this update block to a WorldPacket.
    ///
    /// `min_mask_blocks`: Minimum mask blocks (6 for units, 41 for players).
    pub fn write_to_packet(&self, packet: &mut WorldPacket, min_mask_blocks: u8) {
        // Update type
        packet.write_u8(self.update_type.as_u8());

        // Packed GUID
        packet.write_packed_guid(self.guid);

        // Object type (only for CREATE_OBJECT / CREATE_OBJECT2)
        if let Some(type_id) = self.type_id {
            packet.write_u8(type_id.as_u8());
        }

        // Update flags (for CREATE_OBJECT / CREATE_OBJECT2 and MOVEMENT)
        if self.type_id.is_some() || self.update_type == ObjectUpdateType::Movement {
            packet.write_u8(self.update_flags);
        }

        // Movement block
        if let Some(ref movement) = self.movement {
            self.write_movement_block_to_packet(packet, movement);
        }

        // Post-movement flags (MaNGOS order: HIGHGUID, ALL, MELEE_ATTACKING, TRANSPORT)
        if self.update_flags & update_flags::UPDATEFLAG_HIGHGUID != 0 {
            packet.write_u32(0);
        }

        if self.update_flags & update_flags::UPDATEFLAG_ALL != 0 {
            packet.write_u32(1);
        }

        if self.update_flags & update_flags::UPDATEFLAG_MELEE_ATTACKING != 0 {
            if let Some(victim) = self.melee_attacking_victim {
                packet.write_packed_guid(victim);
            } else {
                packet.write_u8(0); // empty packed GUID mask
            }
        }

        // Update mask and values
        self.mask
            .write_to_packet_with_min_blocks(packet, min_mask_blocks);
    }

    // =========================================================================
    // Private Helpers
    // =========================================================================

    fn write_movement_block(&self, buf: &mut BytesMut, movement: &MovementData) {
        if self.update_flags & update_flags::UPDATEFLAG_LIVING != 0 {
            // Full living unit movement block
            buf.put_u32_le(movement.movement_flags);

            // Server timestamp
            let server_time = crate::shared::common::server_mstime();
            buf.put_u32_le(server_time);

            // Position - validate and normalize orientation before writing
            let mut normalized_position = movement.position;
            let orientation_valid = normalized_position.validate_orientation();
            if !orientation_valid {
                warn!(
                    "[UpdateBlockBuilder] ⚠️ Invalid orientation in BytesMut movement block: {:.4} (NaN or infinite). Normalized to {:.4}",
                    movement.position.o,
                    normalized_position.o
                );
            }
            buf.put_f32_le(normalized_position.x);
            buf.put_f32_le(normalized_position.y);
            buf.put_f32_le(normalized_position.z);
            buf.put_f32_le(normalized_position.o);

            // Fall time
            buf.put_u32_le(0);

            // Movement speeds
            let speeds = movement.speeds.as_ref().cloned().unwrap_or_default();
            buf.put_f32_le(speeds.walk);
            buf.put_f32_le(speeds.run);
            buf.put_f32_le(speeds.run_back);
            buf.put_f32_le(speeds.swim);
            buf.put_f32_le(speeds.swim_back);
            buf.put_f32_le(speeds.turn_rate);
        } else if self.update_flags & update_flags::UPDATEFLAG_HAS_POSITION != 0 {
            // Position-only block (game objects) - validate and normalize orientation
            let mut normalized_position = movement.position;
            let orientation_valid = normalized_position.validate_orientation();
            if !orientation_valid {
                warn!(
                    "[UpdateBlockBuilder] ⚠️ Invalid orientation in BytesMut position block: {:.4} (NaN or infinite). Normalized to {:.4}",
                    movement.position.o,
                    normalized_position.o
                );
            }
            buf.put_f32_le(normalized_position.x);
            buf.put_f32_le(normalized_position.y);
            buf.put_f32_le(normalized_position.z);
            buf.put_f32_le(normalized_position.o);
        }
    }

    fn write_movement_block_to_packet(&self, packet: &mut WorldPacket, movement: &MovementData) {
        if self.update_flags & update_flags::UPDATEFLAG_LIVING != 0 {
            // Full living unit movement block
            packet.write_u32(movement.movement_flags);

            // Server uptime timestamp (matches WoW 1.12 client's time domain)
            let server_time = crate::shared::common::server_mstime();
            packet.write_u32(server_time);

            // Position - validate and normalize orientation before writing
            let mut normalized_position = movement.position;
            let orientation_valid = normalized_position.validate_orientation();
            if !orientation_valid {
                warn!(
                    "[UpdateBlockBuilder] ⚠️ Invalid orientation in movement block: {:.4} (NaN or infinite). Normalized to {:.4}",
                    movement.position.o,
                    normalized_position.o
                );
            }
            packet.write_f32(normalized_position.x);
            packet.write_f32(normalized_position.y);
            packet.write_f32(normalized_position.z);
            packet.write_f32(normalized_position.o);

            // Fall time
            packet.write_u32(0);

            // Movement speeds
            let speeds = movement.speeds.as_ref().cloned().unwrap_or_default();
            packet.write_f32(speeds.walk);
            packet.write_f32(speeds.run);
            packet.write_f32(speeds.run_back);
            packet.write_f32(speeds.swim);
            packet.write_f32(speeds.swim_back);
            packet.write_f32(speeds.turn_rate);
        } else if self.update_flags & update_flags::UPDATEFLAG_HAS_POSITION != 0 {
            // Position-only block (game objects) - validate and normalize orientation
            let mut normalized_position = movement.position;
            let orientation_valid = normalized_position.validate_orientation();
            if !orientation_valid {
                warn!(
                    "[UpdateBlockBuilder] ⚠️ Invalid orientation in position block: {:.4} (NaN or infinite). Normalized to {:.4}",
                    movement.position.o,
                    normalized_position.o
                );
            }
            packet.write_f32(normalized_position.x);
            packet.write_f32(normalized_position.y);
            packet.write_f32(normalized_position.z);
            packet.write_f32(normalized_position.o);
        }
    }

    fn write_mask_to_buffer(&self, buf: &mut BytesMut, min_blocks: u8) {
        let total_fields = self.mask.field_count();
        if total_fields == 0 {
            buf.put_u8(0);
            return;
        }

        let calculated_blocks = self.mask.block_count();
        let block_count = calculated_blocks.max(min_blocks);
        let mask_blocks = self.mask.build_mask_blocks_with_min(min_blocks);

        // Write block count
        buf.put_u8(block_count);

        // Write mask blocks
        for block in &mask_blocks {
            buf.put_u32_le(*block);
        }

        // Write field values in ascending order
        let sorted_fields = self.mask.get_sorted_fields();
        for (_idx, value) in sorted_fields {
            buf.put_u32_le(value);
        }

        // Write float fields
        // Note: UpdateMask stores floats separately, we need to handle them
        // For now, this is handled by UpdateMask::write_to_packet_with_min_blocks
    }
}

/// Minimum mask blocks required for each object type.
/// Based on field counts: (FIELD_END + 31) / 32
pub mod min_mask_blocks {
    /// OBJECT_END = 6, so (6 + 31) / 32 = 1
    pub const OBJECT: u8 = 1;
    /// ITEM_END = 56, so (56 + 31) / 32 = 3 (but docs show 2 blocks used)
    pub const ITEM: u8 = 2;
    /// CONTAINER_END = 82, so (82 + 31) / 32 = 4
    pub const CONTAINER: u8 = 4;
    /// UNIT_END = 188, so (188 + 31) / 32 = 6
    pub const UNIT: u8 = 6;
    /// PLAYER_END = 1276, so (1276 + 31) / 32 = 41
    pub const PLAYER: u8 = 41;
    /// GAMEOBJECT_END = 18, so (18 + 31) / 32 = 2
    pub const GAMEOBJECT: u8 = 2;
    /// DYNAMICOBJECT_END = 12, so (12 + 31) / 32 = 1
    pub const DYNAMICOBJECT: u8 = 1;
    /// CORPSE_END = 34, so (34 + 31) / 32 = 2
    pub const CORPSE: u8 = 2;
}

/// Builder for complete SMSG_UPDATE_OBJECT packets.
///
/// Wraps a single update block and handles the packet header and compression.
#[derive(Debug)]
pub struct UpdatePacketBuilder {
    has_transport: bool,
    block: UpdateBlockBuilder,
    min_mask_blocks: u8,
}

impl UpdatePacketBuilder {
    /// Create a new packet builder with the given update block.
    pub fn new(block: UpdateBlockBuilder) -> Self {
        Self {
            has_transport: false,
            block,
            min_mask_blocks: 0,
        }
    }

    /// Mark the packet as having transport data.
    pub fn with_transport(mut self) -> Self {
        self.has_transport = true;
        self
    }

    /// Set minimum mask blocks manually.
    pub fn with_min_mask_blocks(mut self, blocks: u8) -> Self {
        self.min_mask_blocks = blocks;
        self
    }

    // =========================================================================
    // Object Type Convenience Methods
    // =========================================================================

    /// Configure for a Unit update (creatures, NPCs).
    /// Sets min_mask_blocks = 6 (UNIT_END = 188).
    pub fn for_unit(mut self) -> Self {
        self.min_mask_blocks = min_mask_blocks::UNIT;
        self
    }

    /// Configure for a Player update.
    /// Sets min_mask_blocks = 41 (PLAYER_END = 1276).
    pub fn for_player(mut self) -> Self {
        self.min_mask_blocks = min_mask_blocks::PLAYER;
        self
    }

    /// Configure for an Item update.
    /// Sets min_mask_blocks = 2.
    pub fn for_item(mut self) -> Self {
        self.min_mask_blocks = min_mask_blocks::ITEM;
        self
    }

    /// Configure for a Container (bag) update.
    /// Sets min_mask_blocks = 4 (CONTAINER_END = 82).
    pub fn for_container(mut self) -> Self {
        self.min_mask_blocks = min_mask_blocks::CONTAINER;
        self
    }

    /// Configure for a GameObject update.
    /// Sets min_mask_blocks = 2 (GAMEOBJECT_END = 18).
    pub fn for_gameobject(mut self) -> Self {
        self.min_mask_blocks = min_mask_blocks::GAMEOBJECT;
        self
    }

    /// Configure for a Corpse update.
    /// Sets min_mask_blocks = 2 (CORPSE_END = 34).
    pub fn for_corpse(mut self) -> Self {
        self.min_mask_blocks = min_mask_blocks::CORPSE;
        self
    }

    /// Build the final packet.
    ///
    /// Automatically compresses if the packet exceeds the threshold.
    pub fn build(self) -> Result<WorldPacket> {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_OBJECT);

        // Header
        packet.write_u32(1); // block count = 1
        packet.write_u8(if self.has_transport { 1 } else { 0 });

        // Write the update block
        self.block
            .write_to_packet(&mut packet, self.min_mask_blocks);

        // Compress if needed
        compress_update_packet_if_needed(packet)
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Write a packed GUID to a BytesMut buffer.
fn write_packed_guid_to_buf(buf: &mut BytesMut, guid: ObjectGuid) {
    let guid_raw = guid.raw();
    let mut mask: u8 = 0;
    let mut guid_bytes = Vec::new();

    let mut temp_guid = guid_raw;
    for i in 0..8 {
        if temp_guid == 0 {
            break;
        }

        let byte = (temp_guid & 0xFF) as u8;
        if byte != 0 {
            mask |= 1 << i;
            guid_bytes.push(byte);
        }

        temp_guid >>= 8;
    }

    buf.put_u8(mask);
    for &byte in &guid_bytes {
        buf.put_u8(byte);
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_values_update_builder() {
        let guid = ObjectGuid::from_raw(0x0000000000000004);
        let block = UpdateBlockBuilder::values(guid).set_u32_required(22, 100); // UNIT_FIELD_HEALTH

        assert_eq!(block.update_type, ObjectUpdateType::Values);
        assert!(block.mask.has_field(22));
    }

    #[test]
    fn test_create_object_builder() {
        let guid = ObjectGuid::from_raw(0x0000000000000004);
        let pos = Position {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            o: 0.0,
        };

        let block = UpdateBlockBuilder::create_object(guid, ObjectTypeId::Player)
            .with_movement(pos, 0)
            .for_self()
            .set_u32(2, 0x19); // OBJECT_FIELD_TYPE

        assert_eq!(block.update_type, ObjectUpdateType::CreateObject);
        assert_eq!(block.type_id, Some(ObjectTypeId::Player));
        assert!(block.update_flags & update_flags::UPDATEFLAG_LIVING != 0);
        assert!(block.update_flags & update_flags::UPDATEFLAG_SELF != 0);
    }

    #[test]
    fn test_packet_builder() {
        let guid = ObjectGuid::from_raw(0x0000000000000004);
        let block = UpdateBlockBuilder::values(guid).set_u32_required(22, 100);

        let packet = UpdatePacketBuilder::new(block)
            .with_min_mask_blocks(6)
            .build()
            .unwrap();

        assert!(packet.size() > 0);
    }

    #[test]
    fn test_packed_guid_simple() {
        let mut buf = BytesMut::new();
        let guid = ObjectGuid::from_raw(0x04);
        write_packed_guid_to_buf(&mut buf, guid);

        // Mask should be 0x01 (bit 0 set), byte should be 0x04
        assert_eq!(buf[0], 0x01);
        assert_eq!(buf[1], 0x04);
    }
}
