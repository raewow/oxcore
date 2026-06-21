//! Creature movement packet sender helpers.

use super::types::MoveType;
use crate::game::broadcast_mgr::broadcast_around_creature;
use crate::World;
use oxcore_shared::protocol::{ObjectGuid, Opcode, WorldPacket};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementChangeType {
    SpeedChangeWalk,
    SpeedChangeRun,
    SpeedChangeRunBack,
    SpeedChangeSwim,
    SpeedChangeSwimBack,
    RateChangeTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementFlagChange {
    Root,
    WaterWalking,
    Hover,
    SafeFall,
}

pub struct MovementPacketSender;

impl MovementPacketSender {
    pub fn get_change_type_by_move_type(move_type: MoveType) -> Option<MovementChangeType> {
        match move_type {
            MoveType::Walk => Some(MovementChangeType::SpeedChangeWalk),
            MoveType::Run => Some(MovementChangeType::SpeedChangeRun),
            MoveType::RunBack => Some(MovementChangeType::SpeedChangeRunBack),
            MoveType::Swim => Some(MovementChangeType::SpeedChangeSwim),
            MoveType::SwimBack => Some(MovementChangeType::SpeedChangeSwimBack),
            MoveType::TurnRate => Some(MovementChangeType::RateChangeTurn),
            MoveType::Flight | MoveType::FlightBack => None,
        }
    }

    pub fn get_move_type_by_change_type(change_type: MovementChangeType) -> MoveType {
        match change_type {
            MovementChangeType::SpeedChangeWalk => MoveType::Walk,
            MovementChangeType::SpeedChangeRun => MoveType::Run,
            MovementChangeType::SpeedChangeRunBack => MoveType::RunBack,
            MovementChangeType::SpeedChangeSwim => MoveType::Swim,
            MovementChangeType::SpeedChangeSwimBack => MoveType::SwimBack,
            MovementChangeType::RateChangeTurn => MoveType::TurnRate,
        }
    }

    fn opcode_for_move_type(move_type: MoveType) -> Option<Opcode> {
        match move_type {
            MoveType::Walk => Some(Opcode::SMSG_SPLINE_SET_WALK_SPEED),
            MoveType::Run => Some(Opcode::SMSG_SPLINE_SET_RUN_SPEED),
            MoveType::RunBack => Some(Opcode::SMSG_SPLINE_SET_RUN_BACK_SPEED),
            MoveType::Swim => Some(Opcode::SMSG_SPLINE_SET_SWIM_SPEED),
            MoveType::SwimBack => Some(Opcode::SMSG_SPLINE_SET_SWIM_BACK_SPEED),
            MoveType::TurnRate => Some(Opcode::SMSG_SPLINE_SET_TURN_RATE),
            MoveType::Flight | MoveType::FlightBack => None,
        }
    }

    pub fn send_speed_change_to_observers(
        world: &World,
        creature_guid: ObjectGuid,
        move_type: MoveType,
        new_speed: f32,
    ) -> bool {
        Self::send_speed_change_to_all(world, creature_guid, move_type, new_speed)
    }

    pub fn send_speed_change_to_all(
        world: &World,
        creature_guid: ObjectGuid,
        move_type: MoveType,
        new_speed: f32,
    ) -> bool {
        let Some(opcode) = Self::opcode_for_move_type(move_type) else {
            return false;
        };

        let creature_info = world
            .managers
            .creature_mgr
            .with_creature(creature_guid, |c| (c.position, c.map_id, c.instance_id));

        let Some((position, _, _)) = creature_info else {
            return false;
        };

        let mut packet = WorldPacket::new(opcode);
        packet.write_packed_guid_raw(creature_guid.raw());
        packet.write_f32(new_speed);

        broadcast_around_creature(world, creature_guid, &packet);
        true
    }

    pub fn send_toggle_run_walk_to_all(world: &World, creature_guid: ObjectGuid, run: bool) {
        let opcode = if run {
            Opcode::SMSG_SPLINE_MOVE_SET_RUN_MODE
        } else {
            Opcode::SMSG_SPLINE_MOVE_SET_WALK_MODE
        };

        if let Some((position, _, _)) = world
            .managers
            .creature_mgr
            .with_creature(creature_guid, |c| (c.position, c.map_id, c.instance_id))
        {
            let mut packet = WorldPacket::new(opcode);
            packet.write_packed_guid_raw(creature_guid.raw());
            broadcast_around_creature(world, creature_guid, &packet);
            let _ = position;
        }
    }

    pub fn send_movement_flag_change_to_observers(
        world: &World,
        creature_guid: ObjectGuid,
        flag: MovementFlagChange,
        apply: bool,
    ) -> bool {
        Self::send_movement_flag_change_to_all(world, creature_guid, flag, apply)
    }

    pub fn send_movement_flag_change_to_all(
        world: &World,
        creature_guid: ObjectGuid,
        flag: MovementFlagChange,
        apply: bool,
    ) -> bool {
        let opcode = match flag {
            MovementFlagChange::Root => {
                if apply { Opcode::MSG_MOVE_ROOT } else { Opcode::MSG_MOVE_UNROOT }
            }
            MovementFlagChange::WaterWalking => Opcode::MSG_MOVE_WATER_WALK,
            MovementFlagChange::Hover => Opcode::MSG_MOVE_HOVER,
            MovementFlagChange::SafeFall => Opcode::MSG_MOVE_FEATHER_FALL,
        };

        let Some((position, _, _)) = world
            .managers
            .creature_mgr
            .with_creature(creature_guid, |c| (c.position, c.map_id, c.instance_id))
        else {
            return false;
        };

        let mut packet = WorldPacket::new(opcode);
        packet.write_packed_guid_raw(creature_guid.raw());
        broadcast_around_creature(world, creature_guid, &packet);
        let _ = position;
        true
    }
}
