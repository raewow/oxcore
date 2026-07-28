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

    fn observer_speed_opcode(move_type: MoveType) -> Option<Opcode> {
        match move_type {
            MoveType::Walk => Some(Opcode::MSG_MOVE_SET_WALK_SPEED),
            MoveType::Run => Some(Opcode::MSG_MOVE_SET_RUN_SPEED),
            MoveType::RunBack => Some(Opcode::MSG_MOVE_SET_RUN_BACK_SPEED),
            MoveType::Swim => Some(Opcode::MSG_MOVE_SET_SWIM_SPEED),
            MoveType::SwimBack => Some(Opcode::MSG_MOVE_SET_SWIM_BACK_SPEED),
            MoveType::TurnRate => Some(Opcode::MSG_MOVE_SET_TURN_RATE),
            MoveType::Flight | MoveType::FlightBack => None,
        }
    }

    fn absolute_speed(move_type: MoveType, rate: f32) -> Option<f32> {
        Self::base_move_speed(move_type).map(|base_speed| rate * base_speed)
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
        let (Some(opcode), Some(speed)) = (
            Self::opcode_for_move_type(move_type),
            Self::absolute_speed(move_type, new_rate),
        ) else {
            return false;
        };

        if !Self::creature_exists(world, creature_guid) {
            return false;
        }

        let mut packet = WorldPacket::new(opcode);
        packet.write_packed_guid_raw(creature_guid.raw());
        packet.write_f32(speed);

        crate::game::broadcast_mgr::broadcast_around_creature_packet(world, creature_guid, &packet);
        true
    }

    /// Broadcast a client-controlled unit's acknowledged speed change to observers.
    ///
    /// A running spline owns the unit's position, so observers receive the compact spline
    /// packet. Otherwise they receive the normal movement packet with full movement info.
    pub fn send_speed_change_to_observers(
        world: &World,
        player_guid: ObjectGuid,
        move_type: MoveType,
        new_speed: f32,
        movement_info: &crate::core::common::MovementInfo,
    ) -> bool {
        let Some(spline_active) = world
            .managers
            .player_mgr
            .with_player(player_guid, |player| {
                player.movement.pending_spline.is_some()
            })
        else {
            return false;
        };

        let Some(opcode) = (if spline_active {
            Self::opcode_for_move_type(move_type)
        } else {
            Self::observer_speed_opcode(move_type)
        }) else {
            return false;
        };

        let mut packet = WorldPacket::new(opcode);
        if spline_active {
            packet.write_packed_guid_raw(player_guid.raw());
        } else {
            movement_info.write_to_packet(&mut packet);
        }
        packet.write_f32(new_speed);
        world
            .managers
            .broadcast_mgr
            .broadcast_nearby_exclude_self(player_guid, &packet);
        true
    }

    fn toggle_run_walk_opcode(run: bool) -> Opcode {
        if run {
            Opcode::SMSG_SPLINE_MOVE_SET_RUN_MODE
        } else {
            Opcode::SMSG_SPLINE_MOVE_SET_WALK_MODE
        }
    }

    pub fn send_toggle_run_walk_to_all(world: &World, creature_guid: ObjectGuid, run: bool) {
        let opcode = Self::toggle_run_walk_opcode(run);

        if !Self::creature_exists(world, creature_guid) {
            return;
        }

        let mut packet = WorldPacket::new(opcode);
        packet.write_packed_guid_raw(creature_guid.raw());
        crate::game::broadcast_mgr::broadcast_around_creature_packet(world, creature_guid, &packet);
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

    /// Opcode sent to observers for a client-controlled unit's acknowledged flag change.
    fn flag_change_observer_opcode(flag: MovementFlagChange, apply: bool) -> Opcode {
        match flag {
            MovementFlagChange::Root => {
                if apply {
                    Opcode::MSG_MOVE_ROOT
                } else {
                    Opcode::MSG_MOVE_UNROOT
                }
            }
            MovementFlagChange::WaterWalking => Opcode::MSG_MOVE_WATER_WALK,
            MovementFlagChange::Hover => Opcode::MSG_MOVE_HOVER,
            MovementFlagChange::SafeFall => Opcode::MSG_MOVE_FEATHER_FALL,
        }
    }

    /// Broadcast an acknowledged movement-flag change to observers of a player-controlled unit.
    pub fn send_movement_flag_change_to_observers(
        world: &World,
        player_guid: ObjectGuid,
        flag: MovementFlagChange,
        apply: bool,
        movement_info: &crate::core::common::MovementInfo,
    ) -> bool {
        if world
            .managers
            .player_mgr
            .with_player(player_guid, |_| ())
            .is_none()
        {
            return false;
        }

        let mut packet = WorldPacket::new(Self::flag_change_observer_opcode(flag, apply));
        movement_info.write_to_packet(&mut packet);
        world
            .managers
            .broadcast_mgr
            .broadcast_nearby_exclude_self(player_guid, &packet);
        true
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
        crate::game::broadcast_mgr::broadcast_around_creature_packet(world, creature_guid, &packet);
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
    fn observer_speed_opcodes_use_non_spline_messages() {
        assert_eq!(
            MovementPacketSender::observer_speed_opcode(MoveType::Run),
            Some(Opcode::MSG_MOVE_SET_RUN_SPEED)
        );
        assert_eq!(
            MovementPacketSender::observer_speed_opcode(MoveType::TurnRate),
            Some(Opcode::MSG_MOVE_SET_TURN_RATE)
        );
        assert_eq!(
            MovementPacketSender::observer_speed_opcode(MoveType::Flight),
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
    fn speed_broadcast_uses_an_absolute_speed() {
        assert_eq!(
            MovementPacketSender::absolute_speed(MoveType::Run, 1.5),
            Some(10.5)
        );
        assert_eq!(
            MovementPacketSender::absolute_speed(MoveType::Flight, 1.5),
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

    #[test]
    fn observer_flag_change_opcodes_match_client_controlled_movement() {
        assert_eq!(
            MovementPacketSender::flag_change_observer_opcode(MovementFlagChange::Root, true),
            Opcode::MSG_MOVE_ROOT
        );
        assert_eq!(
            MovementPacketSender::flag_change_observer_opcode(MovementFlagChange::Root, false),
            Opcode::MSG_MOVE_UNROOT
        );
        assert_eq!(
            MovementPacketSender::flag_change_observer_opcode(
                MovementFlagChange::WaterWalking,
                false
            ),
            Opcode::MSG_MOVE_WATER_WALK
        );
        assert_eq!(
            MovementPacketSender::flag_change_observer_opcode(MovementFlagChange::Hover, true),
            Opcode::MSG_MOVE_HOVER
        );
        assert_eq!(
            MovementPacketSender::flag_change_observer_opcode(MovementFlagChange::SafeFall, false),
            Opcode::MSG_MOVE_FEATHER_FALL
        );
    }

    #[test]
    fn toggling_run_walk_uses_the_matching_spline_opcode() {
        assert_eq!(
            MovementPacketSender::toggle_run_walk_opcode(true),
            Opcode::SMSG_SPLINE_MOVE_SET_RUN_MODE
        );
        assert_eq!(
            MovementPacketSender::toggle_run_walk_opcode(false),
            Opcode::SMSG_SPLINE_MOVE_SET_WALK_MODE
        );
    }
}
