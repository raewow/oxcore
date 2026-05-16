//! Update object message structs
//!
//! Provides clean, type-safe builders for SMSG_UPDATE_OBJECT packets.
//! These structs implement the `ToWorldPacket` trait for serialization.
//!
//! # Example
//! ```rust,no_run
//! use wow_server::shared::messages::update::{
//!     SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock, ObjectType,
//!     SmsgValuesUpdate,
//! };
//! use wow_server::shared::protocol::guid::ObjectGuid;
//! use wow_server::shared::protocol::update_fields::UNIT_FIELD_HEALTH;
//!
//! // Simple VALUES_UPDATE using convenience builder
//! let guid = ObjectGuid::from_raw(0x0000000000000004);
//! let msg = SmsgValuesUpdate::new(guid, ObjectType::Unit)
//!     .set_field(UNIT_FIELD_HEALTH, 100);
//!
//! // Or using the full struct
//! let msg = SmsgUpdateObject::new()
//!     .add_block(UpdateBlockData::Values(
//!         ValuesUpdateBlock::new(guid, ObjectType::Unit)
//!             .set_field(UNIT_FIELD_HEALTH, 100)
//!     ));
//! ```

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::guid::ObjectGuid;
use crate::shared::protocol::position::Position;
use crate::shared::protocol::updates::movement_block::MovementSpeeds;
use crate::shared::protocol::updates::update_block_builder::{
    min_mask_blocks, update_flags, UpdateBlockBuilder,
};
use crate::shared::protocol::updates::update_types::ObjectTypeId;
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;

// Re-export convenience types
pub use crate::shared::protocol::updates::update_types::ObjectUpdateType;

// =========================================================================
// MAIN CONTAINER - SmsgUpdateObject
// =========================================================================

/// SMSG_UPDATE_OBJECT - Main container for update packets
///
/// Can contain multiple update blocks of any type (VALUES, CREATE_OBJECT, MOVEMENT, OUT_OF_RANGE).
///
/// This is the primary struct for constructing SMSG_UPDATE_OBJECT packets in a type-safe way.
/// Each block can represent a different type of update operation.
#[derive(Debug, Clone)]
pub struct SmsgUpdateObject {
    pub blocks: Vec<UpdateBlockData>,
    pub has_transport: bool,
}

impl SmsgUpdateObject {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            has_transport: false,
        }
    }

    pub fn with_transport(mut self) -> Self {
        self.has_transport = true;
        self
    }

    pub fn add_block(mut self, block: UpdateBlockData) -> Self {
        self.blocks.push(block);
        self
    }

    pub fn add_blocks(mut self, blocks: impl IntoIterator<Item = UpdateBlockData>) -> Self {
        self.blocks.extend(blocks);
        self
    }

    /// Merge another SmsgUpdateObject's blocks into this one
    pub fn merge(mut self, other: SmsgUpdateObject) -> Self {
        self.blocks.extend(other.blocks);
        if other.has_transport {
            self.has_transport = true;
        }
        self
    }
}

impl Default for SmsgUpdateObject {
    fn default() -> Self {
        Self::new()
    }
}

impl ToWorldPacket for SmsgUpdateObject {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_OBJECT);
        packet.write_u32(self.blocks.len() as u32);
        packet.write_u8(if self.has_transport { 1 } else { 0 });

        for block in self.blocks.iter() {
            block.write_to_packet(&mut packet);
        }

        packet
    }
}

// =========================================================================
// BLOCK DATA ENUM
// =========================================================================

/// Represents a single update block within SMSG_UPDATE_OBJECT.
///
/// Each variant corresponds to one of the four update types:
/// - VALUES: Partial field updates on existing objects
/// - CreateObject: Spawn new objects
/// - CreateObject2: Extended spawn format
/// - Movement: Position/movement updates
/// - OutOfRange: Objects that should no longer be visible
#[derive(Debug, Clone)]
pub enum UpdateBlockData {
    Values(ValuesUpdateBlock),
    CreateObject(CreateObjectBlock),
    CreateObject2(CreateObjectBlock),
    Movement(MovementUpdateBlock),
    OutOfRange(Vec<ObjectGuid>),
}

impl UpdateBlockData {
    fn write_to_packet(&self, packet: &mut WorldPacket) {
        match self {
            UpdateBlockData::Values(block) => block.write_to_packet(packet),
            UpdateBlockData::CreateObject(block) => block.write_to_packet(packet, false),
            UpdateBlockData::CreateObject2(block) => block.write_to_packet(packet, true),
            UpdateBlockData::Movement(block) => block.write_to_packet(packet),
            UpdateBlockData::OutOfRange(guids) => {
                packet.write_u8(4);
                packet.write_u32(guids.len() as u32);
                for guid in guids {
                    packet.write_packed_guid_raw(guid.raw());
                }
            }
        }
    }
}

// =========================================================================
// VALUES UPDATE BLOCK
// =========================================================================

/// VALUES_UPDATE block (UPDATETYPE_VALUES = 0).
///
/// Used for partial field updates on existing objects (health, mana, etc.).
/// Only fields that are set via the builder will be sent to the client.
#[derive(Debug, Clone)]
pub struct ValuesUpdateBlock {
    pub guid: ObjectGuid,
    pub object_type: ObjectType,
    pub fields: Vec<(u32, u32)>,
}

impl ValuesUpdateBlock {
    pub fn new(guid: ObjectGuid, object_type: ObjectType) -> Self {
        Self {
            guid,
            object_type,
            fields: Vec::new(),
        }
    }

    pub fn set_field(mut self, index: u32, value: u32) -> Self {
        self.fields.push((index, value));
        self
    }

    pub fn set_fields(mut self, fields: impl IntoIterator<Item = (u32, u32)>) -> Self {
        self.fields.extend(fields);
        self
    }

    pub fn set_guid_field(mut self, index: u32, guid: ObjectGuid) -> Self {
        let raw = guid.raw();
        // GUID fields are always included (even if 0) - push to fields directly
        // set_u32 will skip 0 values, but GUID high parts are typically non-zero
        self.fields.push((index, raw as u32));
        self.fields.push((index + 1, (raw >> 32) as u32));
        self
    }

    pub fn set_float_field(mut self, index: u32, value: f32) -> Self {
        self.fields.push((index, value.to_bits()));
        self
    }

    pub fn set_required(mut self, index: u32, value: u32) -> Self {
        self.fields.push((index, value));
        self
    }

    fn write_to_packet(&self, packet: &mut WorldPacket) {
        // VALUES updates (partial field changes) use minimal mask size -
        // only CREATE_OBJECT needs the full min_mask_blocks for the object type.
        let mut builder = UpdateBlockBuilder::values(self.guid);
        for &(index, value) in &self.fields {
            builder = builder.set_u32_required(index, value);
        }

        builder.write_to_packet(packet, 0);
    }
}

// =========================================================================
// CREATE OBJECT BLOCK
// =========================================================================

/// CREATE_OBJECT block (UPDATETYPE_CREATE_OBJECT = 2, UPDATETYPE_CREATE_OBJECT2 = 3).
///
/// Used for spawning new objects visible to the client.
/// Supports all object types (items, creatures, players, game objects, etc.).
#[derive(Debug, Clone)]
pub struct CreateObjectBlock {
    pub guid: ObjectGuid,
    pub type_id: ObjectTypeId,
    pub update_flags: u8,
    pub object_type: ObjectType,
    pub movement: Option<MovementBlockData>,
    pub melee_attacking_victim: Option<ObjectGuid>,
    pub fields: Vec<(u32, u32)>,
    pub required_fields: Vec<(u32, u32)>, // Fields that must be sent even when value is 0
    pub bytes_fields: Vec<(u32, [u8; 4])>,
}

impl CreateObjectBlock {
    pub fn new(guid: ObjectGuid, type_id: ObjectTypeId, object_type: ObjectType) -> Self {
        Self {
            guid,
            type_id,
            update_flags: update_flags::UPDATEFLAG_NONE,
            object_type,
            movement: None,
            melee_attacking_victim: None,
            fields: Vec::new(),
            required_fields: Vec::new(),
            bytes_fields: Vec::new(),
        }
    }

    pub fn with_flags(mut self, flags: u8) -> Self {
        self.update_flags |= flags;
        self
    }

    pub fn add_flags(mut self, flags: u8) -> Self {
        self.update_flags |= flags;
        self
    }

    pub fn with_position(mut self, position: Position) -> Self {
        self.update_flags |= update_flags::UPDATEFLAG_HAS_POSITION;
        self.movement = Some(MovementBlockData::Position(position));
        self
    }

    pub fn with_melee_attacking(mut self, victim: ObjectGuid) -> Self {
        self.update_flags |= update_flags::UPDATEFLAG_MELEE_ATTACKING;
        self.melee_attacking_victim = Some(victim);
        self
    }

    pub fn with_movement(
        mut self,
        position: Position,
        movement_flags: u32,
        speeds: Option<MovementSpeeds>,
    ) -> Self {
        // Set UPDATEFLAG_LIVING for full movement block with speeds
        self.update_flags |= update_flags::UPDATEFLAG_LIVING;
        // Set UPDATEFLAG_ALL to trigger post-movement u32(1) marker
        // This matches old world behavior (0x70 = 0x20 | 0x40 | 0x10)
        // Without this flag, the packet is misaligned by 4 bytes causing creatures to become invisible
        self.update_flags |= update_flags::UPDATEFLAG_ALL;
        self.movement = Some(MovementBlockData::Living {
            position,
            movement_flags,
            speeds,
        });
        self
    }

    pub fn set_field(mut self, index: u32, value: u32) -> Self {
        self.fields.push((index, value));
        self
    }

    pub fn set_fields(mut self, fields: impl IntoIterator<Item = (u32, u32)>) -> Self {
        self.fields.extend(fields);
        self
    }

    pub fn set_guid_field(mut self, index: u32, guid: ObjectGuid) -> Self {
        let raw = guid.raw();
        // GUID fields must always be included (even if 0) - use required_fields
        // This ensures both low and high parts are sent even if high is 0
        self.required_fields.push((index, raw as u32));
        self.required_fields.push((index + 1, (raw >> 32) as u32));
        self
    }

    pub fn set_float_field(mut self, index: u32, value: f32) -> Self {
        self.fields.push((index, value.to_bits()));
        self
    }

    pub fn set_required(mut self, index: u32, value: u32) -> Self {
        self.required_fields.push((index, value));
        self
    }

    pub fn set_bytes_field(mut self, index: u32, bytes: [u8; 4]) -> Self {
        self.bytes_fields.push((index, bytes));
        self
    }

    fn write_to_packet(&self, packet: &mut WorldPacket, is_create2: bool) {
        let min_blocks = self.object_type.min_mask_blocks();

        // Use the correct builder based on update type
        // CreateObject (type 2) is used for creatures, CreateObject2 (type 3) for players
        let mut builder = if is_create2 {
            UpdateBlockBuilder::create_object2(self.guid, self.type_id)
        } else {
            UpdateBlockBuilder::create_object(self.guid, self.type_id)
        }
        .with_flags(self.update_flags);

        if let Some(ref movement) = self.movement {
            match movement {
                MovementBlockData::Position(pos) => {
                    builder = builder.with_position(*pos);
                }
                MovementBlockData::Living {
                    position,
                    movement_flags,
                    speeds,
                } => {
                    if let Some(s) = speeds {
                        builder = builder.with_movement_speeds(*position, *movement_flags, *s);
                    } else {
                        builder = builder.with_movement(*position, *movement_flags);
                    }
                }
            }
        }

        // Pass melee attacking victim to the builder for UPDATEFLAG_MELEE_ATTACKING
        if let Some(victim) = self.melee_attacking_victim {
            builder = builder.with_melee_attacking(victim);
        }

        for &(index, value) in &self.fields {
            builder = builder.set_u32(index, value);
        }

        // Required fields are sent even when value is 0 (use set_u32_required)
        for &(index, value) in &self.required_fields {
            builder = builder.set_u32_required(index, value);
        }

        for &(index, bytes) in &self.bytes_fields {
            builder = builder.set_bytes(index, bytes);
        }

        builder.write_to_packet(packet, min_blocks);
    }
}

// =========================================================================
// MOVEMENT UPDATE BLOCK
// =========================================================================

/// MOVEMENT_UPDATE block (UPDATETYPE_MOVEMENT = 1).
///
/// Used for position/movement updates on living units.
/// This is more efficient than sending a full VALUES update for position changes.
#[derive(Debug, Clone)]
pub struct MovementUpdateBlock {
    pub guid: ObjectGuid,
    pub movement_flags: u32,
    pub position: Position,
    pub speeds: Option<MovementSpeeds>,
}

impl MovementUpdateBlock {
    pub fn new(guid: ObjectGuid, position: Position, movement_flags: u32) -> Self {
        Self {
            guid,
            movement_flags,
            position,
            speeds: None,
        }
    }

    pub fn with_speeds(mut self, speeds: MovementSpeeds) -> Self {
        self.speeds = Some(speeds);
        self
    }

    fn write_to_packet(&self, packet: &mut WorldPacket) {
        let mut builder = UpdateBlockBuilder::movement(self.guid);
        if let Some(ref speeds) = self.speeds {
            builder = builder.with_movement_speeds(self.position, self.movement_flags, *speeds);
        } else {
            builder = builder.with_movement(self.position, self.movement_flags);
        }
        builder.write_to_packet(packet, 0);
    }
}

// =========================================================================
// CONVENIENCE BUILDERS
// =========================================================================

/// Builder for simple VALUES_UPDATE packets.
///
/// This is a convenience wrapper that directly creates an SmsgUpdateObject
/// with a single ValuesUpdateBlock.
///
/// # Example
/// ```rust,no_run
/// use wow_server::shared::messages::update::SmsgValuesUpdate;
/// use wow_server::shared::messages::ToWorldPacket;
/// use wow_server::shared::protocol::guid::ObjectGuid;
/// use wow_server::shared::protocol::update_fields::UNIT_FIELD_HEALTH;
/// use wow_server::shared::messages::update::ObjectType;
///
/// let guid = ObjectGuid::from_raw(0x0000000000000004);
/// let packet = SmsgValuesUpdate::new(guid, ObjectType::Unit)
///     .set_field(UNIT_FIELD_HEALTH, 100)
///     .to_world_packet();
/// ```
#[derive(Debug, Clone)]
pub struct SmsgValuesUpdate {
    pub guid: ObjectGuid,
    pub object_type: ObjectType,
}

impl SmsgValuesUpdate {
    pub fn new(guid: ObjectGuid, object_type: ObjectType) -> Self {
        Self { guid, object_type }
    }

    pub fn set_field(self, index: u32, value: u32) -> SmsgUpdateObject {
        SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(self.guid, self.object_type).set_field(index, value),
        ))
    }

    pub fn set_fields(self, fields: impl IntoIterator<Item = (u32, u32)>) -> SmsgUpdateObject {
        SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(self.guid, self.object_type).set_fields(fields),
        ))
    }

    pub fn set_guid_field(self, index: u32, guid: ObjectGuid) -> SmsgUpdateObject {
        SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(self.guid, self.object_type).set_guid_field(index, guid),
        ))
    }

    pub fn set_float_field(self, index: u32, value: f32) -> SmsgUpdateObject {
        SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(self.guid, self.object_type).set_float_field(index, value),
        ))
    }
}

impl ToWorldPacket for SmsgValuesUpdate {
    fn to_world_packet(&self) -> WorldPacket {
        SmsgUpdateObject::new()
            .add_block(UpdateBlockData::Values(ValuesUpdateBlock::new(
                self.guid,
                self.object_type,
            )))
            .to_world_packet()
    }
}

// =========================================================================
// HELPER TYPES
// =========================================================================

/// Object type enumeration for determining minimum mask blocks.
///
/// Each object type has a different field count, which affects how many
/// mask blocks need to be sent in update packets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Object,
    Item,
    Container,
    Unit,
    Player,
    GameObject,
    DynamicObject,
    Corpse,
}

impl ObjectType {
    pub fn min_mask_blocks(self) -> u8 {
        match self {
            ObjectType::Object => min_mask_blocks::OBJECT,
            ObjectType::Item => min_mask_blocks::ITEM,
            ObjectType::Container => min_mask_blocks::CONTAINER,
            ObjectType::Unit => min_mask_blocks::UNIT,
            ObjectType::Player => min_mask_blocks::PLAYER,
            ObjectType::GameObject => min_mask_blocks::GAMEOBJECT,
            ObjectType::DynamicObject => min_mask_blocks::DYNAMICOBJECT,
            ObjectType::Corpse => min_mask_blocks::CORPSE,
        }
    }
}

/// Movement data for update blocks.
///
/// Living units have full movement data with flags and speeds,
/// while game objects only have position.
#[derive(Debug, Clone)]
pub enum MovementBlockData {
    Position(Position),
    Living {
        position: Position,
        movement_flags: u32,
        speeds: Option<MovementSpeeds>,
    },
}

// =========================================================================
// TESTS
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smsg_update_object_empty() {
        let msg = SmsgUpdateObject::new();
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_UPDATE_OBJECT);
    }

    #[test]
    fn test_smsg_update_object_with_values() {
        let guid = ObjectGuid::from_raw(0x0000000000000004);
        let msg = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(guid, ObjectType::Unit).set_field(22, 100),
        ));
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_UPDATE_OBJECT);
    }

    #[test]
    fn test_smsg_values_update() {
        let guid = ObjectGuid::from_raw(0x0000000000000004);
        let msg = SmsgValuesUpdate::new(guid, ObjectType::Unit).set_field(22, 100);
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_UPDATE_OBJECT);
    }

    #[test]
    fn test_values_update_block() {
        let guid = ObjectGuid::from_raw(0x0000000000000004);
        let block = ValuesUpdateBlock::new(guid, ObjectType::Unit)
            .set_field(22, 100)
            .set_field(23, 200);
        assert_eq!(block.fields.len(), 2);
    }

    #[test]
    fn test_create_object_block() {
        let guid = ObjectGuid::from_raw(0x0000000000000004);
        let pos = Position::new(100.0, 200.0, 300.0, 1.5);
        let block = CreateObjectBlock::new(guid, ObjectTypeId::Player, ObjectType::Player)
            .with_flags(update_flags::UPDATEFLAG_LIVING)
            .with_movement(pos, 0, None)
            .set_field(2, 0x19);
        assert_eq!(
            block.update_flags,
            update_flags::UPDATEFLAG_LIVING | update_flags::UPDATEFLAG_ALL
        );
        assert!(block.movement.is_some());
    }

    #[test]
    fn test_object_type_min_mask_blocks() {
        assert_eq!(ObjectType::Object.min_mask_blocks(), 1);
        assert_eq!(ObjectType::Unit.min_mask_blocks(), 6);
        assert_eq!(ObjectType::Player.min_mask_blocks(), 41);
        assert_eq!(ObjectType::Item.min_mask_blocks(), 2);
    }

    #[test]
    fn test_smsg_update_object_multiple_blocks() {
        let guid1 = ObjectGuid::from_raw(0x0000000000000004);
        let guid2 = ObjectGuid::from_raw(0x0000000000000005);

        let msg = SmsgUpdateObject::new()
            .add_block(UpdateBlockData::Values(
                ValuesUpdateBlock::new(guid1, ObjectType::Unit).set_field(22, 100),
            ))
            .add_block(UpdateBlockData::Values(
                ValuesUpdateBlock::new(guid2, ObjectType::Unit).set_field(22, 200),
            ));

        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_UPDATE_OBJECT);
        assert_eq!(msg.blocks.len(), 2);
    }
}
