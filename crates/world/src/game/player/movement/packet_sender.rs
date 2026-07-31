//! Server-forced movement changes sent to the unit's controlling player.
//!
//! Unlike the broadcast senders in the creature module, every change here has to be
//! acknowledged by the client: each one is stamped with a movement counter, queued on the
//! player, and only applied server-side once the matching ack arrives.

use super::state::{PendingChangeKind, PendingMovementChange};
use crate::game::creature::movement::packet_sender::{
    MovementFlagChange, MovementPacketSender as BroadcastSender,
};
use crate::game::creature::movement::MoveType;
use crate::World;
use oxcore_shared::protocol::{ObjectGuid, Opcode, WorldPacket};

/// Base speeds the client applies a rate to, in yards/sec.
const BASE_WALK_SPEED: f32 = 2.5;
const BASE_RUN_SPEED: f32 = 7.0;
const BASE_RUN_BACK_SPEED: f32 = 4.5;
const BASE_SWIM_SPEED: f32 = 4.722222;
const BASE_SWIM_BACK_SPEED: f32 = 2.5;
const BASE_TURN_RATE: f32 = 3.141594;

pub struct MovementControllerSender;

impl MovementControllerSender {
    pub fn base_move_speed(move_type: MoveType) -> Option<f32> {
        match move_type {
            MoveType::Walk => Some(BASE_WALK_SPEED),
            MoveType::Run => Some(BASE_RUN_SPEED),
            MoveType::RunBack => Some(BASE_RUN_BACK_SPEED),
            MoveType::Swim => Some(BASE_SWIM_SPEED),
            MoveType::SwimBack => Some(BASE_SWIM_BACK_SPEED),
            MoveType::TurnRate => Some(BASE_TURN_RATE),
            MoveType::Flight | MoveType::FlightBack => None,
        }
    }

    /// Opcode forcing a speed change on the controlling client.
    pub fn force_speed_opcode(move_type: MoveType) -> Option<Opcode> {
        match move_type {
            MoveType::Walk => Some(Opcode::SMSG_FORCE_WALK_SPEED_CHANGE),
            MoveType::Run => Some(Opcode::SMSG_FORCE_RUN_SPEED_CHANGE),
            MoveType::RunBack => Some(Opcode::SMSG_FORCE_RUN_BACK_SPEED_CHANGE),
            MoveType::Swim => Some(Opcode::SMSG_FORCE_SWIM_SPEED_CHANGE),
            MoveType::SwimBack => Some(Opcode::SMSG_FORCE_SWIM_BACK_SPEED_CHANGE),
            MoveType::TurnRate => Some(Opcode::SMSG_FORCE_TURN_RATE_CHANGE),
            MoveType::Flight | MoveType::FlightBack => None,
        }
    }

    /// Opcode forcing a movement flag toggle on the controlling client.
    pub fn force_flag_opcode(flag: MovementFlagChange, apply: bool) -> Opcode {
        match flag {
            MovementFlagChange::Root => {
                if apply {
                    Opcode::SMSG_FORCE_MOVE_ROOT
                } else {
                    Opcode::SMSG_FORCE_MOVE_UNROOT
                }
            }
            MovementFlagChange::WaterWalking => {
                if apply {
                    Opcode::SMSG_MOVE_WATER_WALK
                } else {
                    Opcode::SMSG_MOVE_LAND_WALK
                }
            }
            MovementFlagChange::Hover => {
                if apply {
                    Opcode::SMSG_MOVE_SET_HOVER
                } else {
                    Opcode::SMSG_MOVE_UNSET_HOVER
                }
            }
            MovementFlagChange::SafeFall => {
                if apply {
                    Opcode::SMSG_MOVE_FEATHER_FALL
                } else {
                    Opcode::SMSG_MOVE_NORMAL_FALL
                }
            }
        }
    }

    /// Queue and send a speed change for the player controlling this unit.
    ///
    /// `new_rate` is the multiplier; the wire carries the resulting absolute speed. The
    /// server-side speed is applied only when the client acknowledges the change.
    pub fn add_speed_change_to_controller(
        world: &World,
        player_guid: ObjectGuid,
        move_type: MoveType,
        new_rate: f32,
    ) -> bool {
        let (Some(base_speed), Some(change_type)) = (
            Self::base_move_speed(move_type),
            BroadcastSender::get_change_type_by_move_type(move_type),
        ) else {
            return false;
        };

        let new_speed_flat = new_rate * base_speed;

        let pending = world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                let counter = player.movement.next_movement_counter();
                let pending = PendingMovementChange {
                    counter,
                    kind: PendingChangeKind::Speed(change_type),
                    new_value: new_speed_flat,
                    apply: true,
                };
                player.movement.push_pending_change(pending);
                pending
            });

        let Some(pending) = pending else {
            return false;
        };

        Self::send_speed_change_to_controller(world, player_guid, move_type, &pending)
    }

    /// Send the packet for an already-queued speed change.
    pub fn send_speed_change_to_controller(
        world: &World,
        player_guid: ObjectGuid,
        move_type: MoveType,
        pending: &PendingMovementChange,
    ) -> bool {
        let Some(opcode) = Self::force_speed_opcode(move_type) else {
            return false;
        };

        let mut packet = WorldPacket::new(opcode);
        packet.write_packed_guid_raw(player_guid.raw());
        packet.write_u32(pending.counter);
        packet.write_f32(pending.new_value);

        world
            .managers
            .broadcast_mgr
            .send_to_player(player_guid, packet);
        true
    }

    /// Queue and send a movement flag toggle for the player controlling this unit.
    pub fn add_movement_flag_change_to_controller(
        world: &World,
        player_guid: ObjectGuid,
        flag: MovementFlagChange,
        apply: bool,
    ) -> bool {
        let pending = world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                let counter = player.movement.next_movement_counter();
                let pending = PendingMovementChange {
                    counter,
                    kind: PendingChangeKind::Flag(flag),
                    new_value: 0.0,
                    apply,
                };
                player.movement.push_pending_change(pending);
                pending
            });

        let Some(pending) = pending else {
            return false;
        };

        Self::send_movement_flag_change_to_controller(world, player_guid, flag, &pending)
    }

    /// Send the packet for an already-queued flag change.
    pub fn send_movement_flag_change_to_controller(
        world: &World,
        player_guid: ObjectGuid,
        flag: MovementFlagChange,
        pending: &PendingMovementChange,
    ) -> bool {
        let mut packet = WorldPacket::new(Self::force_flag_opcode(flag, pending.apply));
        packet.write_packed_guid_raw(player_guid.raw());
        packet.write_u32(pending.counter);

        world
            .managers
            .broadcast_mgr
            .send_to_player(player_guid, packet);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::creature::movement::packet_sender::MovementChangeType;
    use crate::game::player::movement::state::MovementState;

    #[test]
    fn base_speeds_and_force_opcodes_cover_every_tracked_move_type() {
        let cases = [
            (MoveType::Walk, 2.5, Opcode::SMSG_FORCE_WALK_SPEED_CHANGE),
            (MoveType::Run, 7.0, Opcode::SMSG_FORCE_RUN_SPEED_CHANGE),
            (
                MoveType::RunBack,
                4.5,
                Opcode::SMSG_FORCE_RUN_BACK_SPEED_CHANGE,
            ),
            (
                MoveType::Swim,
                4.722222,
                Opcode::SMSG_FORCE_SWIM_SPEED_CHANGE,
            ),
            (
                MoveType::SwimBack,
                2.5,
                Opcode::SMSG_FORCE_SWIM_BACK_SPEED_CHANGE,
            ),
            (
                MoveType::TurnRate,
                3.141594,
                Opcode::SMSG_FORCE_TURN_RATE_CHANGE,
            ),
        ];

        for (move_type, base, opcode) in cases {
            assert_eq!(
                MovementControllerSender::base_move_speed(move_type),
                Some(base)
            );
            assert_eq!(
                MovementControllerSender::force_speed_opcode(move_type),
                Some(opcode)
            );
        }

        assert_eq!(
            MovementControllerSender::force_speed_opcode(MoveType::Flight),
            None
        );
    }

    #[test]
    fn force_flag_opcodes_are_paired_on_apply() {
        let cases = [
            (
                MovementFlagChange::Root,
                Opcode::SMSG_FORCE_MOVE_ROOT,
                Opcode::SMSG_FORCE_MOVE_UNROOT,
            ),
            (
                MovementFlagChange::WaterWalking,
                Opcode::SMSG_MOVE_WATER_WALK,
                Opcode::SMSG_MOVE_LAND_WALK,
            ),
            (
                MovementFlagChange::Hover,
                Opcode::SMSG_MOVE_SET_HOVER,
                Opcode::SMSG_MOVE_UNSET_HOVER,
            ),
            (
                MovementFlagChange::SafeFall,
                Opcode::SMSG_MOVE_FEATHER_FALL,
                Opcode::SMSG_MOVE_NORMAL_FALL,
            ),
        ];

        for (flag, on, off) in cases {
            assert_eq!(MovementControllerSender::force_flag_opcode(flag, true), on);
            assert_eq!(
                MovementControllerSender::force_flag_opcode(flag, false),
                off
            );
        }
    }

    #[test]
    fn movement_counter_never_hands_out_the_zero_sentinel() {
        let mut state = MovementState::default();
        state.movement_counter = u32::MAX;

        assert_eq!(state.next_movement_counter(), 1);
    }

    #[test]
    fn speed_ack_matches_only_the_queued_counter_type_and_value() {
        let mut state = MovementState::default();
        let counter = state.next_movement_counter();
        state.push_pending_change(PendingMovementChange {
            counter,
            kind: PendingChangeKind::Speed(MovementChangeType::SpeedChangeRun),
            new_value: 14.0,
            apply: true,
        });

        assert!(state.has_pending_movement_change());
        // Wrong counter, wrong type and wrong speed are all rejected.
        assert!(!state.find_pending_speed_change(
            14.0,
            counter + 1,
            MovementChangeType::SpeedChangeRun
        ));
        assert!(!state.find_pending_speed_change(
            14.0,
            counter,
            MovementChangeType::SpeedChangeWalk
        ));
        assert!(!state.find_pending_speed_change(
            20.0,
            counter,
            MovementChangeType::SpeedChangeRun
        ));

        // Speeds within the 0.01 tolerance match.
        assert!(state.find_pending_speed_change(
            14.005,
            counter,
            MovementChangeType::SpeedChangeRun
        ));
        assert!(!state.has_pending_movement_change());
    }

    #[test]
    fn flag_ack_reports_whether_the_change_applied_the_flag() {
        let mut state = MovementState::default();
        let counter = state.next_movement_counter();
        state.push_pending_change(PendingMovementChange {
            counter,
            kind: PendingChangeKind::Flag(MovementFlagChange::WaterWalking),
            new_value: 0.0,
            apply: false,
        });

        assert_eq!(
            state.find_pending_flag_change(counter, MovementFlagChange::Hover),
            None
        );
        assert_eq!(
            state.find_pending_flag_change(counter, MovementFlagChange::WaterWalking),
            Some(false)
        );
        assert!(!state.has_pending_movement_change());
    }
}
