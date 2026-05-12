//! Movement-related message structures

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Opcode, Position, WorldPacket};
use crate::shared::protocol::guid::ObjectGuid as WorldObjectGuid;
use crate::shared::protocol::packet::WorldPacketGuidExt;

/// Generic movement broadcast message (MSG_MOVE_*)
/// Used to broadcast movement to nearby players (Phase 5 - not yet implemented)
#[allow(dead_code)]
pub struct MsgMoveBroadcast {
    pub opcode: Opcode,
}

impl ToWorldPacket for MsgMoveBroadcast {
    fn to_world_packet(&self) -> WorldPacket {
        // TODO: Implement for Phase 5 (broadcasting to nearby players)
        WorldPacket::new(self.opcode)
    }
}

/// SMSG_PONG - response to CMSG_PING
pub struct SmsgPong {
    pub sequence: u32,
}

impl ToWorldPacket for SmsgPong {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_PONG);
        packet.write_u32(self.sequence);
        packet
    }
}

/// SMSG_FORCE_MOVE_ROOT - Lock player in place (prevent movement)
#[derive(Debug, Clone)]
pub struct SmsgForceMoveRoot {
    pub guid: WorldObjectGuid,
}

impl ToWorldPacket for SmsgForceMoveRoot {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_FORCE_MOVE_ROOT);
        packet.write_packed_guid(self.guid);
        packet.write_u32(0); // counter
        packet
    }
}

/// SMSG_FORCE_MOVE_UNROOT - Unlock player (allow movement)
#[derive(Debug, Clone)]
pub struct SmsgForceMoveUnroot {
    pub guid: WorldObjectGuid,
}

impl ToWorldPacket for SmsgForceMoveUnroot {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_FORCE_MOVE_UNROOT);
        packet.write_packed_guid(self.guid);
        packet.write_u32(0); // counter
        packet
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
    pub facing_angle: Option<f32>,         // For move_type 4
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
            move_type: 1, // Stop
            facing_target: None,
            facing_angle: None,
            spline_flags: super::movement::spline_flags::DONE,
            duration: 0,
            waypoints: vec![],
        }
    }
}

impl ToWorldPacket for SmsgMonsterMove {
    fn to_world_packet(&self) -> WorldPacket {
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
                    packet.write_pack_xyz(
                        middle_x - wp.x,
                        middle_y - wp.y,
                        middle_z - wp.z,
                    );
                }
            }
        }

        packet
    }
}
