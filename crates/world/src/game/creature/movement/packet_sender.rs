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

    /// Unmodified base speed for a move type, in yards/sec.
    ///
    /// Speed changes travel the wire as an absolute speed, so the rate the server tracks is
    /// multiplied by this before being sent. Flight speeds have no entry in 1.12.
    fn base_move_speed(move_type: MoveType) -> Option<f32> {
        match move_type {
            MoveType::Walk => Some(2.5),
            MoveType::Run => Some(7.0),
            MoveType::RunBack => Some(4.5),
            MoveType::Swim => Some(4.722222),
            MoveType::SwimBack => Some(2.5),
            MoveType::TurnRate => Some(3.141594),
            MoveType::Flight | MoveType::FlightBack => None,
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

    fn creature_exists(world: &World, creature_guid: ObjectGuid) -> bool {
        world
            .managers
            .creature_mgr
            .with_creature(creature_guid, |_| ())
            .is_some()
    }

    /// Broadcast a speed change for a server-controlled unit.
    ///
    /// `new_rate` is the speed multiplier the server tracks; the wire carries the resulting
    /// absolute speed.
    pub fn send_speed_change_to_all(
        world: &World,
        creature_guid: ObjectGuid,
        move_type: MoveType,
        new_rate: f32,
    ) -> bool {
        let (Some(opcode), Some(base_speed)) = (
            Self::opcode_for_move_type(move_type),
            Self::base_move_speed(move_type),
        ) else {
            return false;
        };

        if !Self::creature_exists(world, creature_guid) {
            return false;
        }

        let mut packet = WorldPacket::new(opcode);
        packet.write_packed_guid_raw(creature_guid.raw());
        packet.write_f32(new_rate * base_speed);

        broadcast_around_creature(world, creature_guid, &packet);
        true
    }

    pub fn send_toggle_run_walk_to_all(world: &World, creature_guid: ObjectGuid, run: bool) {
        let opcode = if run {
            Opcode::SMSG_SPLINE_MOVE_SET_RUN_MODE
        } else {
            Opcode::SMSG_SPLINE_MOVE_SET_WALK_MODE
        };

        if !Self::creature_exists(world, creature_guid) {
            return;
        }

        let mut packet = WorldPacket::new(opcode);
        packet.write_packed_guid_raw(creature_guid.raw());
        broadcast_around_creature(world, creature_guid, &packet);
    }

    /// Opcode carrying a movement flag change for a server-controlled unit.
    fn flag_change_broadcast_opcode(flag: MovementFlagChange, apply: bool) -> Opcode {
        match flag {
            MovementFlagChange::Root => {
                if apply {
                    Opcode::SMSG_SPLINE_MOVE_ROOT
                } else {
                    Opcode::SMSG_SPLINE_MOVE_UNROOT
                }
            }
            MovementFlagChange::WaterWalking => {
                if apply {
                    Opcode::SMSG_SPLINE_MOVE_WATER_WALK
                } else {
                    Opcode::SMSG_SPLINE_MOVE_LAND_WALK
                }
            }
            MovementFlagChange::Hover => {
                if apply {
                    Opcode::SMSG_SPLINE_MOVE_SET_HOVER
                } else {
                    Opcode::SMSG_SPLINE_MOVE_UNSET_HOVER
                }
            }
            MovementFlagChange::SafeFall => {
                if apply {
                    Opcode::SMSG_SPLINE_MOVE_FEATHER_FALL
                } else {
                    Opcode::SMSG_SPLINE_MOVE_NORMAL_FALL
                }
            }
        }
    }

    pub fn send_movement_flag_change_to_all(
        world: &World,
        creature_guid: ObjectGuid,
        flag: MovementFlagChange,
        apply: bool,
    ) -> bool {
        if !Self::creature_exists(world, creature_guid) {
            return false;
        }

        let mut packet = WorldPacket::new(Self::flag_change_broadcast_opcode(flag, apply));
        packet.write_packed_guid_raw(creature_guid.raw());
        broadcast_around_creature(world, creature_guid, &packet);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_type_mapping_covers_supported_move_types() {
        let cases = [
            (MoveType::Walk, MovementChangeType::SpeedChangeWalk),
            (MoveType::Run, MovementChangeType::SpeedChangeRun),
            (MoveType::RunBack, MovementChangeType::SpeedChangeRunBack),
            (MoveType::Swim, MovementChangeType::SpeedChangeSwim),
            (MoveType::SwimBack, MovementChangeType::SpeedChangeSwimBack),
            (MoveType::TurnRate, MovementChangeType::RateChangeTurn),
        ];

        for (move_type, change_type) in cases {
            assert_eq!(
                MovementPacketSender::get_change_type_by_move_type(move_type),
                Some(change_type)
            );
            assert_eq!(
                MovementPacketSender::get_move_type_by_change_type(change_type),
                move_type
            );
        }
    }

    #[test]
    fn change_type_mapping_rejects_flight_move_types() {
        assert_eq!(
            MovementPacketSender::get_change_type_by_move_type(MoveType::Flight),
            None
        );
        assert_eq!(
            MovementPacketSender::get_change_type_by_move_type(MoveType::FlightBack),
            None
        );
    }

    #[test]
    fn speed_broadcast_opcode_mapping_matches_move_type() {
        let cases = [
            (MoveType::Walk, Opcode::SMSG_SPLINE_SET_WALK_SPEED),
            (MoveType::Run, Opcode::SMSG_SPLINE_SET_RUN_SPEED),
            (MoveType::RunBack, Opcode::SMSG_SPLINE_SET_RUN_BACK_SPEED),
            (MoveType::Swim, Opcode::SMSG_SPLINE_SET_SWIM_SPEED),
            (MoveType::SwimBack, Opcode::SMSG_SPLINE_SET_SWIM_BACK_SPEED),
            (MoveType::TurnRate, Opcode::SMSG_SPLINE_SET_TURN_RATE),
        ];

        for (move_type, opcode) in cases {
            assert_eq!(
                MovementPacketSender::opcode_for_move_type(move_type),
                Some(opcode)
            );
        }

        assert_eq!(
            MovementPacketSender::opcode_for_move_type(MoveType::Flight),
            None
        );
        assert_eq!(
            MovementPacketSender::opcode_for_move_type(MoveType::FlightBack),
            None
        );
    }

    #[test]
    fn base_move_speeds_match_the_client_defaults() {
        assert_eq!(
            MovementPacketSender::base_move_speed(MoveType::Walk),
            Some(2.5)
        );
        assert_eq!(
            MovementPacketSender::base_move_speed(MoveType::Run),
            Some(7.0)
        );
        assert_eq!(
            MovementPacketSender::base_move_speed(MoveType::RunBack),
            Some(4.5)
        );
        assert_eq!(
            MovementPacketSender::base_move_speed(MoveType::Swim),
            Some(4.722222)
        );
        assert_eq!(
            MovementPacketSender::base_move_speed(MoveType::SwimBack),
            Some(2.5)
        );
        assert_eq!(
            MovementPacketSender::base_move_speed(MoveType::TurnRate),
            Some(3.141594)
        );
        assert_eq!(
            MovementPacketSender::base_move_speed(MoveType::Flight),
            None
        );
        assert_eq!(
            MovementPacketSender::base_move_speed(MoveType::FlightBack),
            None
        );
    }

    #[test]
    fn flag_change_broadcast_uses_paired_spline_opcodes() {
        let cases = [
            (
                MovementFlagChange::Root,
                Opcode::SMSG_SPLINE_MOVE_ROOT,
                Opcode::SMSG_SPLINE_MOVE_UNROOT,
            ),
            (
                MovementFlagChange::WaterWalking,
                Opcode::SMSG_SPLINE_MOVE_WATER_WALK,
                Opcode::SMSG_SPLINE_MOVE_LAND_WALK,
            ),
            (
                MovementFlagChange::Hover,
                Opcode::SMSG_SPLINE_MOVE_SET_HOVER,
                Opcode::SMSG_SPLINE_MOVE_UNSET_HOVER,
            ),
            (
                MovementFlagChange::SafeFall,
                Opcode::SMSG_SPLINE_MOVE_FEATHER_FALL,
                Opcode::SMSG_SPLINE_MOVE_NORMAL_FALL,
            ),
        ];

        for (flag, on, off) in cases {
            assert_eq!(
                MovementPacketSender::flag_change_broadcast_opcode(flag, true),
                on
            );
            assert_eq!(
                MovementPacketSender::flag_change_broadcast_opcode(flag, false),
                off
            );
        }
    }
}
