//! Update object message structs
//!
//! Provides clean, type-safe builders for SMSG_UPDATE_OBJECT packets.
//! These structs implement the `ToWorldPacket` trait for serialization.
//!
//! # Example
//! ```rust,no_run
//! use oxcore_shared::messages::update::{
//!     SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock, ObjectType,
//!     SmsgValuesUpdate,
//! };
//! use oxcore_shared::protocol::guid::ObjectGuid;
//! use oxcore_shared::protocol::update_fields::UNIT_FIELD_HEALTH;
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

use crate::messages::{Recipient, ToWorldPacket};
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::guid::ObjectGuid;
use crate::protocol::position::Position;
use crate::protocol::updates::modern::{
    ModernCreateData, ModernObjectType, ModernUpdateBlock, ModernUpdateType,
};
use crate::protocol::updates::movement_block::MovementSpeeds;
use crate::protocol::updates::update_block_builder::{
    min_mask_blocks, update_flags, UpdateBlockBuilder,
};
use crate::protocol::updates::update_types::ObjectTypeId;
use crate::protocol::Opcode;
use crate::protocol::WorldPacket;

// Re-export convenience types
pub use crate::protocol::updates::update_types::ObjectUpdateType;

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
    /// Map the recipient is standing on. Modern bodies carry this in the header; vanilla does not,
    /// so it defaults to 0 and must be set by callers that a modern client might receive.
    pub map_id: u16,
    /// The recipient's own object, when this update is aimed at one player.
    ///
    /// Modern splits the player field table in two, and the self-only half (xp, coinage, inventory,
    /// skills) only exists under the `ActivePlayer` type. Without this the recipient's own object
    /// encodes as a plain `Player` and every self-only field is silently dropped.
    pub self_guid: Option<ObjectGuid>,
    /// Objects the client should destroy outright, as opposed to the out-of-range blocks.
    ///
    /// Vanilla has a separate `SMSG_DESTROY_OBJECT` for this; 1.14 folds it in here, which is why
    /// this has no vanilla counterpart and only [`ToWorldPacket::to_modern`] reads it.
    pub destroyed: Vec<ObjectGuid>,
}

impl SmsgUpdateObject {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            has_transport: false,
            map_id: 0,
            self_guid: None,
            destroyed: Vec::new(),
        }
    }

    /// Tell the client to destroy an object.
    ///
    /// Modern-only: vanilla sends [`SmsgDestroyItem`] as its own packet instead.
    pub fn destroy(mut self, guid: ObjectGuid) -> Self {
        self.destroyed.push(guid);
        self
    }

    /// Set the map id and the recipient, both of which only the modern body uses.
    pub fn for_recipient(mut self, self_guid: ObjectGuid, map_id: u16) -> Self {
        self.self_guid = Some(self_guid);
        self.map_id = map_id;
        self
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_OBJECT);
        packet.write_u32(self.blocks.len() as u32);
        packet.write_u8(if self.has_transport { 1 } else { 0 });

        for block in self.blocks.iter() {
            block.write_to_packet(&mut packet);
        }

        packet
    }

    /// The modern body reshapes the packet rather than just renumbering fields.
    ///
    /// Out-of-range objects move out of the block list into a removal list in the header, the
    /// object blocks are length-prefixed as one chunk, and each block's fields are translated
    /// through the 1.14 slot map.
    fn to_modern(&self) -> Option<WorldPacket> {
        self.encode_modern(self.self_guid, self.map_id, DEFAULT_REALM_ID)
    }

    /// Prefers the recipient the send path supplies over anything baked into the message.
    ///
    /// A broadcast reaches many players, and only one of them owns the object being described, so
    /// the message itself cannot know. `self_guid`/`map_id` remain as a fallback for the paths that
    /// build an update for one known player.
    fn to_modern_for(&self, recipient: Recipient) -> Option<WorldPacket> {
        self.encode_modern(Some(recipient.guid), recipient.map_id, recipient.realm_id)
    }
}

impl SmsgUpdateObject {
    fn encode_modern(
        &self,
        self_guid: Option<ObjectGuid>,
        map_id: u16,
        realm_id: u16,
    ) -> Option<WorldPacket> {
        let mut removed: Vec<ObjectGuid> = Vec::new();
        let mut blocks: Vec<ModernUpdateBlock> = Vec::new();

        for block in &self.blocks {
            match block {
                UpdateBlockData::Values(values) => {
                    blocks.push(values.to_modern(self_guid, realm_id))
                }
                UpdateBlockData::CreateObject(create) => blocks.push(create.to_modern(
                    ModernUpdateType::CreateObject1,
                    self_guid,
                    realm_id,
                )),
                UpdateBlockData::CreateObject2(create) => blocks.push(create.to_modern(
                    ModernUpdateType::CreateObject2,
                    self_guid,
                    realm_id,
                )),
                UpdateBlockData::OutOfRange(guids) => removed.extend(guids.iter().copied()),
                // 1.14 has no standalone movement block inside SMSG_UPDATE_OBJECT -- movement
                // arrives as its own SMSG_MOVE_* message. Dropping is correct here; until those
                // are ported a modern client sees objects teleport rather than walk.
                UpdateBlockData::Movement(_) => {}
            }
        }

        let mut writer = BitWriter::new();
        writer.write_u32(blocks.len() as u32);
        writer.write_u16(map_id);

        writer.write_bit(!removed.is_empty() || !self.destroyed.is_empty());
        if !removed.is_empty() || !self.destroyed.is_empty() {
            // Two counts, then the two lists back to back: destroyed objects first, then the ones
            // that merely went out of range. Destroying plays a removal effect where going out of
            // range just forgets the object, so the split is visible in-game.
            writer.write_u16(self.destroyed.len() as u16);
            writer.write_i32((self.destroyed.len() + removed.len()) as i32);
            for guid in self.destroyed.iter().chain(&removed) {
                let (high, low) = guid.to_guid128(realm_id);
                writer.write_packed_guid_128(high, low);
            }
        }

        let mut data = BitWriter::new();
        for block in &blocks {
            block.write_to(&mut data, realm_id);
        }
        let data = data.into_bytes();

        writer.write_i32(data.len() as i32);
        writer.write_bytes(&data);

        Some(writer.finish(Opcode::SMSG_UPDATE_OBJECT))
    }
}

/// Realm assumed when a caller has no recipient to take one from.
///
/// Only [`ToWorldPacket::to_modern`] uses it; the real send path goes through `to_modern_for` and
/// carries the recipient's actual realm. Keeping it here rather than threading an `Option` means a
/// direct `to_modern` call still produces a coherent body on a single-realm server.
pub const DEFAULT_REALM_ID: u16 = 1;

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

    fn to_modern(&self, self_guid: Option<ObjectGuid>, realm_id: u16) -> ModernUpdateBlock {
        let object_type = self.object_type.to_modern(self_guid == Some(self.guid));
        let mut block =
            ModernUpdateBlock::new(ModernUpdateType::Values, self.guid, object_type, realm_id);
        for &(index, value) in &self.fields {
            block.fields.set_vanilla(index, value);
        }
        block
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

    fn to_modern(
        &self,
        update_type: ModernUpdateType,
        self_guid: Option<ObjectGuid>,
        realm_id: u16,
    ) -> ModernUpdateBlock {
        // Vanilla marks the recipient's own object with a flag on the block; the caller-supplied
        // guid is the more reliable signal, so either will do.
        let is_self =
            self_guid == Some(self.guid) || self.update_flags & update_flags::UPDATEFLAG_SELF != 0;
        let object_type = ModernObjectType::from_vanilla(self.type_id, is_self);

        let mut block = ModernUpdateBlock::new(update_type, self.guid, object_type, realm_id);

        let (position, movement_flags, speeds, has_position_data) = match &self.movement {
            Some(MovementBlockData::Position(position)) => (*position, 0, None, true),
            Some(MovementBlockData::Living {
                position,
                movement_flags,
                speeds,
            }) => (*position, *movement_flags, *speeds, true),
            // Items and containers are created without any position; see `has_position_data`.
            None => (Position::default(), 0, None, false),
        };

        block.create = Some(ModernCreateData {
            position,
            movement_flags,
            speeds,
            has_position_data,
            this_is_you: is_self,
            combat_victim: self.melee_attacking_victim,
        });

        for &(index, value) in self.fields.iter().chain(&self.required_fields) {
            block.fields.set_vanilla(index, value);
        }
        for &(index, bytes) in &self.bytes_fields {
            block.fields.set_vanilla(index, u32::from_le_bytes(bytes));
        }

        block
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
/// use oxcore_shared::messages::update::SmsgValuesUpdate;
/// use oxcore_shared::messages::ToWorldPacket;
/// use oxcore_shared::protocol::guid::ObjectGuid;
/// use oxcore_shared::protocol::update_fields::UNIT_FIELD_HEALTH;
/// use oxcore_shared::messages::update::ObjectType;
///
/// let guid = ObjectGuid::from_raw(0x0000000000000004);
/// let packet = SmsgValuesUpdate::new(guid, ObjectType::Unit)
///     .set_field(UNIT_FIELD_HEALTH, 100)
///     .to_vanilla();
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
    fn to_vanilla(&self) -> WorldPacket {
        SmsgUpdateObject::new()
            .add_block(UpdateBlockData::Values(ValuesUpdateBlock::new(
                self.guid,
                self.object_type,
            )))
            .to_vanilla()
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
    /// Map to the 1.14 numbering, promoting the recipient's own object to `ActivePlayer`.
    fn to_modern(self, is_self: bool) -> ModernObjectType {
        match self {
            ObjectType::Object => ModernObjectType::Object,
            ObjectType::Item => ModernObjectType::Item,
            ObjectType::Container => ModernObjectType::Container,
            ObjectType::Unit => ModernObjectType::Unit,
            ObjectType::Player if is_self => ModernObjectType::ActivePlayer,
            ObjectType::Player => ModernObjectType::Player,
            ObjectType::GameObject => ModernObjectType::GameObject,
            ObjectType::DynamicObject => ModernObjectType::DynamicObject,
            ObjectType::Corpse => ModernObjectType::Corpse,
        }
    }

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
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_UPDATE_OBJECT);
    }

    #[test]
    fn test_smsg_update_object_with_values() {
        let guid = ObjectGuid::from_raw(0x0000000000000004);
        let msg = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(guid, ObjectType::Unit).set_field(22, 100),
        ));
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_UPDATE_OBJECT);
    }

    #[test]
    fn test_smsg_values_update() {
        let guid = ObjectGuid::from_raw(0x0000000000000004);
        let msg = SmsgValuesUpdate::new(guid, ObjectType::Unit).set_field(22, 100);
        let packet = msg.to_vanilla();
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

    /// The header is a count, a map id, then a removal bit that flushes into its own byte before
    /// the length-prefixed block chunk.
    #[test]
    fn modern_empty_update_has_header_and_zero_length_chunk() {
        let packet = SmsgUpdateObject::new().to_modern().expect("ported");

        assert_eq!(packet.opcode(), Opcode::SMSG_UPDATE_OBJECT);
        assert_eq!(
            packet.contents(),
            &[
                0x00, 0x00, 0x00, 0x00, // NumObjUpdates
                0x00, 0x00, // MapID
                0x00, // no removals; the bit flushes to a whole byte
                0x00, 0x00, 0x00, 0x00, // block chunk length
            ][..]
        );
    }

    /// Out-of-range objects leave the block list entirely in 1.14 and become a removal list in the
    /// header, so the update count stays at zero.
    #[test]
    fn modern_out_of_range_moves_into_the_header() {
        let msg = SmsgUpdateObject::new()
            .add_block(UpdateBlockData::OutOfRange(vec![ObjectGuid::from_low(7)]));
        let packet = msg.to_modern().expect("ported");

        assert_eq!(
            packet.contents(),
            &[
                0x00, 0x00, 0x00, 0x00, // NumObjUpdates: the removal is not an object update
                0x00, 0x00, // MapID
                0x80, // removal bit set, MSB-first, then flushed
                0x00, 0x00, // DestroyedCount: out of range is not the same as destroyed
                0x01, 0x00, 0x00, 0x00, // destroyed + out of range
                0x01, 0xA0, 0x07, 0x04, 0x08, // guid128
                0x00, 0x00, 0x00, 0x00, // block chunk length
            ][..]
        );
    }

    /// A values block is a type byte, a guid128, the full-width mask, the set values, then the
    /// empty dynamic mask. Getting the mask width wrong desynchronises every later block.
    #[test]
    fn modern_values_block_layout() {
        let msg = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            // UNIT_FIELD_HEALTH, which lands at modern slot 55.
            ValuesUpdateBlock::new(ObjectGuid::from_low(4), ObjectType::Unit).set_field(22, 100),
        ));
        let packet = msg.to_modern().expect("ported");
        let body = packet.contents();

        let chunk_len = u32::from_le_bytes(body[7..11].try_into().unwrap()) as usize;
        let block = &body[11..];
        assert_eq!(
            block.len(),
            chunk_len,
            "chunk length must match what follows"
        );

        assert_eq!(block[0], 0, "UpdateTypeModern::Values");
        assert_eq!(&block[1..6], &[0x01, 0xA0, 0x04, 0x04, 0x08], "guid128");

        let mask = &block[6..];
        assert_eq!(mask[0], 7, "Unit has 218 slots = 7 blocks");
        // Slot 55 is block 1, bit 23.
        assert_eq!(&mask[5..9], &(1u32 << 23).to_le_bytes());
        assert_eq!(&mask[1 + 7 * 4..1 + 7 * 4 + 4], &100u32.to_le_bytes());
        // Then the empty dynamic mask: Unit has 3 dynamic slots, so one block.
        assert_eq!(&mask[1 + 7 * 4 + 4..], &[0x01, 0x00, 0x00, 0x00, 0x00]);
    }

    /// The same broadcast encodes differently per recipient: whoever owns the object gets
    /// `ThisIsYou` and the larger `ActivePlayer` field table, everyone else does not.
    #[test]
    fn modern_recipient_decides_who_owns_the_object() {
        let owner = ObjectGuid::from_low(4);
        let bystander = ObjectGuid::from_low(5);
        let msg =
            SmsgUpdateObject::new().add_block(UpdateBlockData::CreateObject2(
                CreateObjectBlock::new(owner, ObjectTypeId::Player, ObjectType::Player)
                    .with_movement(Position::new(1.0, 2.0, 3.0, 0.0), 0, None),
            ));

        let to_owner = msg
            .to_modern_for(Recipient {
                guid: owner,
                map_id: 1,
                realm_id: DEFAULT_REALM_ID,
            })
            .expect("ported");
        let to_bystander = msg
            .to_modern_for(Recipient {
                guid: bystander,
                map_id: 1,
                realm_id: DEFAULT_REALM_ID,
            })
            .expect("ported");

        assert_eq!(&to_owner.contents()[4..6], &1u16.to_le_bytes(), "MapID");

        // ObjectTypeBCC: ActivePlayer is 5, Player is 4.
        assert_eq!(to_owner.contents()[17], 5);
        assert_eq!(to_bystander.contents()[17], 4);

        // ThisIsYou and ActivePlayer are create bits 14 and 16 of 18, MSB-first: byte 1 bit 6 and
        // byte 2 bit 7.
        let owner_bits = &to_owner.contents()[22..25];
        let bystander_bits = &to_bystander.contents()[22..25];
        assert_eq!(owner_bits[1] & 0x02, 0x02, "ThisIsYou");
        assert_eq!(owner_bits[2] & 0x80, 0x80, "ActivePlayer");
        assert_eq!(bystander_bits[1] & 0x02, 0, "not ThisIsYou");
        assert_eq!(bystander_bits[2] & 0x80, 0, "not ActivePlayer");
    }

    /// Destroying counts separately from going out of range: the client plays a removal effect for
    /// one and silently forgets the other.
    #[test]
    fn modern_destroy_is_counted_apart_from_out_of_range() {
        let msg = SmsgUpdateObject::new()
            .destroy(ObjectGuid::from_low(9))
            .add_block(UpdateBlockData::OutOfRange(vec![ObjectGuid::from_low(7)]));
        let packet = msg.to_modern().expect("ported");

        assert_eq!(
            packet.contents(),
            &[
                0x00, 0x00, 0x00, 0x00, // NumObjUpdates
                0x00, 0x00, // MapID
                0x80, // removal bit
                0x01, 0x00, // DestroyedCount
                0x02, 0x00, 0x00, 0x00, // destroyed + out of range
                0x01, 0xA0, 0x09, 0x04, 0x08, // destroyed guid first
                0x01, 0xA0, 0x07, 0x04, 0x08, // then out of range
                0x00, 0x00, 0x00, 0x00, // block chunk length
            ][..]
        );
    }

    /// A create block opens with 18 presence bits that flush to three bytes, then the movement
    /// block. If the bit count or the flush is off, the client reads the mover GUID out of the
    /// middle of the header and the whole packet is lost.
    #[test]
    fn modern_create_block_bit_header_and_movement_layout() {
        let guid = ObjectGuid::from_low(4);
        let msg = SmsgUpdateObject::new().add_block(UpdateBlockData::CreateObject(
            CreateObjectBlock::new(guid, ObjectTypeId::Unit, ObjectType::Unit).with_movement(
                Position::new(1.0, 2.0, 3.0, 0.5),
                0,
                None,
            ),
        ));
        let packet = msg.to_modern().expect("ported");
        let body = packet.contents();
        let block = &body[11..];

        assert_eq!(block[0], 1, "UpdateTypeModern::CreateObject1");
        assert_eq!(&block[1..6], &[0x01, 0xA0, 0x04, 0x04, 0x08], "guid128");
        assert_eq!(block[6], 3, "ObjectTypeBCC::Unit");
        assert_eq!(
            &block[7..11],
            &(0x001i32 | 0x008).to_le_bytes(),
            "HeirFlags: Object | Unit"
        );

        // Only MovementUpdate is set, and it is bit 3 of the first byte, MSB-first.
        assert_eq!(&block[11..14], &[0x10, 0x00, 0x00], "18 create bits");

        // MovementInfo opens by repeating the object's guid as the mover.
        assert_eq!(&block[14..19], &[0x01, 0xA0, 0x04, 0x04, 0x08], "MoverGUID");
        // Flags, FlagsExtra, FlagsExtra2, MoveTime, then the position.
        assert_eq!(&block[19..23], &0u32.to_le_bytes(), "Flags");
        assert_eq!(&block[35..39], &1.0f32.to_le_bytes(), "position x");
        assert_eq!(&block[39..43], &2.0f32.to_le_bytes(), "position y");
        assert_eq!(&block[43..47], &3.0f32.to_le_bytes(), "position z");
        assert_eq!(&block[47..51], &0.5f32.to_le_bytes(), "orientation");

        // Pitch, StepUpStartElevation, RemoveForcesIDs count, MoveIndex, then 6 bits that flush to
        // one byte, then the nine speeds.
        assert_eq!(block[67], 0x00, "movement presence bits");
        assert_eq!(&block[68..72], &2.5f32.to_le_bytes(), "walk speed");
        assert_eq!(
            &block[88..92],
            &7.0f32.to_le_bytes(),
            "flight speed default"
        );
    }

    /// An object's own `OBJECT_FIELD_GUID` must equal the GUID in its block header.
    ///
    /// Vanilla stores a GUID in two slots, modern in four. Copying only the two vanilla supplies
    /// leaves the upper 64 bits zero, so the field decodes as high-type `Null` while the header
    /// says `Player` — the object claims to be two different things and the client crashes on it.
    ///
    /// Uses a values block deliberately: it has no movement block and gets no placeholder fields,
    /// so the four GUID slots are the only values present and their offsets are unambiguous.
    #[test]
    fn object_guid_field_is_widened_to_match_the_header() {
        use crate::protocol::update_fields::OBJECT_FIELD_GUID;

        let guid = ObjectGuid::new_player(4);
        let msg = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(guid, ObjectType::Player)
                .set_guid_field(OBJECT_FIELD_GUID, guid),
        ));
        let packet = msg.to_modern().expect("ported");

        let (expected_high, expected_low) = guid.to_guid128(DEFAULT_REALM_ID);
        assert_ne!(
            expected_high, 0,
            "a player guid128 has a non-zero high half"
        );

        // Values block: type byte, packed guid128, mask, then the set values.
        let block = &packet.contents()[11..];
        let blocks = ModernObjectType::Player.field_count().div_ceil(32) as usize;
        assert_eq!(block[0], 0, "UpdateTypeModern::Values");
        assert_eq!(block[6], blocks as u8);

        let values = &block[7 + blocks * 4..];
        assert_eq!(
            &values[0..4],
            &(expected_low as u32).to_le_bytes(),
            "guid low word"
        );
        assert_eq!(
            &values[4..8],
            &((expected_low >> 32) as u32).to_le_bytes(),
            "guid low upper word"
        );
        assert_eq!(
            &values[8..12],
            &(expected_high as u32).to_le_bytes(),
            "guid high word — zero here is the crash"
        );
        assert_eq!(
            &values[12..16],
            &((expected_high >> 32) as u32).to_le_bytes(),
            "guid high upper word"
        );
    }

    /// An ActivePlayer create ends with three more bits before the field masks begin.
    ///
    /// Omitting them does not just lose three bits: the client reads them out of the first byte of
    /// the field mask -- the block count -- and every field offset after that is wrong. The
    /// recipient's own object becomes unparseable while every other object in the packet is fine.
    ///
    /// The mask's block count is the alignment witness: for ActivePlayer it must be 147
    /// (4682 slots), and it lands one byte later than for a plain Player.
    #[test]
    fn an_active_player_create_ends_with_its_own_bit_block() {
        let guid = ObjectGuid::new_player(4);
        let build = |is_self: bool| {
            let msg = SmsgUpdateObject::new().add_block(UpdateBlockData::CreateObject2(
                CreateObjectBlock::new(guid, ObjectTypeId::Player, ObjectType::Player)
                    .with_movement(Position::new(1.0, 2.0, 3.0, 0.0), 0, None),
            ));
            let recipient = Recipient {
                guid: if is_self {
                    guid
                } else {
                    ObjectGuid::new_player(99)
                },
                map_id: 0,
                realm_id: DEFAULT_REALM_ID,
            };
            msg.to_modern_for(recipient)
                .expect("ported")
                .contents()
                .to_vec()
        };

        let mine = build(true);
        let theirs = build(false);

        // Everything up to the movement block is the same length for both, so the ActivePlayer
        // tail is the only difference in where the mask starts.
        let active_blocks = ModernObjectType::ActivePlayer.field_count().div_ceil(32) as u8;
        let player_blocks = ModernObjectType::Player.field_count().div_ceil(32) as u8;
        assert_eq!(active_blocks, 147);
        assert_eq!(player_blocks, 24);

        // Find each body's mask by walking back from the end: values, then the dynamic mask.
        // Simpler and sufficient: the counts must appear, and the ActivePlayer body must be one
        // byte longer in its pre-mask section than the Player one.
        assert!(
            mine.contains(&active_blocks),
            "the ActivePlayer mask block count must be present"
        );
        assert!(
            theirs.contains(&player_blocks),
            "the Player mask block count must be present"
        );

        let mine_pos = mine.iter().position(|&b| b == active_blocks).unwrap();
        let theirs_pos = theirs.iter().position(|&b| b == player_blocks).unwrap();
        assert_eq!(
            mine_pos,
            theirs_pos + 1,
            "the ActivePlayer tail must push the field mask exactly one byte later"
        );
    }

    /// An object created without position data must get neither a movement block nor a stationary
    /// position.
    ///
    /// Items are created this way, and they go out alongside the player at login. Writing a
    /// position for them adds 16 bytes the client never reads, desynchronising every block after
    /// it in the same packet.
    #[test]
    fn an_item_create_carries_no_position_block() {
        let item = ObjectGuid::from_raw(0x4000_0000_0000_0001);
        let msg = SmsgUpdateObject::new().add_block(UpdateBlockData::CreateObject2(
            CreateObjectBlock::new(item, ObjectTypeId::Item, ObjectType::Item),
        ));
        let packet = msg.to_modern().expect("ported");
        let block = &packet.contents()[11..];

        // type byte, packed guid128, object type, heir flags, then the 18 create bits.
        let guid_len = 2 + 1 + 2;
        let bits_at = 1 + guid_len + 1 + 4;
        assert_eq!(
            &block[bits_at..bits_at + 3],
            &[0x00, 0x00, 0x00],
            "no create bits set: neither MovementUpdate nor Stationary"
        );

        // PauseTimesCount follows immediately; a stray position would push it 16 bytes later.
        assert_eq!(&block[bits_at + 3..bits_at + 7], &0i32.to_le_bytes());
    }

    /// A unit *with* movement still gets its movement block, so the guard above cannot silently
    /// strip position from creatures and players.
    #[test]
    fn a_unit_with_movement_still_gets_its_movement_block() {
        let guid = ObjectGuid::new_without_entry(crate::protocol::HighGuid::Unit, 9);
        let msg = SmsgUpdateObject::new().add_block(UpdateBlockData::CreateObject(
            CreateObjectBlock::new(guid, ObjectTypeId::Unit, ObjectType::Unit).with_movement(
                Position::new(1.0, 2.0, 3.0, 0.0),
                0,
                None,
            ),
        ));
        let packet = msg.to_modern().expect("ported");
        let block = &packet.contents()[11..];

        let bits_at = 1 + 5 + 1 + 4;
        assert_eq!(block[bits_at], 0x10, "MovementUpdate is bit 3");
    }

    /// Vanilla and modern movement-flag words agree only up to WalkMode. Vanilla Root is 0x1000,
    /// which 1.14 reads as FallingFar.
    #[test]
    fn modern_movement_flags_translate_by_name() {
        use crate::protocol::updates::modern::block::to_modern_movement_flags;

        assert_eq!(
            to_modern_movement_flags(0x0000_0001),
            0x0000_0001,
            "Forward"
        );
        assert_eq!(to_modern_movement_flags(0x0000_1000), 0x0000_0400, "Root");
        assert_eq!(
            to_modern_movement_flags(0x0020_0000),
            0x0010_0000,
            "Swimming"
        );
        // Levitating, FixedZ, OnTransport and SplineEnabled have no 1.14 member of the same name.
        assert_eq!(to_modern_movement_flags(0x0000_0400), 0, "Levitating");
        assert_eq!(to_modern_movement_flags(0x0200_0000), 0, "OnTransport");
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

        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_UPDATE_OBJECT);
        assert_eq!(msg.blocks.len(), 2);
    }
}
