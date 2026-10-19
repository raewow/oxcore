//! Movement-related message structures

use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::guid::ObjectGuid as WorldObjectGuid;
use crate::protocol::packet::WorldPacketGuidExt;
use crate::protocol::updates::modern::block::normalize_orientation;
use crate::protocol::{MovementInfo, ObjectGuid, Opcode, Position, WorldPacket};

/// SMSG_TRANSFER_PENDING
///
/// First half of the cross-map teleport handshake: tells the client a
/// teleport to `dest_map` is starting so it can show the loading screen.
/// Followed by [`SmsgNewWorld`].
#[derive(Debug, Clone)]
pub struct SmsgTransferPending {
    pub dest_map: u32,
    /// Position on the map the player is leaving. Modern-only field.
    pub old_position: Position,
}

impl ToWorldPacket for SmsgTransferPending {
    /// `TransferPending::Write` for build 42597: adds the pre-teleport
    /// position and two "has ship" / "has transfer spell" bits, both false
    /// for every teleport path this server initiates (no transport-borne or
    /// spell-triggered transfers yet).
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.dest_map);
        writer.write_f32(self.old_position.x);
        writer.write_f32(self.old_position.y);
        writer.write_f32(self.old_position.z);
        writer.write_bit(false); // Ship
        writer.write_bit(false); // TransferSpellID
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_TRANSFER_PENDING))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_TRANSFER_PENDING);
        packet.write_u32(self.dest_map);
        packet
    }
}

/// Reference `Player.h::TransferAbortReason` values for the vanilla client.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum TransferAbortReason {
    MaxPlayers = 1,
    NotFound = 2,
    TooManyInstances = 3,
    Silently = 4,
    ZoneInCombat = 5,
}

/// SMSG_TRANSFER_ABORTED - the requested destination cannot be entered.
#[derive(Debug, Clone)]
pub struct SmsgTransferAborted {
    /// Only modern clients include the destination map in this response.
    pub map_id: u32,
    pub reason: TransferAbortReason,
}

impl ToWorldPacket for SmsgTransferAborted {
    fn to_vanilla(&self) -> WorldPacket {
        // Reference core Misc::TransferAborted sends only the reason byte.
        let mut packet = WorldPacket::new(Opcode::SMSG_TRANSFER_ABORTED);
        packet.write_u8(self.reason as u8);
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        // JimsProxy MovementPackets::TransferAborted: reason values are
        // translated by name, then encoded in a six-bit field.
        let reason = match self.reason {
            TransferAbortReason::MaxPlayers => 2,
            TransferAbortReason::NotFound => 3,
            TransferAbortReason::TooManyInstances => 4,
            TransferAbortReason::Silently => 1, // Error
            TransferAbortReason::ZoneInCombat => 6,
        };
        let mut writer = BitWriter::new();
        writer.write_u32(self.map_id);
        writer.write_u8(0); // Arg
        writer.write_i32(-6); // MapDifficultyXConditionID (proxy default)
        writer.write_bits(reason, 6);
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_TRANSFER_ABORTED))
    }
}

/// SMSG_NEW_WORLD
///
/// Second half of the cross-map teleport handshake: gives the client the
/// destination map/position. The client responds with
/// MSG_MOVE_WORLDPORT_ACK once it has loaded the new map.
#[derive(Debug, Clone)]
pub struct SmsgNewWorld {
    pub dest_map: u32,
    pub dest_pos: Position,
}

impl ToWorldPacket for SmsgNewWorld {
    /// `NewWorld::Write` for build 42597: adds a `Reason` field (4, the
    /// ordinary transfer the 1.14 client loads a map for) and a
    /// `MovementOffset` Vector3 (always zero — nothing in-flight needs
    /// adjusting across the port).
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.dest_map);
        writer.write_f32(self.dest_pos.x);
        writer.write_f32(self.dest_pos.y);
        writer.write_f32(self.dest_pos.z);
        writer.write_f32(self.dest_pos.o);
        writer.write_u32(4); // Reason: ordinary transfer
        writer.write_f32(0.0); // MovementOffset.x
        writer.write_f32(0.0); // MovementOffset.y
        writer.write_f32(0.0); // MovementOffset.z
        Some(writer.finish(Opcode::SMSG_NEW_WORLD))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_NEW_WORLD);
        packet.write_u32(self.dest_map);
        packet.write_f32(self.dest_pos.x);
        packet.write_f32(self.dest_pos.y);
        packet.write_f32(self.dest_pos.z);
        packet.write_f32(self.dest_pos.o);
        packet
    }
}

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

/// `MoveSetFlag`: a packed 128-bit mover and a counter.
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

/// SMSG_CONTROL_UPDATE: whether the client may steer its own character
/// (taken away for a flight on 1.14).
#[derive(Debug, Clone)]
pub struct SmsgControlUpdate {
    pub guid: WorldObjectGuid,
    pub has_control: bool,
}

impl ToWorldPacket for SmsgControlUpdate {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CONTROL_UPDATE);
        packet.write_packed_guid(self.guid);
        packet.write_u8(u8::from(self.has_control));
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.guid.to_guid128(MODERN_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_bit(self.has_control);
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_CONTROL_UPDATE))
    }
}

/// SMSG_MOVE_ENABLE_GRAVITY (1.14 only): gravity back on after a flight.
#[derive(Debug, Clone)]
pub struct SmsgMoveEnableGravity {
    pub guid: WorldObjectGuid,
}

impl ToWorldPacket for SmsgMoveEnableGravity {
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_MOVE_ENABLE_GRAVITY)
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        Some(move_set_flag(Opcode::SMSG_MOVE_ENABLE_GRAVITY, self.guid))
    }
}

/// SMSG_MOVE_UNSET_CAN_FLY (1.14 only): the client stops flying after a flight.
#[derive(Debug, Clone)]
pub struct SmsgMoveUnsetCanFly {
    pub guid: WorldObjectGuid,
}

impl ToWorldPacket for SmsgMoveUnsetCanFly {
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_MOVE_UNSET_CAN_FLY)
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        Some(move_set_flag(Opcode::SMSG_MOVE_UNSET_CAN_FLY, self.guid))
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

/// Spline flags 1.14 wants for a flight path: Flying, CatmullRom, CanSwim,
/// UncompressedPath and the Unknown5/Steering/Unknown10 set that makes the
/// client animate the move.
const MODERN_FLIGHT_SPLINE_FLAGS: u32 =
    0x0000_0200 | 0x0000_0800 | 0x0020_0000 | 0x0040_0000 | 0x0100_0000 | 0x1000_0000 | 0x8000_0000;

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
    /// The creature's current position is included as a waypoint even for facing-only moves.
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
    /// MoveSplineInit::Launch with SetStop() sends position + new splineId + move_type=1
    pub fn new_stop(guid: ObjectGuid, position: Position) -> Self {
        Self {
            guid,
            position,
            spline_id: rand::random(), // new unique ID
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

        // Stop (move_type=1): return immediately after move_type byte
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

        let wp_count = self.waypoints.len() as u32;
        packet.write_u32(wp_count);

        // A flying (smooth) path lists every point in full.
        if self.spline_flags & spline_flags::FLYING != 0 {
            for wp in &self.waypoints {
                packet.write_f32(wp.x);
                packet.write_f32(wp.y);
                packet.write_f32(wp.z);
            }
            return packet;
        }

        // A linear path gives the destination in full, then each intermediate
        // point as a packed offset back from the destination.
        if let Some(dest) = self.waypoints.last() {
            packet.write_f32(dest.x);
            packet.write_f32(dest.y);
            packet.write_f32(dest.z);

            for wp in &self.waypoints[..self.waypoints.len() - 1] {
                let (dx, dy, mut dz) = (dest.x - wp.x, dest.y - wp.y, dest.z - wp.z);
                // The client freezes on an all-zero offset; nudge it.
                if dx.abs() < 0.25 && dy.abs() < 0.25 && dz.abs() < 0.25 {
                    dz += if dz < 0.0 { 0.51 } else { 0.26 };
                }
                packet.write_pack_xyz(dx, dy, dz);
            }
        }

        packet
    }

    /// `SMSG_ON_MONSTER_MOVE`, per the 1.14 wire format.
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
        let flying = self.spline_flags & spline_flags::FLYING != 0;
        let (points, deltas) = if is_stop {
            (Vec::new(), Vec::new())
        } else if flying {
            (self.waypoints.clone(), Vec::new())
        } else {
            self.modern_path()
        };

        writer.write_bit(false); // CrzTeleport
                                 // 2 means "no tolerance to apply" for a pathless move.
        writer.write_bits(if points.is_empty() { 2 } else { 0 }, 3);

        writer.write_u32(if is_stop {
            0
        } else if flying {
            MODERN_FLIGHT_SPLINE_FLAGS
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
                writer.write_f32(normalize_orientation(self.facing_angle.unwrap_or(0.0)));
                let (high, low) = self
                    .facing_target
                    .unwrap_or_default()
                    .to_guid128(MODERN_REALM_ID);
                writer.write_packed_guid_128(high, low);
            }
            MODERN_SPLINE_TYPE_FACING_ANGLE => {
                writer.write_f32(normalize_orientation(self.facing_angle.unwrap_or(0.0)));
            }
            MODERN_SPLINE_TYPE_FACING_SPOT => {
                // The spot is written as a Vector3 here. The vanilla body carries one
                // too, but our struct never records it, so this sends the origin rather than
                // omitting 12 bytes and desynchronising everything after it. No caller produces
                // FacingSpot today; add the field before one does.
                writer.write_f32(0.0);
                writer.write_f32(0.0);
                writer.write_f32(0.0);
            }
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
/// The `Runmode`-only case is special-cased: real vanilla servers send
/// exactly that flag for an ordinary creature move, and the modern client wants a specific set of
/// flags rather than a literal translation of it.
///
/// **Note for whoever verifies this against a live client:** our `spline_flags::WALKMODE` is
/// `0x100`, but the vanilla table calls `0x100` `Runmode` and `spline_flags::RUNMODE` is
/// `0`. Those two constants look inverted relative to what a real vanilla server sends. That is a
/// pre-existing vanilla-side question and is deliberately not changed here, since the 1.12 client
/// works today -- but it means a running creature may take the literal branch below and a walking
/// one the default branch.
pub fn to_modern_spline_flags(vanilla: u32) -> u32 {
    /// Vanilla `SplineFlagVanilla::Runmode`, the whole flag word an unmodified vanilla server
    /// sends for a normal move.
    const VANILLA_RUNMODE: u32 = 0x0000_0100;

    /// The substitute for that default: `Unknown5 | Steering | Unknown10`. Opaque
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

    #[test]
    fn transfer_aborted_matches_reference_bodies_and_reason_values() {
        // core Misc::TransferAborted and JimsProxy MovementPackets::TransferAborted.
        // Modern build 42597 uses JimsProxy's V2_5_3_41750 opcode table.
        for (reason, vanilla, modern_bits) in [
            (TransferAbortReason::MaxPlayers, 1, 0x08),
            (TransferAbortReason::NotFound, 2, 0x0c),
            (TransferAbortReason::TooManyInstances, 3, 0x10),
            (TransferAbortReason::Silently, 4, 0x04),
            (TransferAbortReason::ZoneInCombat, 5, 0x18),
        ] {
            let message = SmsgTransferAborted { map_id: 36, reason };
            let packet = message.to_vanilla();
            assert_eq!(packet.opcode().vanilla(), 0x0040);
            assert_eq!(packet.contents(), &[vanilla]);
            let packet = message.to_modern().unwrap();
            assert_eq!(packet.opcode().modern(), 0x26fa);
            assert_eq!(
                packet.contents(),
                &[36, 0, 0, 0, 0, 0xfa, 0xff, 0xff, 0xff, modern_bits]
            );
        }
    }

    fn guid() -> ObjectGuid {
        ObjectGuid::new_without_entry(crate::protocol::HighGuid::Unit, 42)
    }

    fn path_move(flags: u32) -> SmsgMonsterMove {
        SmsgMonsterMove {
            guid: guid(),
            position: Position::new(0.0, 0.0, 0.0, 0.0),
            spline_id: 1,
            move_type: 0,
            facing_target: None,
            facing_angle: None,
            spline_flags: flags,
            duration: 1000,
            waypoints: vec![
                Position::new(10.0, 0.0, 0.0, 0.0),
                Position::new(20.0, 4.0, 0.0, 0.0),
            ],
        }
    }

    #[test]
    fn vanilla_linear_path_packs_intermediate_points_back_from_the_destination() {
        let packet = path_move(0x100).to_vanilla();
        let body = packet.contents();
        // count, destination, one packed offset
        let path = &body[body.len() - 20..];
        assert_eq!(&path[..4], &2u32.to_le_bytes());
        assert_eq!(&path[4..8], &20.0f32.to_le_bytes());
        // One packed offset: destination (20, 4, 0) minus point (10, 0, 0).
        let packed = u32::from_le_bytes(path[16..20].try_into().unwrap());
        let x = ((packed & 0x7FF) as i32) << 21 >> 21;
        let y = (((packed >> 11) & 0x7FF) as i32) << 21 >> 21;
        assert_eq!((x as f32 * 0.25, y as f32 * 0.25), (10.0, 4.0));
        assert_eq!(&body[body.len() - 24..body.len() - 20], &1000u32.to_le_bytes());
    }

    #[test]
    fn flying_path_lists_every_point_in_full_on_both_protocols() {
        let message = path_move(0x100 | spline_flags::FLYING);
        let vanilla = message.to_vanilla();
        let body = vanilla.contents();
        let path = &body[body.len() - 28..];
        assert_eq!(&path[..4], &2u32.to_le_bytes());
        assert_eq!(&path[4..8], &10.0f32.to_le_bytes());
        assert_eq!(&path[16..20], &20.0f32.to_le_bytes());
        assert_eq!(&body[body.len() - 32..body.len() - 28], &1000u32.to_le_bytes());

        let modern = message.to_modern().unwrap();
        // Both points travel uncompressed, so the body ends with them.
        let body = modern.contents();
        let tail = &body[body.len() - 24..];
        assert_eq!(&tail[..4], &10.0f32.to_le_bytes());
        assert_eq!(&tail[12..16], &20.0f32.to_le_bytes());
    }

    #[test]
    fn move_teleport_puts_the_counter_between_mover_and_movement() {
        let mover = ObjectGuid::new_without_entry(crate::protocol::HighGuid::Player, 5);
        let mut info = MovementInfo::new();
        info.time = 9;
        info.position = Position::new(1.0, 2.0, 3.0, 0.5);
        let message = SmsgMoveTeleport {
            mover,
            counter: 7,
            info,
        };

        let vanilla = message.to_vanilla();
        assert_eq!(vanilla.opcode().vanilla(), 0x00C7);
        let body = vanilla.contents();
        assert_eq!(&body[..2], &[0x01, 5]); // packed guid
        assert_eq!(&body[2..6], &7u32.to_le_bytes());
        assert_eq!(&body[6..10], &0u32.to_le_bytes()); // flags
        assert_eq!(&body[10..14], &9u32.to_le_bytes()); // time
        assert_eq!(&body[14..18], &1.0f32.to_le_bytes());
        assert_eq!(body.len(), 2 + 4 + 4 + 4 + 16 + 4);

        let modern = message.to_modern().unwrap();
        assert_eq!(modern.opcode().modern(), 0x2E04);
        let body = modern.contents();
        let tail = &body[body.len() - 22..];
        assert_eq!(&tail[..4], &7u32.to_le_bytes());
        assert_eq!(&tail[4..8], &1.0f32.to_le_bytes());
        assert_eq!(&tail[16..20], &0.5f32.to_le_bytes());
        assert_eq!(&tail[20..], &[0, 0]); // no preload, no transport or vehicle
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

/// A same-map teleport, sent to the client that moves the unit. The client
/// lands at the new position and answers with a teleport ack carrying the same
/// counter.
///
/// Vanilla reuses `MSG_MOVE_TELEPORT_ACK` for this (mover, counter, then the
/// full movement block at the destination); 1.14 has a dedicated
/// `SMSG_MOVE_TELEPORT` with just the position.
#[derive(Debug, Clone)]
pub struct SmsgMoveTeleport {
    pub mover: ObjectGuid,
    pub counter: u32,
    /// Movement at the destination, with moving flags already cleared.
    pub info: MovementInfo,
}

impl ToWorldPacket for SmsgMoveTeleport {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_MOVE_TELEPORT_ACK);
        packet.write_packed_guid_raw(self.mover.counter() as u64);
        packet.write_u32(self.counter);
        self.info.write_body(&mut packet);
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.mover.to_guid128(MODERN_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u32(self.counter);
        writer.write_f32(self.info.position.x);
        writer.write_f32(self.info.position.y);
        writer.write_f32(self.info.position.z);
        writer.write_f32(normalize_orientation(self.info.position.o));
        writer.write_u8(0); // PreloadWorld
        writer.write_bit(false); // TransportGUID
        writer.write_bit(false); // Vehicle
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_MOVE_TELEPORT))
    }
}

/// SMSG_SUMMON_REQUEST - someone (a warlock's ritual, a meeting stone) offers
/// to summon the player to them. The client answers with CMSG_SUMMON_RESPONSE.
#[derive(Debug, Clone)]
pub struct SmsgSummonRequest {
    pub summoner: ObjectGuid,
    pub zone_id: u32,
    /// How long the offer stands before the client declines it by itself.
    pub delay_ms: u32,
}

impl ToWorldPacket for SmsgSummonRequest {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SUMMON_REQUEST);
        packet.write_guid_raw(self.summoner.raw());
        packet.write_u32(self.zone_id);
        packet.write_u32(self.delay_ms);
        packet
    }

    /// 1.14 drops the delay (the client keeps its own timer) and adds the
    /// summoner's realm and a reason (0: a spell).
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.summoner.to_guid128(MODERN_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u32(0x0101_0000 | u32::from(MODERN_REALM_ID)); // virtual realm address
        writer.write_u32(self.zone_id);
        writer.write_u8(0); // Reason: spell
        writer.write_bit(false); // SkipStartingArea
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_SUMMON_REQUEST))
    }
}

/// A player's movement, relayed to nearby observers.
///
/// The two protocols disagree about what this even is. Vanilla echoes the client's own
/// `MSG_MOVE_*` opcode back to observers, so the opcode carries the meaning (started running,
/// stopped, jumped). 1.14 has one `SMSG_MOVE_UPDATE` for all of them and puts the meaning in the
/// movement flags instead.
///
/// That is why this is a message rather than a re-broadcast of the received packet: the opcode has
/// to change per recipient, which a pre-encoded `WorldPacket` cannot express.
#[derive(Debug, Clone)]
pub struct MsgMovementBroadcast {
    /// The opcode the client sent, echoed to vanilla observers.
    pub opcode: Opcode,
    pub mover: ObjectGuid,
    pub info: MovementInfo,
    /// Knockback observers receive the acknowledged launch vector after the movement block.
    pub knockback: Option<KnockbackVector>,
}

/// The launch vector trailing `MSG_MOVE_KNOCK_BACK`.
#[derive(Debug, Clone, Copy, Default)]
pub struct KnockbackVector {
    pub cos_angle: f32,
    pub sin_angle: f32,
    pub xy_speed: f32,
    pub velocity: f32,
}

impl ToWorldPacket for MsgMovementBroadcast {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(self.opcode);
        self.info.write_to_packet(&mut packet);

        if let Some(knockback) = self.knockback {
            packet.write_f32(knockback.cos_angle);
            packet.write_f32(knockback.sin_angle);
            packet.write_f32(knockback.xy_speed);
            packet.write_f32(knockback.velocity);
        }

        packet
    }

    /// `SMSG_MOVE_UPDATE`: just a movement block, with the mover named inside it.
    ///
    /// The knockback vector has no place in this body -- 1.14 sends knockbacks as their own
    /// message -- so it is dropped here. Observers see the destination but not the launch arc.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        self.info
            .write_modern(&mut writer, self.mover, MODERN_REALM_ID);
        Some(writer.finish(Opcode::SMSG_MOVE_UPDATE))
    }
}
