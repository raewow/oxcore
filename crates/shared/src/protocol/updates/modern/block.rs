//! One object's update block inside a modern `SMSG_UPDATE_OBJECT`.
//!
//! Transcribed from HermesProxy `ObjectUpdateBuilder.WriteToPacket` /`BuildMovementUpdate` and
//! `MovementInfo.WriteMovementInfoModern` for build 40688.

use super::fields::{ModernFieldsArray, ModernObjectType};
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::guid::ObjectGuid;
use crate::protocol::position::Position;
use crate::protocol::updates::movement_block::MovementSpeeds;

/// Flight speed defaults, from HermesProxy's `ObjectUpdate.InitializePlaceholders`. Vanilla has no
/// flight speeds at all, so these are the only source.
const FLIGHT_SPEED: f32 = 7.0;
const FLIGHT_BACK_SPEED: f32 = 4.5;

/// Translate a vanilla movement-flag word to its 1.14 equivalent.
///
/// The two words agree only up to `WalkMode` (0x100); above that they diverge badly. Vanilla
/// `Root` is 0x1000, which in 1.14 means `FallingFar` -- passing the word through unchanged makes
/// every rooted creature appear to be plummeting.
///
/// Matches HermesProxy's `CastFlags<T>`, which translates by enum member *name*: a vanilla flag
/// with no identically-named modern member is dropped rather than remapped by value. That drops
/// `Levitating`, `FixedZ`, `OnTransport` and `SplineEnabled`; the last two are carried by dedicated
/// bits in the movement block instead, and `FixedZ` re-emerges as the hover animation bit.
pub fn to_modern_movement_flags(vanilla: u32) -> u32 {
    MOVEMENT_FLAGS
        .iter()
        .filter(|(from, _)| vanilla & from != 0)
        .fold(0, |acc, (_, to)| acc | to)
}

/// Translate a 1.14 movement-flag word back to vanilla, for packets arriving from a modern client.
///
/// The exact inverse of [`to_modern_movement_flags`], sharing one table so the two cannot drift.
/// Modern-only flags (the `Pending*` family, `DisableGravity`, `DisableCollision`) have no vanilla
/// bit and are dropped.
pub fn from_modern_movement_flags(modern: u32) -> u32 {
    MOVEMENT_FLAGS
        .iter()
        .filter(|(_, to)| modern & to != 0)
        .fold(0, |acc, (from, _)| acc | from)
}

/// (vanilla bit, modern bit) for every movement flag whose name survived into 1.14.
///
/// Read in either direction. `None` is deliberately absent: it is zero on both sides, so including
/// it would make the reverse translation match every input.
const MOVEMENT_FLAGS: [(u32, u32); 19] = [
    (0x0000_0001, 0x0000_0001), // Forward
    (0x0000_0002, 0x0000_0002), // Backward
    (0x0000_0004, 0x0000_0004), // StrafeLeft
    (0x0000_0008, 0x0000_0008), // StrafeRight
    (0x0000_0010, 0x0000_0010), // TurnLeft
    (0x0000_0020, 0x0000_0020), // TurnRight
    (0x0000_0040, 0x0000_0040), // PitchUp
    (0x0000_0080, 0x0000_0080), // PitchDown
    (0x0000_0100, 0x0000_0100), // WalkMode
    (0x0000_1000, 0x0000_0400), // Root
    (0x0000_2000, 0x0000_0800), // Falling
    (0x0000_4000, 0x0000_1000), // FallingFar
    (0x0020_0000, 0x0010_0000), // Swimming
    (0x0080_0000, 0x0080_0000), // CanFly
    (0x0100_0000, 0x0100_0000), // Flying
    (0x0400_0000, 0x0200_0000), // SplineElevation
    (0x1000_0000, 0x0400_0000), // Waterwalking
    (0x2000_0000, 0x0800_0000), // CanSafeFall
    (0x4000_0000, 0x1000_0000), // Hover
];

/// Vanilla `MOVEFLAG_FIXED_Z`, which 1.14 expresses as the `PlayHoverAnim` create bit rather than
/// a movement flag.
const VANILLA_FIXED_Z: u32 = 0x0000_0800;

/// 1.14 update types. Vanilla's `Movement`, `OutOfRange` and `NearObjects` have no equivalent:
/// movement folds into a create block, and the other two moved into the packet header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModernUpdateType {
    Values = 0,
    CreateObject1 = 1,
    CreateObject2 = 2,
}

/// Everything a create block carries beyond field values.
#[derive(Debug, Clone, Default)]
pub struct ModernCreateData {
    pub position: Position,
    pub movement_flags: u32,
    pub speeds: Option<MovementSpeeds>,
    /// Whether the source block actually carried position or movement data.
    ///
    /// Not the same as "has a position field": an item create has none at all. 1.14 gates both the
    /// movement block and the stationary position on this, so writing a position for an object
    /// that never had one adds 16 bytes the client does not read and desynchronises every block
    /// after it.
    pub has_position_data: bool,
    /// Set for the recipient's own object, so the client knows which one to attach the camera to.
    pub this_is_you: bool,
    /// Present while the object is auto-attacking, so the client can draw the swing animation.
    pub combat_victim: Option<ObjectGuid>,
}

/// One object's block: a header, optionally a movement update, then its fields.
#[derive(Debug, Clone)]
pub struct ModernUpdateBlock {
    pub update_type: ModernUpdateType,
    pub guid: ObjectGuid,
    pub object_type: ModernObjectType,
    pub create: Option<ModernCreateData>,
    pub fields: ModernFieldsArray,
}

impl ModernUpdateBlock {
    pub fn new(
        update_type: ModernUpdateType,
        guid: ObjectGuid,
        object_type: ModernObjectType,
        realm_id: u16,
    ) -> Self {
        let mut fields = ModernFieldsArray::new(object_type, realm_id);
        if update_type != ModernUpdateType::Values {
            // 1.14 needs a set of scales and divisors present on every newly created object;
            // vanilla has no source for them and zero is fatal. See `placeholders`.
            //
            // Applied first so that anything the caller sets afterwards wins.
            super::placeholders::apply(&mut fields, object_type, guid, realm_id);
        }

        Self {
            update_type,
            guid,
            object_type,
            create: None,
            fields,
        }
    }

    pub fn write_to(&self, writer: &mut BitWriter, realm_id: u16) {
        writer.write_u8(self.update_type as u8);
        let (high, low) = self.guid.to_guid128(realm_id);
        writer.write_packed_guid_128(high, low);

        if self.update_type != ModernUpdateType::Values {
            writer.write_u8(self.object_type as u8);
            writer.write_i32(self.object_type.type_mask()); // HeirFlags
            self.write_movement_update(writer, realm_id);
        }

        self.fields.write_to(writer);
        self.fields.write_empty_dynamic_to(writer);
    }

    /// The create header: 18 presence bits, then whichever blocks they selected.
    fn write_movement_update(&self, writer: &mut BitWriter, realm_id: u16) {
        let create = self.create.clone().unwrap_or_default();

        // Units get a full movement block, other positioned objects a bare position -- but an
        // object with no position data at all (an item, say) gets neither. Matches the reference's
        // `MoveInfo != null` guard on both bits.
        let has_movement = create.has_position_data && self.object_type.is_unit();
        let stationary = create.has_position_data && !self.object_type.is_unit();

        writer.write_bit(false); // NoBirthAnim
        writer.write_bit(false); // EnablePortals
        writer.write_bit(create.movement_flags & VANILLA_FIXED_Z != 0); // PlayHoverAnim
        writer.write_bit(has_movement); // MovementUpdate
        writer.write_bit(false); // MovementTransport
        writer.write_bit(stationary); // Stationary
        writer.write_bit(create.combat_victim.is_some()); // CombatVictim
        writer.write_bit(false); // ServerTime
        writer.write_bit(false); // Vehicle
        writer.write_bit(false); // AnimKit
        writer.write_bit(false); // Rotation
        writer.write_bit(false); // AreaTrigger
        writer.write_bit(false); // GameObject
        writer.write_bit(false); // SmoothPhasing
        writer.write_bit(create.this_is_you); // ThisIsYou
        writer.write_bit(false); // SceneObject
        writer.write_bit(create.this_is_you); // ActivePlayer
        writer.write_bit(false); // Conversation
        writer.flush_bits();

        if has_movement {
            self.write_movement_info(writer, &create, realm_id);

            let speeds = create.speeds.unwrap_or_default();
            writer.write_f32(speeds.walk);
            writer.write_f32(speeds.run);
            writer.write_f32(speeds.run_back);
            writer.write_f32(speeds.swim);
            writer.write_f32(speeds.swim_back);
            // Vanilla has no flight speeds -- flying mounts postdate 1.12 -- but the client still
            // reads the slots, so send the defaults HermesProxy substitutes.
            writer.write_f32(FLIGHT_SPEED);
            writer.write_f32(FLIGHT_BACK_SPEED);
            writer.write_f32(speeds.turn_rate);
            // 1.14 added a pitch rate; likewise absent from vanilla, and the client is content
            // treating it as the turn rate.
            writer.write_f32(speeds.turn_rate);

            writer.write_u32(0); // MovementForces count
            writer.write_f32(1.0); // MovementForcesModMagnitude

            writer.write_bit(false); // HasMovementSpline
            writer.flush_bits();
        }

        writer.write_i32(0); // PauseTimesCount

        if stationary {
            writer.write_f32(create.position.x);
            writer.write_f32(create.position.y);
            writer.write_f32(create.position.z);
            writer.write_f32(create.position.o);
        }

        if let Some(victim) = create.combat_victim {
            let (high, low) = victim.to_guid128(realm_id);
            writer.write_packed_guid_128(high, low);
        }

        // The ActivePlayer tail. Mandatory whenever the ActivePlayer create bit is set, and it
        // closes the movement update -- the field masks start immediately after.
        //
        // Omitting it does not simply lose three bits: the client reads them out of the first byte
        // of the field mask, which is the block count, and every field offset after that is wrong.
        // That makes the recipient's own object unparseable while every other object in the same
        // packet is fine.
        if create.this_is_you {
            writer.write_bit(false); // HasSceneInstanceIDs
            writer.write_bit(false); // HasRuneState -- death knights, so never in Classic Era
                                     // Action buttons live here in 1.14, not in their own packet, which is why the
                                     // reference has no SMSG_UPDATE_ACTION_BUTTONS. Sending none is valid; wiring the
                                     // player's real bar through means writing 132 i32s in this branch.
            writer.write_bit(false); // HasActionButtons
            writer.flush_bits();
        }
    }

    /// `MovementInfo.WriteMovementInfoModern`.
    ///
    /// From 1.14.1 on, the movement flags are three plain u32s ahead of the timestamp. Earlier
    /// builds bit-packed them (30 and 18 bits) after the position instead, so this layout is
    /// specific to our target build and would corrupt the stream on an older client.
    fn write_movement_info(
        &self,
        writer: &mut BitWriter,
        create: &ModernCreateData,
        realm_id: u16,
    ) {
        let (high, low) = self.guid.to_guid128(realm_id);
        writer.write_packed_guid_128(high, low); // MoverGUID

        // The flag word must agree with the presence bits below. We write `HasFall = false` and no
        // transport or spline blocks, so any flag claiming that data is present has to come out --
        // a client told it is falling with no fall block reads the next field as a jump velocity.
        const MODERN_FALLING: u32 = 0x0000_0800;
        const MODERN_FALLING_FAR: u32 = 0x0000_1000;
        const CONTRADICTS_OMITTED_BLOCKS: u32 = MODERN_FALLING | MODERN_FALLING_FAR;

        let flags = to_modern_movement_flags(create.movement_flags) & !CONTRADICTS_OMITTED_BLOCKS;
        writer.write_u32(flags);
        writer.write_u32(0); // FlagsExtra
        writer.write_u32(0); // FlagsExtra2

        writer.write_u32(0); // MoveTime
        writer.write_f32(create.position.x);
        writer.write_f32(create.position.y);
        writer.write_f32(create.position.z);
        writer.write_f32(create.position.o);

        writer.write_f32(0.0); // Pitch
        writer.write_f32(0.0); // StepUpStartElevation

        writer.write_u32(0); // RemoveForcesIDs count
        writer.write_u32(0); // MoveIndex

        writer.write_bit(false); // HasTransport
        writer.write_bit(false); // HasFall
        writer.write_bit(false); // HasSpline
        writer.write_bit(false); // HeightChangeFailed
        writer.write_bit(false); // RemoteTimeValid
        writer.write_bit(false); // HasInertia, added in 1.14.1
        writer.flush_bits();
    }
}
