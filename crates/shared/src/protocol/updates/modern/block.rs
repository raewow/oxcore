//! One object's update block inside a modern `SMSG_UPDATE_OBJECT` for build 40688.

use super::fields::{ModernFieldsArray, ModernObjectType};
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::guid::ObjectGuid;
use crate::protocol::position::Position;
use crate::protocol::updates::movement_block::MovementSpeeds;

/// Flight speed defaults. Vanilla has no
/// flight speeds at all, so these are the only source.
const FLIGHT_SPEED: f32 = 7.0;
const FLIGHT_BACK_SPEED: f32 = 4.5;

/// Translate a vanilla movement-flag word to its 1.14 equivalent.
///
/// The two words agree only up to `WalkMode` (0x100); above that they diverge badly. Vanilla
/// `Root` is 0x1000, which in 1.14 means `FallingFar` -- passing the word through unchanged makes
/// every rooted creature appear to be plummeting.
///
/// Translates by enum member *name*: a vanilla flag
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
    /// Gameobject quaternion, carried both in public fields and the packed create payload.
    pub rotation: Option<[f32; 4]>,
    /// The recipient's action bar, as vanilla-packed `action | type << 24` words.
    ///
    /// 1.14 has no `SMSG_ACTION_BUTTONS`: the bar rides in the `ActivePlayer` create tail instead.
    /// `None` sets `HasActionButtons = false` and the client keeps whatever its local cache holds.
    pub action_buttons: Option<Vec<u32>>,
}

/// Action-bar slots 1.14 expects in the create tail, per the 1.14 wire format (`MaxActionButtons`).
///
/// Vanilla sends 120. The reader leaves the packed values untouched and zero-pads
/// to 132, so the extra slots are the
/// only difference between the two.
pub const MODERN_ACTION_BUTTON_COUNT: usize = 132;

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
        // object with no position data at all (an item, say) gets neither.
        let has_movement = create.has_position_data && self.object_type.is_unit();
        let stationary = create.has_position_data && !self.object_type.is_unit();
        let has_rotation =
            self.object_type == ModernObjectType::GameObject && create.rotation.is_some();

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
        writer.write_bit(has_rotation); // Rotation
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
            // reads the slots, so send the defaults.
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

        if let Some(rotation) = create.rotation.filter(|_| has_rotation) {
            writer.write_i64(pack_gameobject_rotation(rotation));
        }

        // The ActivePlayer tail. Mandatory whenever the ActivePlayer create bit is set, and it
        // closes the movement update -- the field masks start immediately after.
        //
        // Omitting it does not simply lose three bits: the client reads them out of the first byte
        // of the field mask, which is the block count, and every field offset after that is wrong.
        // That makes the recipient's own object unparseable while every other object in the same
        // packet is fine.
        //
        // per the 1.14 wire format.
        if create.this_is_you {
            let action_buttons = create.action_buttons.as_deref();

            writer.write_bit(false); // HasSceneInstanceIDs
            writer.write_bit(false); // HasRuneState -- death knights, so never in Classic Era
            writer.write_bit(action_buttons.is_some()); // HasActionButtons
            writer.flush_bits();

            // Action buttons live here in 1.14 rather than in their own packet. The count is fixed at 132 and
            // the packed word is passed through unchanged, so a short bar is zero-padded rather
            // than truncating the field.
            if let Some(buttons) = action_buttons {
                for index in 0..MODERN_ACTION_BUTTON_COUNT {
                    writer.write_i32(buttons.get(index).copied().unwrap_or(0) as i32);
                }
            }
        }
    }

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
        // This 1.14 default is substituted when the legacy movement state has no extra
        // flags. A zero value reaches the client only through the unported legacy path and has
        // proven unsafe for freshly visible creatures.
        writer.write_u32(512); // FlagsExtra
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

/// Pack a gameobject rotation quaternion.
fn pack_gameobject_rotation(rotation: [f32; 4]) -> i64 {
    const PACK_YZ: i64 = 1 << 20;
    const PACK_X: i64 = PACK_YZ << 1;
    const PACK_YZ_MASK: i64 = (PACK_YZ << 1) - 1;
    const PACK_X_MASK: i64 = (PACK_X << 1) - 1;

    let sign = if rotation[3] >= 0.0 { 1 } else { -1 };
    let x = ((rotation[0] * PACK_X as f32) as i64 * sign) & PACK_X_MASK;
    let y = ((rotation[1] * PACK_YZ as f32) as i64 * sign) & PACK_YZ_MASK;
    let z = ((rotation[2] * PACK_YZ as f32) as i64 * sign) & PACK_YZ_MASK;
    z | (y << 21) | (x << 42)
}

#[cfg(test)]
mod action_button_tests {
    use super::*;
    use crate::protocol::updates::update_types::ObjectTypeId;

    fn active_player_create(action_buttons: Option<Vec<u32>>) -> Vec<u8> {
        let guid = ObjectGuid::new_player(4);
        let mut block = ModernUpdateBlock::new(
            ModernUpdateType::CreateObject2,
            guid,
            ModernObjectType::from_vanilla(ObjectTypeId::Player, true),
            1,
        );
        block.create = Some(ModernCreateData {
            has_position_data: true,
            this_is_you: true,
            action_buttons,
            ..Default::default()
        });

        let mut writer = BitWriter::new();
        block.write_to(&mut writer, 1);
        writer.into_bytes()
    }

    /// 1.14 has no action-buttons packet; the bar is 132 × i32 in the create tail. The count is
    /// fixed, so a 120-slot vanilla bar must be zero-padded rather than truncating the field —
    /// getting the length wrong shifts every field mask that follows.
    #[test]
    fn a_short_bar_is_padded_to_the_fixed_count() {
        let with_bar = active_player_create(Some(vec![0x0100_002A; 120]));
        let without = active_player_create(None);

        assert_eq!(
            with_bar.len() - without.len(),
            MODERN_ACTION_BUTTON_COUNT * 4,
            "the tail must always carry {MODERN_ACTION_BUTTON_COUNT} buttons"
        );
    }

    /// The packed `action | type << 24` word is identical in both protocols, so it passes through
    /// untouched. A transform here would silently rewrite everyone's bars.
    ///
    /// The button block sits between the movement update and the field masks, not at the end of the
    /// packet, so it is located by diffing against the no-bar encoding rather than from the tail.
    #[test]
    fn packed_words_pass_through_unchanged() {
        let packed = 0x0100_002Au32; // action 42, type 1
        let with_bar = active_player_create(Some(vec![packed]));
        let without = active_player_create(None);

        // The two encodings agree up to the byte holding the presence bits; the buttons start on the
        // next byte boundary after it.
        let diverge = with_bar
            .iter()
            .zip(&without)
            .position(|(a, b)| a != b)
            .expect("the HasActionButtons bit must change the encoding");
        let buttons = &with_bar[diverge + 1..diverge + 1 + MODERN_ACTION_BUTTON_COUNT * 4];

        assert_eq!(&buttons[..4], &packed.to_le_bytes(), "first slot verbatim");
        assert_eq!(&buttons[4..8], &0u32.to_le_bytes(), "second slot padded");
    }

    /// Without a bar the presence bit must be clear and nothing written, or the client reads 528
    /// bytes of field mask as buttons.
    #[test]
    fn no_bar_writes_no_buttons() {
        let without = active_player_create(None);
        let empty_bar = active_player_create(Some(Vec::new()));
        assert_eq!(
            empty_bar.len() - without.len(),
            MODERN_ACTION_BUTTON_COUNT * 4,
            "an empty-but-present bar still writes the full block"
        );
    }
}
