//! Movement-related message structures

use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::guid::ObjectGuid as WorldObjectGuid;
use crate::protocol::packet::WorldPacketGuidExt;
use crate::protocol::{ObjectGuid, Opcode, Position, WorldPacket};

/// SMSG_PONG - response to CMSG_PING
pub struct SmsgPong {
    pub sequence: u32,
}

impl ToWorldPacket for SmsgPong {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_PONG);
        packet.write_u32(self.sequence);
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut packet = WorldPacket::new(Opcode::SMSG_PONG);
        packet.write_u32(self.sequence);
        Some(packet)
    }
}

/// SMSG_FORCE_MOVE_ROOT - Lock player in place (prevent movement)
#[derive(Debug, Clone)]
pub struct SmsgForceMoveRoot {
    pub guid: WorldObjectGuid,
}

impl ToWorldPacket for SmsgForceMoveRoot {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_FORCE_MOVE_ROOT);
        packet.write_packed_guid(self.guid);
        packet.write_u32(0); // counter
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        Some(move_set_flag(Opcode::SMSG_FORCE_MOVE_ROOT, self.guid))
    }
}

/// `MoveSetFlag` (HermesProxy `MovementPackets.cs:390`): a packed 128-bit mover and a counter.
///
/// The whole family of root/unroot/water-walk/hover messages shares this body; only the opcode
/// differs.
fn move_set_flag(opcode: Opcode, guid: WorldObjectGuid) -> WorldPacket {
    let mut writer = BitWriter::new();
    let (high, low) = guid.to_guid128(MODERN_REALM_ID);
    writer.write_packed_guid_128(high, low);
    writer.write_u32(0); // MoveCounter
    writer.finish(opcode)
}

/// SMSG_FORCE_MOVE_UNROOT - Unlock player (allow movement)
#[derive(Debug, Clone)]
pub struct SmsgForceMoveUnroot {
    pub guid: WorldObjectGuid,
}

impl ToWorldPacket for SmsgForceMoveUnroot {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_FORCE_MOVE_UNROOT);
        packet.write_packed_guid(self.guid);
        packet.write_u32(0); // counter
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        Some(move_set_flag(Opcode::SMSG_FORCE_MOVE_UNROOT, self.guid))
    }
}

/// Spline flags for movement packets
pub mod spline_flags {
    pub const DONE: u32 = 0x00000001;
    pub const FALLING: u32 = 0x00000002;
    pub const FLYING: u32 = 0x00000200;
    pub const NO_SPLINE: u32 = 0x00000400;
    pub const WALKMODE: u32 = 0x00000100;
    pub const RUNMODE: u32 = 0x00000000;
    pub const CATMULLROM: u32 = 0x00100000;
}

/// SMSG_MONSTER_MOVE (0x00DD)
#[derive(Debug, Clone)]
pub struct SmsgMonsterMove {
    pub guid: ObjectGuid,
    pub position: Position,
    pub spline_id: u32,
    pub move_type: u8, // 0 = normal, 1 = stop, 2 = facing spot, 3 = facing target, 4 = facing angle
    pub facing_target: Option<ObjectGuid>, // For move_type 3
    pub facing_angle: Option<f32>, // For move_type 4
    pub spline_flags: u32,
    pub duration: u32,
    pub waypoints: Vec<Position>,
}

impl SmsgMonsterMove {
    /// Create a simple point-to-point move
    ///
    /// Used by ChaseMovementGenerator and HomeMovementGenerator
    pub fn new_point_move(
        guid: ObjectGuid,
        from: Position,
        to: Position,
        speed: f32,
        is_walking: bool,
    ) -> Self {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let dz = to.z - from.z;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();
        let duration = ((distance / speed) * 1000.0) as u32;

        let spline_flags = if is_walking {
            super::movement::spline_flags::WALKMODE
        } else {
            super::movement::spline_flags::RUNMODE
        };

        Self {
            guid,
            position: from,
            spline_id: rand::random(),
            move_type: 0, // Normal
            facing_target: None,
            facing_angle: None,
            spline_flags,
            duration,
            waypoints: vec![to],
        }
    }

    /// Create a multi-waypoint path move (linear with packed intermediate waypoints)
    ///
    /// `path` should contain intermediate + destination waypoints (NOT the start position).
    /// The start position is `from`.
    pub fn new_path_move(
        guid: ObjectGuid,
        from: Position,
        path: Vec<Position>,
        duration: u32,
        is_walking: bool,
    ) -> Self {
        let spline_flags = if is_walking {
            super::movement::spline_flags::WALKMODE
        } else {
            super::movement::spline_flags::RUNMODE
        };

        Self {
            guid,
            position: from,
            spline_id: rand::random(),
            move_type: 0, // Normal
            facing_target: None,
            facing_angle: None,
            spline_flags,
            duration,
            waypoints: path,
        }
    }

    /// Create a chase move (faces the target GUID while moving)
    pub fn new_chase_move(
        guid: ObjectGuid,
        from: Position,
        to: Position,
        speed: f32,
        target: ObjectGuid,
    ) -> Self {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let dz = to.z - from.z;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();
        let duration = ((distance / speed) * 1000.0) as u32;

        Self {
            guid,
            position: from,
            spline_id: rand::random(),
            move_type: 3, // FacingTarget
            facing_target: Some(target),
            facing_angle: None,
            spline_flags: super::movement::spline_flags::RUNMODE,
            duration,
            waypoints: vec![to],
        }
    }

    /// Create a multi-waypoint chase move (faces the target GUID while following path)
    pub fn new_chase_path_move(
        guid: ObjectGuid,
        from: Position,
        path: Vec<Position>,
        duration: u32,
        target: ObjectGuid,
    ) -> Self {
        Self {
            guid,
            position: from,
            spline_id: rand::random(),
            move_type: 3, // FacingTarget
            facing_target: Some(target),
            facing_angle: None,
            spline_flags: super::movement::spline_flags::RUNMODE,
            duration,
            waypoints: path,
        }
    }

    /// Create a facing-only packet (no movement, just rotate)
    /// MaNGOS includes the creature's current position as a waypoint even for facing-only moves.
    /// The 1.12.1 client requires at least one waypoint for non-stop move types.
    pub fn new_facing_angle(guid: ObjectGuid, position: Position, angle: f32) -> Self {
        Self {
            guid,
            position,
            spline_id: rand::random(),
            move_type: 4, // FacingAngle
            facing_target: None,
            facing_angle: Some(angle),
            spline_flags: super::movement::spline_flags::DONE,
            duration: 0,
            waypoints: vec![position], // Must include at least one waypoint
        }
    }

    /// Create a stop movement packet
    /// vmangos: MoveSplineInit::Launch with SetStop() sends position + new splineId + move_type=1
    pub fn new_stop(guid: ObjectGuid, position: Position) -> Self {
        Self {
            guid,
            position,
            spline_id: rand::random(), // vmangos uses splineCounter++ (a new unique ID)
            move_type: 1,              // Stop
            facing_target: None,
            facing_angle: None,
            spline_flags: super::movement::spline_flags::DONE,
            duration: 0,
            waypoints: vec![],
        }
    }
}

impl ToWorldPacket for SmsgMonsterMove {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_MONSTER_MOVE);

        packet.write_packed_guid(self.guid);
        packet.write_f32(self.position.x);
        packet.write_f32(self.position.y);
        packet.write_f32(self.position.z);
        packet.write_u32(self.spline_id);
        packet.write_u8(self.move_type);

        // Stop (move_type=1): MaNGOS returns immediately after move_type byte
        if self.move_type == 1 {
            return packet;
        }

        // Write facing data based on move_type (between move_type and spline_flags)
        match self.move_type {
            3 => {
                // FacingTarget: write target GUID as u64
                if let Some(target) = self.facing_target {
                    packet.write_u64(target.raw());
                }
            }
            4 => {
                // FacingAngle: write angle as f32
                if let Some(angle) = self.facing_angle {
                    packet.write_f32(angle);
                }
            }
            _ => {} // Normal (0) has no extra facing data
        }

        packet.write_u32(self.spline_flags);
        packet.write_u32(self.duration);

        // MaNGOS linear path format (packet_builder.cpp:WriteLinearPath):
        // - count (number of waypoints excluding start, i.e. segments)
        // - destination (last waypoint) as full xyz
        // - intermediate waypoints as packed offsets from midpoint(start, dest)
        let wp_count = self.waypoints.len() as u32;
        packet.write_u32(wp_count);

        if !self.waypoints.is_empty() {
            let dest = self.waypoints.last().unwrap();
            // Write destination as full xyz
            packet.write_f32(dest.x);
            packet.write_f32(dest.y);
            packet.write_f32(dest.z);

            // Intermediate waypoints as packed delta from midpoint
            if self.waypoints.len() > 1 {
                let middle_x = (self.position.x + dest.x) / 2.0;
                let middle_y = (self.position.y + dest.y) / 2.0;
                let middle_z = (self.position.z + dest.z) / 2.0;

                // Write all intermediate points (skip last which is the destination)
                for wp in &self.waypoints[..self.waypoints.len() - 1] {
                    packet.write_pack_xyz(middle_x - wp.x, middle_y - wp.y, middle_z - wp.z);
                }
            }
        }

        packet
    }

    /// `SMSG_ON_MONSTER_MOVE`, per HermesProxy `MovementPackets.cs:95` and the legacy conversion in
    /// `MovementHandler.cs:383`.
    ///
    /// Same information as vanilla, reordered and bit-packed: the facing data moves *after* the
    /// counts instead of sitting between the move type and the flags, and the waypoint count is a
    /// 16-bit field inside a bit run rather than a u32.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.guid.to_guid128(MODERN_REALM_ID);
        writer.write_packed_guid_128(high, low);

        writer.write_f32(self.position.x);
        writer.write_f32(self.position.y);
        writer.write_f32(self.position.z);
        writer.write_u32(self.spline_id);

        // Destination is sent as the last of `Points`, not here.
        writer.write_f32(0.0);
        writer.write_f32(0.0);
        writer.write_f32(0.0);

        // A stop carries no path, so nothing below produces points.
        let is_stop = self.move_type == SPLINE_TYPE_STOP;
        let (points, deltas) = if is_stop {
            (Vec::new(), Vec::new())
        } else {
            self.modern_path()
        };

        writer.write_bit(false); // CrzTeleport
                                 // 2 means "no tolerance to apply" and is what HermesProxy sends for a pathless move.
        writer.write_bits(if points.is_empty() { 2 } else { 0 }, 3);

        writer.write_u32(if is_stop {
            0
        } else {
            to_modern_spline_flags(self.spline_flags)
        });
        writer.write_i32(0); // Elapsed
        writer.write_u32(if is_stop { 0 } else { self.duration });
        writer.write_u32(0); // FadeObjectTime
        writer.write_u8(0); // SplineMode
        writer.write_packed_guid_128(0, 0); // TransportGUID
        writer.write_u8(0); // TransportSeat

        let spline_type = if is_stop {
            MODERN_SPLINE_TYPE_NONE
        } else {
            modern_spline_type(self.move_type)
        };
        writer.write_bits(spline_type as u32, 2);
        writer.write_bits(points.len() as u32, 16);
        writer.write_bit(false); // VehicleExitVoluntary
        writer.write_bit(false); // Interpolate
        writer.write_bits(deltas.len() as u32, 16);
        writer.write_bit(false); // SplineFilter
        writer.write_bit(false); // SpellEffectExtraData
        writer.write_bit(false); // JumpExtraData
        writer.flush_bits();

        // Facing data trails the counts here, where vanilla puts it right after the move type.
        match spline_type {
            MODERN_SPLINE_TYPE_FACING_TARGET => {
                writer.write_f32(self.facing_angle.unwrap_or(0.0));
                let (high, low) = self
                    .facing_target
                    .unwrap_or_default()
                    .to_guid128(MODERN_REALM_ID);
                writer.write_packed_guid_128(high, low);
            }
            MODERN_SPLINE_TYPE_FACING_ANGLE => {
                writer.write_f32(self.facing_angle.unwrap_or(0.0));
            }
            // FacingSpot needs a target position, which the vanilla body never carried.
            _ => {}
        }

        for point in &points {
            writer.write_f32(point.x);
            writer.write_f32(point.y);
            writer.write_f32(point.z);
        }
        for delta in &deltas {
            writer.write_u32(pack_xyz(delta.0, delta.1, delta.2));
        }

        Some(writer.finish(Opcode::SMSG_MONSTER_MOVE))
    }
}

impl SmsgMonsterMove {
    /// Split the waypoint list the way 1.14 wants it: the destination as a full point, and the
    /// intermediate waypoints as deltas from the midpoint.
    ///
    /// Same decomposition the vanilla body uses, so the two stay consistent.
    fn modern_path(&self) -> (Vec<Position>, Vec<(f32, f32, f32)>) {
        let Some(dest) = self.waypoints.last().copied() else {
            return (Vec::new(), Vec::new());
        };

        let middle = (
            (self.position.x + dest.x) / 2.0,
            (self.position.y + dest.y) / 2.0,
            (self.position.z + dest.z) / 2.0,
        );
        let deltas = self.waypoints[..self.waypoints.len() - 1]
            .iter()
            .map(|wp| (middle.0 - wp.x, middle.1 - wp.y, middle.2 - wp.z))
            .collect();

        (vec![dest], deltas)
    }
}

/// Realm used to qualify GUIDs in modern movement bodies. Must match the object updates'.
const MODERN_REALM_ID: u16 = 1;

/// `SplineTypeLegacy::Stop`.
const SPLINE_TYPE_STOP: u8 = 1;

const MODERN_SPLINE_TYPE_NONE: u8 = 0;
const MODERN_SPLINE_TYPE_FACING_SPOT: u8 = 1;
const MODERN_SPLINE_TYPE_FACING_TARGET: u8 = 2;
const MODERN_SPLINE_TYPE_FACING_ANGLE: u8 = 3;

/// Vanilla spline types are renumbered in 1.14: `Stop` disappears (a stop is just a move with no
/// path) and everything after it shifts down by one.
fn modern_spline_type(vanilla_move_type: u8) -> u8 {
    match vanilla_move_type {
        2 => MODERN_SPLINE_TYPE_FACING_SPOT,
        3 => MODERN_SPLINE_TYPE_FACING_TARGET,
        4 => MODERN_SPLINE_TYPE_FACING_ANGLE,
        _ => MODERN_SPLINE_TYPE_NONE,
    }
}

/// Translate vanilla spline flags to 1.14.
///
/// Like the movement flags, these are translated by name rather than value -- the two enums share
/// almost no bit positions. `Done` moves from `0x01` to `0x20`, `NoSpline` from `0x400` to `0x80`,
/// and so on.
///
/// The `Runmode`-only case is special-cased the way HermesProxy does it: real vanilla servers send
/// exactly that flag for an ordinary creature move, and the modern client wants a specific set of
/// flags rather than a literal translation of it.
///
/// **Note for whoever verifies this against a live client:** our `spline_flags::WALKMODE` is
/// `0x100`, but HermesProxy's vanilla table calls `0x100` `Runmode` and `spline_flags::RUNMODE` is
/// `0`. Those two constants look inverted relative to what a real vanilla server sends. That is a
/// pre-existing vanilla-side question and is deliberately not changed here, since the 1.12 client
/// works today -- but it means a running creature may take the literal branch below and a walking
/// one the default branch.
pub fn to_modern_spline_flags(vanilla: u32) -> u32 {
    /// Vanilla `SplineFlagVanilla::Runmode`, the whole flag word an unmodified vanilla server
    /// sends for a normal move.
    const VANILLA_RUNMODE: u32 = 0x0000_0100;

    /// What HermesProxy substitutes for that default: `Unknown5 | Steering | Unknown10`. Opaque
    /// but load-bearing -- without them the client will not animate the move.
    const MODERN_DEFAULT: u32 = 0x0100_0000 | 0x1000_0000 | 0x8000_0000;

    if vanilla == VANILLA_RUNMODE {
        return MODERN_DEFAULT;
    }

    /// (vanilla bit, modern bit) for every spline flag whose name survived into 1.14.
    const TRANSLATED: [(u32, u32); 8] = [
        (0x0000_0001, 0x0000_0020), // Done
        (0x0000_0002, 0x0000_0040), // Falling
        (0x0000_0200, 0x0000_0200), // Flying
        (0x0000_0400, 0x0000_0080), // NoSpline
        (0x0010_0000, 0x0000_1000), // Cyclic
        (0x0020_0000, 0x0000_2000), // EnterCycle
        (0x0040_0000, 0x0000_4000), // Frozen
        (0x4000_0000, 0x0040_0000), // UncompressedPath
    ];

    TRANSLATED
        .iter()
        .filter(|(from, _)| vanilla & from != 0)
        .fold(0, |acc, (_, to)| acc | to)
}

/// Pack a position delta into one u32 at quarter-yard resolution, as 1.14 expects.
fn pack_xyz(x: f32, y: f32, z: f32) -> u32 {
    ((x / 0.25) as i32 as u32 & 0x7FF)
        | ((y / 0.25) as i32 as u32 & 0x7FF) << 11
        | ((z / 0.25) as i32 as u32 & 0x3FF) << 22
}

#[cfg(test)]
mod modern_tests {
    use super::*;

    fn guid() -> ObjectGuid {
        ObjectGuid::new_without_entry(crate::protocol::HighGuid::Unit, 42)
    }

    /// The bit run in the middle is the fragile part: a 2-bit spline type, two 16-bit counts and
    /// five loose bits, flushed together. Miscounting shifts the facing data and the path.
    #[test]
    fn monster_move_modern_layout() {
        let msg = SmsgMonsterMove::new_point_move(
            guid(),
            Position::new(10.0, 20.0, 30.0, 0.0),
            Position::new(11.0, 21.0, 31.0, 0.0),
            7.0,
            false,
        );
        let packet = msg.to_modern().expect("ported");
        let body = packet.contents();

        assert_eq!(packet.opcode(), Opcode::SMSG_MONSTER_MOVE);

        // MoverGUID, then StartPosition.
        let guid_len = 2 + 1 + 2; // masks, low byte, high bytes
        assert_eq!(&body[guid_len..guid_len + 4], &10.0f32.to_le_bytes());
        assert_eq!(&body[guid_len + 4..guid_len + 8], &20.0f32.to_le_bytes());
        assert_eq!(&body[guid_len + 8..guid_len + 12], &30.0f32.to_le_bytes());

        // SplineId, then a zeroed Destination -- the real one rides in Points.
        let after_spline_id = guid_len + 12 + 4;
        assert_eq!(&body[after_spline_id..after_spline_id + 12], &[0u8; 12]);

        // CrzTeleport plus a 3-bit tolerance of 0 (there is a path), flushed to one byte.
        let bits = after_spline_id + 12;
        assert_eq!(body[bits], 0x00);

        // The destination is the single entry in Points, at the very end.
        let tail = &body[body.len() - 12..];
        assert_eq!(&tail[0..4], &11.0f32.to_le_bytes());
        assert_eq!(&tail[4..8], &21.0f32.to_le_bytes());
        assert_eq!(&tail[8..12], &31.0f32.to_le_bytes());
    }

    /// A stop carries no path, and 1.14 has no `Stop` spline type -- it is a move with zero points
    /// and a tolerance of 2.
    #[test]
    fn monster_move_stop_has_no_path() {
        let msg = SmsgMonsterMove::new_stop(guid(), Position::new(1.0, 2.0, 3.0, 0.0));
        let packet = msg.to_modern().expect("ported");
        let body = packet.contents();

        let bits = 5 + 12 + 4 + 12;
        // CrzTeleport false then 3-bit tolerance 2 = 0b0_010 in the top bits, MSB-first.
        assert_eq!(body[bits], 0b0010_0000);
        assert_eq!(
            modern_spline_type(SPLINE_TYPE_STOP),
            MODERN_SPLINE_TYPE_NONE
        );
    }

    /// Vanilla and modern spline flags share almost no bit positions, so they translate by name.
    #[test]
    fn spline_flags_translate_by_name() {
        assert_eq!(to_modern_spline_flags(0x0000_0001), 0x0000_0020, "Done");
        assert_eq!(to_modern_spline_flags(0x0000_0400), 0x0000_0080, "NoSpline");
        assert_eq!(to_modern_spline_flags(0x0010_0000), 0x0000_1000, "Cyclic");
        assert_eq!(to_modern_spline_flags(0x0000_0200), 0x0000_0200, "Flying");
    }

    /// A bare Runmode is what an unmodified vanilla server sends for an ordinary move, and the
    /// modern client wants a specific replacement set rather than a literal translation.
    #[test]
    fn a_bare_runmode_becomes_the_modern_default_set() {
        assert_eq!(
            to_modern_spline_flags(0x0000_0100),
            0x0100_0000 | 0x1000_0000 | 0x8000_0000
        );
    }

    #[test]
    fn root_and_unroot_share_the_move_set_flag_body() {
        let root = SmsgForceMoveRoot { guid: guid() }
            .to_modern()
            .expect("ported");
        let unroot = SmsgForceMoveUnroot { guid: guid() }
            .to_modern()
            .expect("ported");

        assert_eq!(root.contents(), unroot.contents());
        // packed guid128 then a u32 counter
        assert_eq!(root.contents().len(), 5 + 4);
        assert_eq!(&root.contents()[5..9], &0u32.to_le_bytes());
    }
}
